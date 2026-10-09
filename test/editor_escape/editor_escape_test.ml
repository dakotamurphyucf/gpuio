open Core
open Gpuio
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn

let%expect_test "Escape policy is opt-in and leaves legacy config unchanged" =
  List.iter [ Text_input.Mode.Single_line; Multiline ] ~f:(fun mode ->
    let config = Text_input.Config.create ~mode ~label:"Draft" () |> ok in
    let clear =
      Text_input.Config.create ~mode ~label:"Draft" ~clear_on_escape:true () |> ok
    in
    assert (not (Text_input.Config.clear_on_escape config));
    assert (Text_input.Config.clear_on_escape clear);
    assert (
      W.Editor.Config.equal
        (Text_input.Expert.config_to_wire config)
        (Text_input.Expert.config_to_wire clear)));
  print_endline "both modes; off by default; legacy configuration unchanged";
  [%expect {| both modes; off by default; legacy configuration unchanged |}]
;;

let%expect_test "operation 79 independently encoded and reconciled without reseeding" =
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let node = Gpuio_protocol.Node_id.create ~slot:1L ~generation:2L |> ok in
  let op id flag = W.Op.Set_editor_clear_on_escape (id, flag) in
  let message =
    W.Message.Apply
      { window; base = 0L; revision = 1L; operations = [ op node false; op node true ] }
  in
  let hex =
    W.Message.encode message
    |> ok
    |> String.to_list
    |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
    |> String.concat
  in
  Eio_main.run (fun env ->
    assert (
      String.equal
        hex
        (Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "editor-escape-operation.hex")
         |> String.strip)));
  let reconciler = Reconciler.create window in
  let render clear_on_escape =
    let config =
      Text_input.Config.create ~mode:Single_line ~label:"Draft" ~clear_on_escape () |> ok
    in
    let view =
      View.text_input
        ~controller:(Key.of_string_exn "draft")
        ~config
        ~initial_text:"seed"
        ~on_event:(fun _ -> ())
        ()
      |> ok
    in
    let update = Reconciler.prepare reconciler ~theme:Theme.default (Some view) |> ok in
    let message = Reconciler.message update in
    Reconciler.accept reconciler update |> ok;
    match message with
    | Some (W.Message.Apply tx) -> tx.operations
    | _ -> []
  in
  let mounted =
    List.find_map_exn (render false) ~f:(function
      | W.Op.Create (id, Input, _, _) -> Some id
      | _ -> None)
  in
  List.iter [ true; false ] ~f:(fun flag ->
    assert (List.equal W.Op.equal (render flag) [ op mounted flag ]));
  assert (List.is_empty (render false));
  print_endline "paired operation 79; one retained input; only policy updates";
  [%expect {| paired operation 79; one retained input; only policy updates |}]
;;
