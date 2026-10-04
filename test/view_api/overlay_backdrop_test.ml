open Core
open Gpuio
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok

let theme color =
  Theme.create
    [ "background", Color.rgb_exn 0xffffff
    ; "foreground", Color.rgb_exn 0
    ; "accent", Color.rgb_exn 0x386ac8
    ; "muted", Color.rgb_exn 0x888888
    ; "shade", color
    ]
  |> ok
;;

let%expect_test "theme backdrop edits retain modal ownership and reset explicitly" =
  let r = Reconciler.create window in
  let view backdrop =
    View.dialog
      ?backdrop
      ~config:(Overlay.Config.create ~label:"Preview" () |> ok)
      ~on_dismiss:ignore
      (Some (View.button ~on_click:(fun () -> ()) "Keep"))
  in
  let update theme view =
    let update = Reconciler.prepare r ~theme (Some view) |> ok in
    Reconciler.accept r update |> ok;
    match Reconciler.message update with
    | Some (W.Message.Apply tx) -> tx.operations
    | None -> []
    | Some _ -> assert false
  in
  let themed = view (Some (Color.token_exn "shade")) in
  let first = update (theme (Color.rgb_exn 0x112233)) themed in
  let node =
    List.find_map_exn first ~f:(function
      | W.Op.Set_overlay_backdrop (node, Some 0x112233ffL) -> Some node
      | _ -> None)
  in
  let second = update (theme (Color.rgb_exn 0x445566)) themed in
  assert (
    List.exists second ~f:(function
      | W.Op.Set_overlay_backdrop (id, Some 0x445566ffL) ->
        Gpuio_protocol.Node_id.equal id node
      | _ -> false));
  assert (
    not
      (List.exists second ~f:(function
         | W.Op.Create _ | Remove _ | Set_overlay _ | Set_focus_scope _ -> true
         | _ -> false)));
  let reset = update Theme.default (view None) in
  assert (
    List.exists reset ~f:(function
      | W.Op.Set_overlay_backdrop (id, None) -> Gpuio_protocol.Node_id.equal id node
      | _ -> false));
  assert (Or_error.is_error (Reconciler.prepare r ~theme:Theme.default (Some themed)));
  print_endline
    "theme-only update retains panel/focus; default reset explicit; missing token \
     rejected";
  [%expect
    {| theme-only update retains panel/focus; default reset explicit; missing token rejected |}]
;;

let%expect_test "backdrop color and transparent reset match independent bytes" =
  let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok in
  let msg =
    W.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Set_overlay_backdrop (node, Some 0x12345678L)
          ; Set_overlay_backdrop (node, Some 0L)
          ; Set_overlay_backdrop (node, None)
          ]
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
        (Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "overlay-backdrop.hex")
         |> String.strip)));
  print_endline "Op90 RGBA/transparent/default match independent bytes";
  [%expect {| Op90 RGBA/transparent/default match independent bytes |}]
;;
