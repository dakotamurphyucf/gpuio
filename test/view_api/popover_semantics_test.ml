open Core
open Gpuio
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok

let commit r view =
  let update = Reconciler.prepare r ~theme:Theme.default (Some view) |> ok in
  Reconciler.accept r update |> ok;
  match Reconciler.message update with
  | Some (W.Message.Apply tx) -> tx.operations
  | None -> []
  | Some _ -> assert false
;;

let config = Overlay.Config.create ~label:"Preview" () |> ok
let popup ~anchor content = View.popover ~config ~on_dismiss:(fun _ -> ()) ~anchor content

let%expect_test "popover marks only its composite and retains plain/rich/custom anchors" =
  List.iter [ false; true ] ~f:(fun custom ->
    let r = Reconciler.create window in
    let button = View.button ~on_click:(fun () -> ()) "Open" in
    let anchor = if custom then View.row [ button ] else button in
    let first = commit r (popup ~anchor None) in
    let owner =
      List.find_map_exn first ~f:(function
        | W.Op.Set_popover (id, true) -> Some id
        | _ -> None)
    in
    let button_id =
      List.find_map_exn first ~f:(function
        | W.Op.Create (id, Button, _, _) -> Some id
        | _ -> None)
    in
    let stable ops =
      assert (
        not
          (List.exists ops ~f:(function
             | W.Op.Remove id ->
               Gpuio_protocol.Node_id.equal id owner
               || Gpuio_protocol.Node_id.equal id button_id
             | W.Op.Set_popover _ -> true
             | _ -> false)))
    in
    stable (commit r (popup ~anchor (Some (View.text "Draft"))));
    stable (commit r (popup ~anchor None));
    if not custom
    then (
      let anchor =
        View.button_with_content
          ~accessible_name:"Open"
          ~on_click:(fun () -> ())
          (View.text "Rich caption")
        |> ok
      in
      stable (commit r (popup ~anchor None)));
    let reset = commit r (View.row [ anchor ]) in
    assert (
      List.exists reset ~f:(function
        | W.Op.Set_popover (id, false) -> Gpuio_protocol.Node_id.equal id owner
        | _ -> false)));
  print_endline
    "one composite marker; open/close/rich update retain owner and button; reset clears \
     marker";
  [%expect
    {| one composite marker; open/close/rich update retain owner and button; reset clears marker |}]
;;

let%expect_test "independent popover marker bytes" =
  let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok in
  let message =
    W.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations = [ Set_popover (node, true); Set_popover (node, false) ]
      }
  in
  let bytes = W.Message.encode message |> ok in
  let hex =
    String.to_list bytes
    |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
    |> String.concat
  in
  Eio_main.run (fun env ->
    let expected =
      Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "popover-operation.hex") |> String.strip
    in
    assert (String.equal hex expected));
  print_endline "Op88 true/false match independent bytes; no previous operation changed";
  [%expect {| Op88 true/false match independent bytes; no previous operation changed |}]
;;
