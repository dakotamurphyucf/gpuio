open Core
open Gpuio
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn

let%expect_test "editor menu carries editor keys through sibling reorders" =
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let r = Reconciler.create window in
  let render names =
    let children =
      List.map names ~f:(fun name ->
        View.text_input
          ~controller:(Key.of_string_exn name)
          ~config:(Text_input.Config.create ~mode:Single_line ~label:name () |> ok)
          ~on_event:(fun _ -> ())
          ()
        |> ok
        |> View.editor_menu
        |> ok)
    in
    let update =
      Reconciler.prepare r ~theme:Theme.default (Some (View.column children)) |> ok
    in
    let message = Reconciler.message update in
    Reconciler.accept r update |> ok;
    match message with
    | Some (W.Message.Apply tx) -> tx.operations
    | _ -> []
  in
  ignore (render [ "a"; "b" ] : W.Op.t list);
  let moved = render [ "b"; "a" ] in
  assert (not (List.is_empty moved));
  assert (
    List.for_all moved ~f:(function
      | W.Op.Splice _ -> true
      | _ -> false));
  print_endline "keyed reorder moves wrappers without recreating native inputs";
  [%expect {| keyed reorder moves wrappers without recreating native inputs |}]
;;

let%expect_test "bound context presentation has independent paired bytes" =
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let node = Gpuio_protocol.Node_id.create ~slot:1L ~generation:2L |> ok in
  let message =
    W.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Set_menu
              ( node
              , { presentation = Editor_context
                ; menus =
                    [ { label = "Edit"; disabled = false; items = [ Command "copy" ] } ]
                } )
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
      Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "editor-menu-operation.hex")
      |> String.strip
    in
    assert (String.equal hex expected));
  print_endline "menu presentation 4; existing Set_menu operation";
  [%expect {| menu presentation 4; existing Set_menu operation |}]
;;

let%expect_test "editor menu labels are validated and the wrapper requires an editor" =
  List.iter
    [ ""; "  "; "bad\000label"; "\255"; String.make 4097 'x'; "Édition" ]
    ~f:(fun label ->
      print_s [%sexp (Result.is_ok (Editor_menu.create ~copy:label ()) : bool)]);
  print_s [%sexp (Result.is_ok (View.editor_menu (View.text "not an editor")) : bool)];
  [%expect
    {|
    false
    false
    false
    false
    false
    true
    false
  |}]
;;

let%expect_test
    "editor menu label changes retain the input and install native scoped commands"
  =
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let r = Reconciler.create window in
  let render config =
    let input =
      View.text_input
        ~controller:(Key.of_string_exn "draft")
        ~config:(Text_input.Config.create ~mode:Single_line ~label:"Draft" () |> ok)
        ~initial_text:"seed"
        ~on_event:(fun _ -> ())
        ()
      |> ok
    in
    let view = View.editor_menu ~config input |> ok in
    let update = Reconciler.prepare r ~theme:Theme.default (Some view) |> ok in
    let message = Reconciler.message update in
    Reconciler.accept r update |> ok;
    match message with
    | Some (W.Message.Apply tx) -> tx.operations
    | _ -> []
  in
  let first = render Editor_menu.default in
  assert (
    List.exists first ~f:(function
      | W.Op.Set_menu (_, { presentation = Editor_context; _ }) -> true
      | _ -> false));
  let commands =
    List.concat_map first ~f:(function
      | W.Op.Set_commands (_, commands) -> commands
      | _ -> [])
  in
  assert (List.length commands = 4);
  assert (
    List.for_all commands ~f:(fun command ->
      match command.W.Command.target with
      | Native _ -> true
      | Callback -> false));
  let next = render (Editor_menu.create ~copy:"Copier" () |> ok) in
  assert (
    List.for_all next ~f:(function
      | W.Op.Set_commands _ -> true
      | _ -> false));
  assert (not (List.is_empty next));
  let disabled = render (Editor_menu.create ~enabled:false ~copy:"Copier" () |> ok) in
  assert (
    List.for_all disabled ~f:(function
      | W.Op.Set_menu _ -> true
      | _ -> false));
  assert (not (List.is_empty disabled));
  print_endline "four scoped native actions; label update retains editor and menu";
  [%expect {| four scoped native actions; label update retains editor and menu |}]
;;
