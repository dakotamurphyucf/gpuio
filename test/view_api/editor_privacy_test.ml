open Core
open Gpuio
open Gpuio_protocol
module I = Text_input
module W = Wire

let ok = Or_error.ok_exn

let%expect_test "password modes reject multiline and preserve legacy editor bytes" =
  List.iter
    I.Privacy.[ Plain; Password Hidden; Password Revealed ]
    ~f:(fun privacy ->
      let config =
        I.Config.create ~privacy ~mode:Single_line ~label:"Password" () |> ok
      in
      assert (I.Privacy.equal (I.Config.privacy config) privacy);
      print_s
        [%sexp
          (Result.is_ok (I.Config.create ~privacy ~mode:Multiline ~label:"Draft" ())
           : bool)];
      let baseline = I.Config.create ~mode:Single_line ~label:"Password" () |> ok in
      assert (
        W.Editor.Config.equal
          (I.Expert.config_to_wire config)
          (I.Expert.config_to_wire baseline)));
  [%expect
    {|
    true
    false
    false
  |}]
;;

let%expect_test "privacy operation has independent paired bytes" =
  let window = Window_id.create ~slot:0L ~generation:1L |> ok in
  let node = Node_id.create ~slot:1L ~generation:2L |> ok in
  let message =
    W.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          List.map
            I.Privacy.[ Plain; Password Hidden; Password Revealed ]
            ~f:(fun privacy ->
              W.Op.Set_editor_privacy (node, I.Expert.privacy_to_wire privacy))
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
      Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "editor-privacy-operation.hex")
      |> String.strip
    in
    assert (String.equal hex expected));
  print_endline "privacy tag 73: plain, hidden, revealed";
  [%expect {| privacy tag 73: plain, hidden, revealed |}]
;;

let%expect_test "reveal updates retain editor identity and do not replace its draft" =
  let window = Window_id.create ~slot:0L ~generation:1L |> ok in
  let r = Reconciler.create window in
  let controller = Key.of_string "password" |> ok in
  let render privacy =
    let config = I.Config.create ~privacy ~mode:Single_line ~label:"Password" () |> ok in
    let view =
      View.text_input ~controller ~config ~initial_text:"seed" ~on_event:(fun _ -> ()) ()
      |> ok
    in
    let update = Reconciler.prepare r ~theme:Theme.default (Some view) |> ok in
    let message = Reconciler.message update in
    Reconciler.accept r update |> ok;
    match message with
    | Some (W.Message.Apply tx) -> tx.operations
    | Some _ | None -> []
  in
  let initial = render (Password Hidden) in
  let node =
    List.find_map_exn initial ~f:(function
      | W.Op.Create (id, Input, _, _) -> Some id
      | _ -> None)
  in
  assert (
    List.count initial ~f:(fun operation ->
      W.Op.equal operation (Set_editor_privacy (node, Password_hidden)))
    = 1);
  List.iter
    I.Privacy.[ Password Revealed; Password Hidden; Plain ]
    ~f:(fun privacy ->
      let operations = render privacy in
      assert (
        List.equal
          W.Op.equal
          operations
          [ Set_editor_privacy (node, I.Expert.privacy_to_wire privacy) ]));
  assert (List.is_empty (render Plain));
  print_endline "one retained editor, privacy-only updates, unchanged render silent";
  [%expect {| one retained editor, privacy-only updates, unchanged render silent |}]
;;
