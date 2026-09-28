open Core
open Gpuio
module W = Gpuio_protocol.Highlight_wire
module Wire = Gpuio_protocol.Wire

let ok = Or_error.ok_exn

let hex bytes =
  Bigstring.to_string bytes
  |> String.to_list
  |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
  |> String.concat
;;

let states : W.State.t list =
  [ Pending
  ; Ready [ { total = 5L; stored = 3L }; { total = 0L; stored = 0L } ]
  ; Invalid_range { spec_index = 1L; range_index = 2L; reason = Scalar_boundary }
  ; Capacity Source
  ; Capacity Work
  ; Capacity Admission
  ; Failed Source_unavailable
  ; Failed Worker_failed
  ; Failed Epoch_exhausted
  ; Invalid_range { spec_index = 0L; range_index = 0L; reason = Out_of_bounds }
  ]
;;

let%expect_test "all observations agree with independent Rust bytes" =
  List.iteri states ~f:(fun i state ->
    let wire : W.Observation.t = { epoch = Int64.of_int (i + 1); state } in
    ignore (Highlight.Expert.observation_of_wire wire |> ok : Highlight.Observation.t);
    let bytes = Bin_prot.Utils.bin_dump W.Observation.bin_writer_t wire in
    let pos_ref = ref 0 in
    assert (W.Observation.equal (W.Observation.bin_read_t bytes ~pos_ref) wire);
    assert (!pos_ref = Bigstring.length bytes);
    print_endline (hex bytes));
  [%expect
    {|
    0100
    02010205030000
    0302010201
    040300
    050301
    060302
    070400
    080401
    090402
    0a02000000
    |}]
;;

let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok
let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok
let handler generation = Gpuio_protocol.Handler_id.create ~slot:0L ~generation |> ok

let event ?(generation = 1L) ?(revision = 1L) state =
  Wire.Event.Highlight_observed
    (window, node, handler generation, revision, { epoch = 1L; state })
;;

let%expect_test "scope envelopes append without changing previous protocol tags" =
  let request =
    Wire.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Create (node, Highlight_scope, "", Some (handler 1L))
          ; Set_highlight_scope (node, [])
          ; Set_root (Some node)
          ]
      }
  in
  print_endline (Wire.Message.encode request |> ok |> Bigstring.of_string |> hex);
  let events = [ event Pending ] in
  let bytes = Bin_prot.Utils.bin_dump [%bin_writer: Wire.Event.t list] events in
  print_endline (hex bytes);
  assert (
    List.equal
      Wire.Event.equal
      events
      (Wire.Event.decode (Bigstring.to_string bytes) |> ok));
  [%expect
    {|
    03000100010300000132000100013900010006010001
    0140000100010001010100
    |}]
;;

let%expect_test "bounded native decoding and malformed observations" =
  List.iter
    [ { W.Observation.epoch = 0L; state = Pending }
    ; { epoch = 1L; state = Ready [ { total = 1L; stored = 2L } ] }
    ; { epoch = 1L
      ; state =
          Ready (List.init 16 ~f:(fun _ -> { W.Count.total = 2000L; stored = 1025L }))
      }
    ; { epoch = 1L
      ; state =
          Invalid_range { spec_index = 16L; range_index = 0L; reason = Out_of_bounds }
      }
    ]
    ~f:(fun observation ->
      assert (Result.is_error (Highlight.Expert.observation_of_wire observation));
      let bytes = Bin_prot.Utils.bin_dump W.Observation.bin_writer_t observation in
      assert (
        Result.is_error
          (Or_error.try_with (fun () -> W.Observation.bin_read_t bytes ~pos_ref:(ref 0)))));
  (* Epoch1, Ready, hostile list count. Bound before reserving/reading elements. *)
  List.iter [ "\001\001\017"; "\001\001\016"; "\001\001\254\255\255" ] ~f:(fun bytes ->
    assert (
      Result.is_error
        (Or_error.try_with (fun () ->
           W.Observation.bin_read_t (Bigstring.of_string bytes) ~pos_ref:(ref 0)))));
  let bytes =
    Bin_prot.Utils.bin_dump
      [%bin_writer: Wire.Event.t list]
      [ event (Ready [ { total = 2L; stored = 2L } ]) ]
    |> Bigstring.to_string
  in
  for n = 0 to String.length bytes - 1 do
    assert (Result.is_error (Wire.Event.decode (String.prefix bytes n)))
  done;
  assert (Result.is_error (Wire.Event.decode (bytes ^ "\000")));
  print_endline
    "epoch, indices, total/stored, aggregate bound, hostile counts and exact consumption";
  [%expect
    {| epoch, indices, total/stored, aggregate bound, hostile counts and exact consumption |}]
;;

let%expect_test "scope config retires callbacks while retaining children" =
  let config radius =
    Highlight.Config.create
      [ Highlight.Spec.create
          ~query:(Highlight.Query.create "a" |> ok)
          ~appearance:(Highlight.Appearance.create ~radius () |> ok)
          ()
        |> ok
      ]
    |> ok
  in
  let r = Reconciler.create window in
  let prepare ?on_update config =
    Reconciler.prepare
      r
      ~theme:Theme.default
      (Some
         (View.highlight_scope
            ~config
            ?on_update
            [ View.text ~key:(Key.of_string_exn "child") "a" ]))
    |> ok
  in
  let apply p = Reconciler.accept r p |> ok in
  apply (prepare (config 2.) ~on_update:(fun _ -> "old"));
  let ready : W.State.t = Ready [ { W.Count.total = 1L; stored = 1L } ] in
  assert (Option.equal String.equal (Reconciler.dispatch r (event ready)) (Some "old"));
  let closure = prepare (config 2.) ~on_update:(fun _ -> "latest") in
  assert (Option.is_none (Reconciler.message closure));
  apply closure;
  assert (Option.equal String.equal (Reconciler.dispatch r (event ready)) (Some "latest"));
  let changed = prepare (config 4.) ~on_update:(fun _ -> "changed") in
  (match Reconciler.message changed with
   | Some
       (Apply
          { operations = [ Bind (bound, Some fresh); Set_highlight_scope (configured, _) ]
          ; _
          }) ->
     assert (
       Gpuio_protocol.Node_id.equal bound node
       && Gpuio_protocol.Node_id.equal configured node);
     assert (Gpuio_protocol.Handler_id.equal fresh (handler 2L))
   | _ -> assert false);
  apply changed;
  assert (Option.is_none (Reconciler.dispatch r (event ready)));
  assert (
    Option.equal
      String.equal
      (Reconciler.dispatch r (event ~generation:2L ~revision:2L ready))
      (Some "changed"));
  assert (
    Option.is_none (Reconciler.dispatch r (event ~generation:2L ~revision:2L (Ready []))));
  assert (Option.is_none (Reconciler.dispatch r (event ~generation:2L ~revision:3L ready)));
  apply (prepare (config 4.));
  assert (Option.is_none (Reconciler.dispatch r (event ~generation:2L ~revision:2L ready)));
  apply (Reconciler.prepare r ~theme:Theme.default None |> ok);
  print_endline
    "latest closure, optional observer, config generations, retained child and stale \
     rejection";
  [%expect
    {| latest closure, optional observer, config generations, retained child and stale rejection |}]
;;
