open Core
module C = Gpuio.Chart_inspection_content
module S = Gpuio.Chart_selection
module W = Gpuio_protocol.Chart_inspection_content_wire
module R = Gpuio.Chart_resource

let ok = Or_error.ok_exn
let datum n = Gpuio.Chart_data.Datum_id.of_int64 n |> ok
let series n = Gpuio.Chart_data.Series_id.of_int64 n |> ok

let resource owner slot =
  R.Expert.handle ~owner (Gpuio_protocol.Resource_id.create ~slot ~generation:2L |> ok)
;;

let span start_index length first last =
  S.Span.create ~start_index ~length ~first:(datum first) ~last:(datum last) |> ok
;;

let cartesian ?(start = 3) ?(length = 2) ?(first = 9L) ?(last = 4L) aggregation =
  S.cartesian ~series:(series 7L) ~span:(span start length first last) ~aggregation |> ok
;;

let target selection data =
  C.Target.of_selection selection ~data ~data_revision:11L ~data_generation:2L |> ok
;;

let hex wire =
  Bin_prot.Utils.bin_dump W.bin_writer_t wire
  |> Bigstring.to_string
  |> String.to_list
  |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
  |> String.concat
;;

let%expect_test "independent metadata bytes cover every target and both containers" =
  let data = resource (R.Expert.Owner.create ()) 4L in
  let targets =
    [ C.Target.cartesian ~series:(series 7L) ~datum:(datum 9L)
    ; C.Target.slice (datum 9L)
    ; C.Target.radar ~series:(series 7L) ~axis:(datum 9L)
    ; C.Target.candlestick (datum 9L)
    ; C.Target.node (Gpuio.Chart_data.Node_id.of_int64 9L |> ok)
    ; C.Target.edge (Gpuio.Chart_data.Edge_id.of_int64 9L |> ok)
    ; target (cartesian Sum) data
    ; target (S.candlestick ~span:(span 0 1 9L 9L) ~aggregated:true |> ok) data
    ]
  in
  let entries =
    List.mapi targets ~f:(fun i target ->
      C.Entry.create
        ~target
        ~container:(if i % 2 = 0 then Card else Overlay)
        (fun () -> failwith "metadata must not invoke content"))
  in
  let content = C.create entries |> ok in
  assert (List.length (C.Expert.entries content) = 8);
  List.iter2_exn entries (C.Expert.entries content) ~f:(fun a b ->
    assert (C.Target.equal (C.Entry.target a) (C.Entry.target b));
    assert (phys_equal (C.Entry.content a) (C.Entry.content b)));
  let wire = C.Expert.metadata content ~data in
  assert (W.valid wire);
  print_endline (hex wire);
  [%expect
    {| 080100070900010109010102070900010309010104090001050901010604020b020007030209040100010604020b0203000109090101 |}]
;;

let%expect_test
    "singular identity survives reorder but aggregate identity fences publications"
  =
  let data = resource (R.Expert.Owner.create ()) 4L in
  let exact start = cartesian ~start ~length:1 ~last:9L Exact in
  let a = target (exact 0) data in
  let b =
    C.Target.of_selection (exact 99) ~data ~data_revision:12L ~data_generation:3L |> ok
  in
  assert (C.Target.equal a b);
  List.iter [ S.Aggregation.Sum; Mean ] ~f:(fun aggregation ->
    (* Length-one reductions remain publication-bound aggregates. *)
    let selection = cartesian ~length:1 ~last:9L aggregation in
    let a = target selection data in
    assert (not (C.Target.equal a (target (exact 0) data)));
    List.iter
      [ 12L, 2L; 11L, 3L ]
      ~f:(fun (data_revision, data_generation) ->
        let b =
          C.Target.of_selection selection ~data ~data_revision ~data_generation |> ok
        in
        assert (not (C.Target.equal a b))));
  List.iter
    [ 0L, 1L; 1L, 0L; -1L, 1L; 1L, -1L ]
    ~f:(fun (data_revision, data_generation) ->
      assert (
        Result.is_error
          (C.Target.of_selection (exact 0) ~data ~data_revision ~data_generation)));
  print_endline "exact IDs survive reorder; every aggregate is source/publication-bound";
  [%expect {| exact IDs survive reorder; every aggregate is source/publication-bound |}]
;;

let%expect_test "metadata hides aggregates from other resources or application owners" =
  let owner = R.Expert.Owner.create () in
  let data = resource owner 4L in
  let aggregate = target (cartesian Sum) data in
  let content =
    C.create
      [ C.Entry.create ~target:aggregate "aggregate"
      ; C.Entry.create ~target:(C.Target.slice (datum 9L)) "singular"
      ]
    |> ok
  in
  List.iter
    [ resource owner 5L; resource (R.Expert.Owner.create ()) 4L ]
    ~f:(fun other ->
      let wire = C.Expert.metadata content ~data:other in
      assert (W.valid wire);
      assert (Option.is_none (List.hd_exn wire).target);
      assert (Option.is_some (List.nth_exn wire 1).target));
  assert (Option.is_some (List.hd_exn (C.Expert.metadata content ~data)).target);
  assert (
    Result.is_error
      (C.Target.Expert.of_wire
         (C.Target.Expert.to_wire aggregate)
         ~data:(resource owner 5L)));
  let hidden : W.t =
    [ { target = None; container = Card }; { target = None; container = Overlay } ]
  in
  assert (W.valid hidden);
  print_endline (hex hidden);
  [%expect {| 0200000001 |}]
;;

let%expect_test "metadata rejects duplicate targets and oversized collections" =
  let data = resource (R.Expert.Owner.create ()) 4L in
  let entry i =
    C.Entry.create ~target:(C.Target.slice (datum (Int64.of_int (i + 1)))) ()
  in
  assert (Result.is_ok (C.create (List.init 128 ~f:entry)));
  assert (Result.is_error (C.create (List.init 129 ~f:entry)));
  assert (
    Result.is_error
      (C.create
         [ entry 0
         ; C.Entry.create ~target:(C.Entry.target (entry 0)) ~container:Overlay ()
         ]));
  let raw = C.Target.Expert.to_wire (target (cartesian Sum) data) in
  let invalid = [ W.Target.Slice 0L; Node (-1L); Cartesian (0L, 1L); Radar (1L, 0L) ] in
  List.iter invalid ~f:(fun target ->
    assert (Result.is_error (C.Target.Expert.of_wire target ~data)));
  (match raw with
   | Aggregate aggregate ->
     let exact =
       Gpuio.Chart_selection.Expert.to_wire (cartesian ~length:1 ~last:9L Exact)
     in
     List.iter
       [ W.Target.Aggregate { aggregate with data_revision = 0L }
       ; Aggregate { aggregate with data_generation = 0L }
       ; Aggregate { aggregate with selection = exact }
       ]
       ~f:(fun target -> assert (Result.is_error (C.Target.Expert.of_wire target ~data)))
   | Cartesian _ | Slice _ | Radar _ | Candlestick _ | Node _ | Edge _ -> assert false);
  let metadata = C.Expert.metadata (C.create [ entry 0 ] |> ok) ~data in
  assert (not (W.valid (metadata @ metadata)));
  assert (
    not
      (W.valid (List.init 129 ~f:(fun _ -> { W.Entry.target = None; container = Card }))));
  print_endline
    "128 bounded slots; duplicate identities rejected independently of content/container";
  [%expect
    {| 128 bounded slots; duplicate identities rejected independently of content/container |}]
;;
