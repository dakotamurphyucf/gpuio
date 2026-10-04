open Core
open Gpuio
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok

let%expect_test "motion changes retain modal and child ownership" =
  let r = Reconciler.create window in
  let view motion =
    View.dialog
      ~motion
      ~config:(Overlay.Config.create ~label:"Preview" () |> ok)
      ~on_dismiss:ignore
      (Some (View.button ~on_click:(fun () -> ()) "Keep"))
  in
  let update motion =
    let update = Reconciler.prepare r ~theme:Theme.default (Some (view motion)) |> ok in
    Reconciler.accept r update |> ok;
    match Reconciler.message update with
    | Some (W.Message.Apply tx) -> tx.operations
    | None -> []
    | Some _ -> assert false
  in
  let initial = update Overlay.Motion.Enter in
  let node =
    List.find_map_exn initial ~f:(function
      | W.Op.Set_overlay_motion (node, true) -> Some node
      | _ -> None)
  in
  List.iter
    [ Overlay.Motion.Immediate, false; Enter, true ]
    ~f:(fun (motion, entering) ->
      let operations = update motion in
      assert (
        List.exists operations ~f:(function
          | W.Op.Set_overlay_motion (id, enabled) ->
            Gpuio_protocol.Node_id.equal id node && Bool.equal enabled entering
          | _ -> false));
      assert (
        not
          (List.exists operations ~f:(function
             | W.Op.Create _ | Remove _ | Set_overlay _ | Set_focus_scope _ -> true
             | _ -> false))));
  print_endline "entry option changes retain focus scope and child identity";
  [%expect {| entry option changes retain focus scope and child identity |}]
;;

let%expect_test "modal entry operation matches independent bytes" =
  let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok in
  let msg =
    W.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations = [ Set_overlay_motion (node, true); Set_overlay_motion (node, false) ]
      }
  in
  let bytes = W.Message.encode msg |> ok in
  let hex =
    String.to_list bytes
    |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
    |> String.concat
  in
  Eio_main.run (fun env ->
    assert (
      String.equal
        hex
        (Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "overlay-motion.hex") |> String.strip)));
  print_endline "Op91 entry/immediate match independent bytes";
  [%expect {| Op91 entry/immediate match independent bytes |}]
;;
