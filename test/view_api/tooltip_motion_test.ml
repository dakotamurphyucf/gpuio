open Core
open Gpuio
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok

let%expect_test "tooltip motion changes retain content and timing configuration" =
  let r = Reconciler.create window in
  let view motion =
    View.tooltip
      ~config:(Tooltip.Config.create ~label:"Preview" ~motion () |> ok)
      ~anchor:(View.button ~on_click:(fun () -> ()) "Anchor")
      ~content:(View.button ~on_click:(fun () -> ()) "Keep")
      ()
  in
  let update motion =
    let update = Reconciler.prepare r ~theme:Theme.default (Some (view motion)) |> ok in
    Reconciler.accept r update |> ok;
    match Reconciler.message update with
    | Some (W.Message.Apply tx) -> tx.operations
    | None -> []
    | Some _ -> assert false
  in
  let initial = update Tooltip.Motion.Enter_and_switch in
  let node =
    List.find_map_exn initial ~f:(function
      | W.Op.Set_tooltip_motion (node, true) -> Some node
      | _ -> None)
  in
  List.iter
    [ Tooltip.Motion.Immediate, false; Enter_and_switch, true ]
    ~f:(fun (motion, entering) ->
      let operations = update motion in
      assert (
        List.exists operations ~f:(function
          | W.Op.Set_tooltip_motion (id, enabled) ->
            Gpuio_protocol.Node_id.equal id node && Bool.equal enabled entering
          | _ -> false));
      assert (
        not
          (List.exists operations ~f:(function
             | W.Op.Create _ | Remove _ | Set_tooltip _ | Set_focus_scope _ -> true
             | _ -> false))));
  print_endline "tooltip motion retains children and native timing config";
  [%expect {| tooltip motion retains children and native timing config |}]
;;

let%expect_test "tooltip motion operation matches independent bytes" =
  let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok in
  let msg =
    W.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations = [ Set_tooltip_motion (node, true); Set_tooltip_motion (node, false) ]
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
        (Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "tooltip-motion.hex") |> String.strip)));
  print_endline "Op92 entry-switch/immediate match independent bytes";
  [%expect {| Op92 entry-switch/immediate match independent bytes |}]
;;
