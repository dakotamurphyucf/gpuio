open Core
open Gpuio
module W = Gpuio_protocol.Wire
module I = Text_input

let ok = Or_error.ok_exn

let%expect_test "password hints require privacy and leave legacy config bytes unchanged" =
  List.iter
    I.Content_hint.[ Password; New_password ]
    ~f:(fun content_hint ->
      assert (
        Result.is_error
          (I.Config.create ~mode:Single_line ~label:"Draft" ~content_hint ()));
      assert (
        Result.is_error
          (I.Config.create
             ~mode:Multiline
             ~label:"Draft"
             ~privacy:(Password Hidden)
             ~content_hint
             ()));
      ignore
        (I.Config.create
           ~mode:Single_line
           ~label:"Draft"
           ~privacy:(Password Hidden)
           ~content_hint
           ()
         |> ok
         : I.Config.t));
  let plain = I.Config.create ~mode:Single_line ~label:"Draft" () |> ok in
  let hinted =
    I.Config.create ~mode:Single_line ~label:"Draft" ~content_hint:Email_address () |> ok
  in
  assert (
    W.Editor.Config.equal (I.Expert.config_to_wire plain) (I.Expert.config_to_wire hinted));
  print_endline
    "semantic hints preserve editor bytes; password semantics require explicit privacy";
  [%expect
    {| semantic hints preserve editor bytes; password semantics require explicit privacy |}]
;;

let%expect_test "content-hint protocol has independent paired bytes" =
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let node = Gpuio_protocol.Node_id.create ~slot:1L ~generation:2L |> ok in
  let hints =
    I.Content_hint.
      [ None; Some Email_address; Some Password; Some Cellular_imei; Some Given_name ]
  in
  let message =
    W.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          List.map hints ~f:(fun hint ->
            W.Op.Set_editor_content_hint
              (node, Option.map hint ~f:I.Content_hint.Expert.to_wire))
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
        (Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "input-content-hint-operation.hex")
         |> String.strip)));
  print_endline "content hint tag 75: absent, email, password, IMEI, given name";
  [%expect {| content hint tag 75: absent, email, password, IMEI, given name |}]
;;

let%expect_test "content hints change without replacing the editing owner" =
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let r = Reconciler.create window in
  let render content_hint =
    let config =
      I.Config.create ~mode:Single_line ~label:"Draft" ?content_hint () |> ok
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
    let update = Reconciler.prepare r ~theme:Theme.default (Some view) |> ok in
    let message = Reconciler.message update in
    Reconciler.accept r update |> ok;
    match message with
    | Some (W.Message.Apply tx) -> tx.operations
    | _ -> []
  in
  let first = render None in
  let node =
    List.find_map_exn first ~f:(function
      | W.Op.Create (id, Input, _, _) -> Some id
      | _ -> None)
  in
  List.iter
    I.Content_hint.[ Some Email_address; Some Url; None ]
    ~f:(fun hint ->
      assert (
        List.equal
          W.Op.equal
          (render hint)
          [ Set_editor_content_hint
              (node, Option.map hint ~f:I.Content_hint.Expert.to_wire)
          ]));
  assert (List.is_empty (render None));
  print_endline "one editor; hint-only updates; unchanged render silent";
  [%expect {| one editor; hint-only updates; unchanged render silent |}]
;;

let%expect_test "hint status messages agree across runtimes and reject malformed metadata"
  =
  Eio_main.run (fun env ->
    let load name =
      let hex = Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / name) |> String.strip in
      String.init
        (String.length hex / 2)
        ~f:(fun i ->
          Char.of_int_exn (Int.of_string ("0x" ^ String.sub hex ~pos:(i * 2) ~len:2)))
    in
    let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
    let node = Gpuio_protocol.Node_id.create ~slot:1L ~generation:2L |> ok in
    let request = W.Message.Editor_command (7L, window, node, Read_content_hint_status) in
    assert (
      String.equal
        (W.Message.encode request |> ok)
        (load "input-content-hint-request.hex"));
    let statuses =
      Gpuio_protocol.Input_content_hint_status_wire.
        [ Inactive None
        ; Inactive (Some Email_address)
        ; Exposed Email_address
        ; Unavailable (Email_address, Backend)
        ; Unavailable (Cellular_imei, Mapping)
        ; Unavailable (Url, Native_view)
        ]
    in
    let events =
      List.mapi statuses ~f:(fun i status ->
        W.Event.Editor_result
          (Int64.of_int (7 + i), window, node, Content_hint_status status))
    in
    let data = load "input-content-hint-events.hex" in
    assert (
      String.equal
        data
        (Bin_prot.Utils.bin_dump [%bin_writer: W.Event.t list] events
         |> Bigstring.to_string));
    assert (List.equal W.Event.equal events (W.Event.decode data |> ok));
    for length = 0 to String.length data - 1 do
      assert (Result.is_error (W.Event.decode (String.prefix data length)))
    done;
    assert (Result.is_error (W.Event.decode (data ^ "\000")));
    (* A single status event: prefix includes result tag 2. *)
    let prefix = "\001\012\007\000\001\001\002\002" in
    List.iter [ "\003"; "\000\002"; "\001\045"; "\002\020\003" ] ~f:(fun invalid ->
      assert (Result.is_error (W.Event.decode (prefix ^ invalid))));
    List.iter statuses ~f:(fun status ->
      print_s
        [%sexp (I.Content_hint.Status.Expert.of_wire status : I.Content_hint.Status.t)]));
  [%expect
    {|
    (Inactive ())
    (Inactive (Email_address))
    (Exposed Email_address)
    (Unavailable Email_address Backend)
    (Unavailable Cellular_imei Mapping)
    (Unavailable Url Native_view)
  |}]
;;
