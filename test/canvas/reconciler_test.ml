open Core
module R = Gpuio.Reconciler
module Canvas = Gpuio.Canvas
module Scene = Gpuio.Canvas_scene
module Wire = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok
let source = Gpuio_protocol.Resource_id.create ~slot:7L ~generation:2L |> ok

let operations update =
  match R.message update with
  | Some (Apply transaction) -> transaction.operations
  | _ -> []
;;

let%expect_test "canvas reconciliation fences replaced handlers and foreign sources" =
  let owner = Scene.Expert.Owner.create () in
  let handle = Scene.Expert.handle ~owner source in
  let r = R.create ~canvas_owner:owner window in
  let config ?(disabled = false) () =
    Canvas.Config.create ~scene:handle ~disabled () |> ok
  in
  let prepare ?(callback = Fn.id) config =
    R.prepare
      r
      ~theme:Gpuio.Theme.default
      (Some (Gpuio.View.canvas ~on_event:callback config))
    |> ok
  in
  let initial = prepare (config ()) in
  let node, handler =
    List.find_map_exn (operations initial) ~f:(function
      | Create (node, Canvas_view, _, Some handler) -> Some (node, handler)
      | _ -> None)
  in
  assert (
    List.exists (operations initial) ~f:(function
      | Set_canvas (_, config) ->
        Option.equal Gpuio_protocol.Resource_id.equal config.source (Some source)
      | _ -> false));
  R.accept r initial |> ok;
  let event ?(source = Some source) ?(revision = 1L) handler observation =
    Wire.Event.Canvas_event (window, node, handler, revision, source, 2L, 1L, observation)
  in
  let activated = event handler (Activated 3L) in
  assert (Option.is_some (R.dispatch r activated));
  assert (Option.is_none (R.dispatch r (event ~source:None handler (Activated 3L))));
  assert (Option.is_none (R.dispatch r (event ~revision:2L handler (Activated 3L))));
  assert (Option.is_none (R.dispatch r (event handler (Activated 0L))));
  let called = ref false in
  let refreshed =
    prepare
      ~callback:(fun value ->
        called := true;
        value)
      (config ())
  in
  assert (List.is_empty (operations refreshed));
  R.accept r refreshed |> ok;
  ignore (R.dispatch r activated : Canvas.Event.t option);
  assert !called;
  let changed = prepare (config ~disabled:true ()) in
  let next_handler =
    List.find_map_exn (operations changed) ~f:(function
      | Bind (_, Some handler) -> Some handler
      | _ -> None)
  in
  assert (
    not
      (List.exists (operations changed) ~f:(function
         | Create _ | Remove _ -> true
         | _ -> false)));
  R.accept r changed |> ok;
  assert (Option.is_none (R.dispatch r activated));
  assert (Option.is_some (R.dispatch r (event next_handler (Command_completed 9L))));
  let foreign_handle = Scene.Expert.handle ~owner:(Scene.Expert.Owner.create ()) source in
  let foreign = prepare (Canvas.Config.create ~scene:foreign_handle () |> ok) in
  let foreign_handler =
    List.find_map_exn (operations foreign) ~f:(function
      | Bind (_, Some handler) -> Some handler
      | _ -> None)
  in
  assert (
    List.exists (operations foreign) ~f:(function
      | Set_canvas (_, c) -> Option.is_none c.source
      | _ -> false));
  R.accept r foreign |> ok;
  assert (Option.is_none (R.dispatch r (event foreign_handler (Activated 3L))));
  let failed =
    Wire.Event.Canvas_event
      (window, node, foreign_handler, 1L, None, 0L, 0L, Failed Wrong_application)
  in
  assert (Option.is_some (R.dispatch r failed));
  R.prepare r ~theme:Gpuio.Theme.default None |> ok |> R.accept r |> ok;
  assert (Option.is_none (R.dispatch r failed));
  R.close r;
  print_endline "callback refresh, config rotation, ownership and unmount fences pass";
  [%expect {| callback refresh, config rotation, ownership and unmount fences pass |}]
;;
