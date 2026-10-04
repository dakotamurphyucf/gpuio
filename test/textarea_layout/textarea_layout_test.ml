open Core
open Gpuio
module L = Text_area_layout
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn

let%expect_test "textarea layout defaults and bounds preserve legacy config bytes" =
  assert (L.soft_wrap L.default);
  assert (L.Wrapping_indent.equal (L.wrapping_indent L.default) Match_first_line);
  assert (not (L.show_whitespace L.default));
  assert (Option.is_none (L.cursor_margin_lines L.default));
  List.iter [ -1; 257; Int.max_value ] ~f:(fun n ->
    assert (Result.is_error (L.create ~cursor_margin_lines:n ())));
  List.iter [ 0; 256 ] ~f:(fun n ->
    assert (
      Option.equal
        Int.equal
        (L.cursor_margin_lines (L.create ~cursor_margin_lines:n () |> ok))
        (Some n)));
  assert (
    Result.is_error
      (Text_input.Config.create ~mode:Single_line ~label:"Input" ~layout:L.default ()));
  let config layout =
    Text_input.Config.create ~mode:Multiline ~label:"Notes" ?layout () |> ok
  in
  assert (
    W.Editor.Config.equal
      (Text_input.Expert.config_to_wire (config None))
      (Text_input.Expert.config_to_wire (config (Some L.default))));
  print_endline
    "native defaults; 0..256 margins; multiline only; legacy editor bytes unchanged";
  [%expect
    {| native defaults; 0..256 margins; multiline only; legacy editor bytes unchanged |}]
;;

let%expect_test "operation 78 has independent bytes and layout-only reconciliation" =
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let node = Gpuio_protocol.Node_id.create ~slot:1L ~generation:2L |> ok in
  let layout =
    L.create
      ~soft_wrap:false
      ~wrapping_indent:Flush_left
      ~show_whitespace:true
      ~cursor_margin_lines:256
      ()
    |> ok
  in
  let operation node layout =
    W.Op.Set_text_area_layout (node, Option.map layout ~f:L.Expert.to_wire)
  in
  let message =
    W.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations = List.map [ None; Some L.default; Some layout ] ~f:(operation node)
      }
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
        (Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "textarea-layout-operation.hex")
         |> String.strip)));
  let reconciler = Reconciler.create window in
  let render layout =
    let config =
      Text_input.Config.create ~mode:Multiline ~label:"Notes" ?layout () |> ok
    in
    let view =
      View.text_input
        ~controller:(Key.of_string_exn "notes")
        ~config
        ~initial_text:"  existing draft\nwith history"
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
    List.find_map_exn (render None) ~f:(function
      | W.Op.Create (id, Textarea, _, _) -> Some id
      | _ -> None)
  in
  List.iter [ Some layout; Some L.default; None ] ~f:(fun layout ->
    assert (List.equal W.Op.equal (render layout) [ operation mounted layout ]));
  assert (List.is_empty (render None));
  print_endline "paired operation 78; one retained textarea; no seed replacement";
  [%expect {| paired operation 78; one retained textarea; no seed replacement |}]
;;
