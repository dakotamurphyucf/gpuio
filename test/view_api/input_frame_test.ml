open Core
open Gpuio
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn

let input ?(privacy = Text_input.Privacy.Plain) ?(mode = Text_input.Mode.Single_line) () =
  View.text_input
    ~controller:(Key.of_string_exn "draft")
    ~config:(Text_input.Config.create ~mode ~privacy ~label:"Draft" () |> ok)
    ~initial_text:"seed"
    ~on_event:(fun _ -> ())
    ()
  |> ok
;;

let%expect_test "frame validates labels, spacing and editor kind" =
  List.iter
    [ ""; " \t"; "a\000b"; "\255"; String.make 4097 'x' ]
    ~f:(fun label -> assert (Result.is_error (Input_frame.create ~clear_label:label ())));
  List.iter [ Float.nan; Float.infinity; -1.; 256.1 ] ~f:(fun gap ->
    assert (Result.is_error (Input_frame.create ~gap ())));
  let clear = Input_frame.create ~clear_label:"Clear" () |> ok in
  assert (Result.is_error (View.input_frame ~config:clear (input ~mode:Multiline ())));
  assert (Result.is_error (View.input_frame ~on_reveal:(fun () -> ()) (input ())));
  assert (Result.is_error (View.input_frame (View.text "not an editor")));
  assert (Result.is_ok (View.input_frame (input ~mode:Multiline ())));
  assert (
    Result.is_ok
      (View.input_frame ~on_reveal:(fun () -> ()) (input ~privacy:(Password Hidden) ())));
  print_endline "invalid frames rejected; textarea and password contracts accepted";
  [%expect {| invalid frames rejected; textarea and password contracts accepted |}]
;;

let%expect_test "frame operation has independent paired bytes" =
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let node = Gpuio_protocol.Node_id.create ~slot:1L ~generation:2L |> ok in
  let message =
    W.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Set_editor_frame (node, None)
          ; Set_editor_frame
              (node, Some { clear_label = Some "Clear"; loading = true; gap = 6. })
          ; Set_editor_frame (node, Some { clear_label = None; loading = false; gap = 0. })
          ]
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
    let expected =
      Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "editor-frame-operation.hex")
      |> String.strip
    in
    assert (String.equal hex expected));
  print_endline "frame tag 74: absent, loading clear, empty frame";
  [%expect {| frame tag 74: absent, loading clear, empty frame |}]
;;

let%expect_test "changing frame slots retains editor identity and caller keys" =
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let r = Reconciler.create window in
  let render framed loading leading =
    let view = input ~privacy:(Password Hidden) () in
    let view =
      if framed
      then
        View.input_frame
          ~config:(Input_frame.create ~clear_label:"Clear" ~loading () |> ok)
          ?leading
          ~on_reveal:(fun () -> ())
          view
        |> ok
      else view
    in
    let update = Reconciler.prepare r ~theme:Theme.default (Some view) |> ok in
    let message = Reconciler.message update in
    Reconciler.accept r update |> ok;
    match message with
    | Some (W.Message.Apply tx) -> tx.operations
    | _ -> []
  in
  let first = render false false None in
  let node =
    List.find_map_exn first ~f:(function
      | W.Op.Create (id, Input, _, _) -> Some id
      | _ -> None)
  in
  let leading = Some (View.text ~key:(Key.of_string_exn "prefix") "Key") in
  List.iter
    [ true, false; true, true; true, false; false, false ]
    ~f:(fun (framed, loading) ->
      let ops = render framed loading leading in
      assert (
        List.exists ops ~f:(function
          | W.Op.Set_editor_frame (id, _) -> Gpuio_protocol.Node_id.equal id node
          | _ -> false));
      assert (
        List.for_all ops ~f:(function
          | W.Op.Create (_, (Input | Textarea), _, _) | Set_editor _ | Set_text _ -> false
          | Remove id -> not (Gpuio_protocol.Node_id.equal id node)
          | _ -> true)));
  assert (List.is_empty (render false false None));
  let child = View.text ~key:(Key.of_string_exn "prefix") "Key" in
  let framed = View.input_frame ~leading:child (input ()) |> ok |> View.Expert.describe in
  let wrapper = List.hd_exn framed.children |> View.Expert.describe in
  let retained = List.hd_exn wrapper.children |> View.Expert.describe in
  assert (Option.equal Key.equal retained.key (Some (Key.of_string_exn "prefix")));
  print_endline "frame add/update/remove preserves the input and user slot keys";
  [%expect {| frame add/update/remove preserves the input and user slot keys |}]
;;
