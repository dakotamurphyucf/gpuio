open Core
module W = Signal_studio_model.Workspace
module G = Gpuio.Canvas_geometry

let ok = Or_error.ok_exn
let id i = Gpuio.Canvas_scene.Item_id.of_int64 i |> ok

let%expect_test "document round trip keeps identities, selection, run and bounded motion" =
  let t = W.create () |> fun t -> W.set_run t 42 |> ok in
  let t = W.move t (id 1L) (G.Point.create ~x:900. ~y:(-20.) |> ok) |> ok in
  let t = W.select t (Some (id 1L)) |> ok in
  let text = W.encode t in
  let restored = W.decode text |> ok in
  let sample = W.selected restored |> Option.value_exn in
  print_s
    [%sexp
      (String.equal text (W.encode restored) : bool)
    , (W.run restored : int)
    , (W.Sample.name sample : string)
    , (W.Sample.position sample : G.Point.t)];
  [%expect {| (true 42 Swift ((x 634) (y 74))) |}]
;;

let%expect_test "untrusted document fields fail validation" =
  let text = W.encode (W.create ()) in
  let check text = print_s [%sexp (Result.is_error (W.decode text) : bool)] in
  List.iter
    [ ""
    ; String.make 16385 'x'
    ; String.substr_replace_first text ~pattern:"(version 1)" ~with_:"(version 2)"
    ; String.substr_replace_first text ~pattern:"(run 0)" ~with_:"(run 101)"
    ; String.substr_replace_first text ~pattern:"(id 1)" ~with_:"(id 2)"
    ; String.substr_replace_first text ~pattern:"(x 180)" ~with_:"(x nan)"
    ; String.substr_replace_first text ~pattern:"(x 180)" ~with_:"(x 900)"
    ; String.substr_replace_first text ~pattern:"(selected ())" ~with_:"(selected (99))"
    ]
    ~f:check;
  [%expect
    {|
    true
    true
    true
    true
    true
    true
    true
    true
    |}]
;;

let%expect_test "routes only select known samples, never acquire I/O authority" =
  List.iter
    [ "gpuio-signal://sample/2"
    ; "gpuio-signal://sample/02"
    ; "gpuio-signal://sample/2?file=/tmp/test"
    ; "gpuio-signal://sample/2#ignored"
    ; "gpuio-signal://open/2"
    ]
    ~f:(fun uri ->
      let link =
        match Gpuio.Deep_link.of_string ~schemes:[ W.scheme ] uri with
        | Ok link -> link
        | Error error -> raise_s [%sexp (error : Gpuio.Deep_link.Error.t)]
      in
      match W.route (W.create ()) link with
      | Ok t ->
        print_s [%sexp (Option.map (W.selected t) ~f:W.Sample.name : string option)]
      | Error _ -> print_endline "rejected");
  [%expect
    {|
    (Sage)
    rejected
    rejected
    rejected
    rejected
    |}]
;;

let%expect_test
    "all valid runs make bounded validated scenes/charts and survive documents"
  =
  for run = 0 to 100 do
    let t = W.set_run (W.create ()) run |> ok in
    assert (String.equal (W.encode t) (W.encode (W.decode (W.encode t) |> ok)));
    ignore (W.scene t : Gpuio.Canvas_scene.t);
    let chart = W.chart t in
    let encoded = Gpuio.Chart_data.Expert.encode chart |> ok in
    assert (Gpuio.Chart_data.equal chart (Gpuio.Chart_data.Expert.decode encoded |> ok))
  done;
  print_s
    [%sexp
      (Result.is_error (W.set_run (W.create ()) (-1)) : bool)
    , (Result.is_error (W.select (W.create ()) (Some (id 99L))) : bool)];
  [%expect {| (true true) |}]
;;
