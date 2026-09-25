open Core
module S = Gpuio.Split_pane
module R = Gpuio.Reconciler
module V = Gpuio.View
module Wire = Gpuio_protocol.Wire

let%expect_test "split config validates bounds and finite dimensions" =
  List.iter
    [ S.Config.create ~label:"Workspace" ()
    ; S.Config.create ~label:" " ()
    ; S.Config.create ~label:"x" ~minimum_first:300. ()
    ; S.Config.create ~label:"x" ~maximum_first:10. ()
    ; S.Config.create ~label:"x" ~keyboard_step:0. ()
    ; S.Config.create ~label:"x" ~initial_first:Float.nan ()
    ; S.Config.create ~label:"x" ~minimum_second:Float.infinity ()
    ; S.Config.create ~label:"x" ~reset_generation:(-1L) ()
    ]
    ~f:(fun result -> print_s [%sexp (Result.is_ok result : bool)]);
  [%expect
    {|
    true
    false
    false
    false
    false
    false
    false
    false
    |}]
;;

let%expect_test "reset preserves children and fences old geometry callbacks" =
  let window =
    Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn
  in
  let r = R.create window in
  let view generation =
    V.split_pane
      ~config:
        (S.Config.create ~label:"Workspace" ~reset_generation:generation ()
         |> Or_error.ok_exn)
      ~first:(V.text "Sidebar")
      ~second:(V.text "Transcript")
      ~on_resize:Fn.id
      ()
  in
  let prepare generation =
    R.prepare r ~theme:Gpuio.Theme.default (Some (view generation)) |> Or_error.ok_exn
  in
  let initial = prepare 0L in
  let node, handler =
    match R.message initial with
    | Some (Wire.Message.Apply tx) ->
      List.find_map_exn tx.operations ~f:(function
        | Wire.Op.Create (id, Split_pane, _, Some handler) -> Some (id, handler)
        | _ -> None)
    | _ -> failwith "initial tree"
  in
  R.accept r initial |> Or_error.ok_exn;
  let snapshot : Gpuio_protocol.Split_wire.Snapshot.t = { first = 200.; second = 600. } in
  let event handler generation =
    Wire.Event.Split_resized (window, node, handler, R.revision r, generation, snapshot)
  in
  print_s [%sexp (Option.is_some (R.dispatch r (event handler 0L)) : bool)];
  let reset = prepare 1L in
  let next_handler =
    match R.message reset with
    | Some (Wire.Message.Apply tx) ->
      print_s
        [%sexp
          (List.count tx.operations ~f:(function
             | Wire.Op.Create _ | Remove _ -> true
             | _ -> false)
           : int)];
      List.find_map_exn tx.operations ~f:(function
        | Wire.Op.Bind (_, Some handler) -> Some handler
        | _ -> None)
    | _ -> failwith "geometry reset"
  in
  R.accept r reset |> Or_error.ok_exn;
  print_s [%sexp (Option.is_some (R.dispatch r (event handler 0L)) : bool)];
  print_s [%sexp (Option.is_some (R.dispatch r (event next_handler 0L)) : bool)];
  print_s [%sexp (Option.is_some (R.dispatch r (event next_handler 1L)) : bool)];
  assert (Result.is_error (R.prepare r ~theme:Gpuio.Theme.default (Some (view 0L))));
  R.close r;
  print_s [%sexp (Option.is_some (R.dispatch r (event next_handler 1L)) : bool)];
  [%expect
    {|
    true
    0
    false
    false
    true
    false
    |}]
;;
