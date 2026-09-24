open Core
open Gpuio_protocol

let window = Window_id.create ~slot:3L ~generation:2L |> Or_error.ok_exn
let node = Node_id.create ~slot:4L ~generation:3L |> Or_error.ok_exn
let handler = Handler_id.create ~slot:5L ~generation:4L |> Or_error.ok_exn

let request =
  Wire.Message.Apply
    { window
    ; base = 127L
    ; revision = 128L
    ; operations =
        [ Create (node, Combobox, "é", Some handler)
        ; Set_combobox_filter (node, Substring)
        ; Set_combobox_filter (node, Unfiltered)
        ]
    }
;;

let snapshot : Wire.Editor.Snapshot.t =
  { revision = 129L
  ; text = "é界"
  ; selection = { anchor = 5L; head = 2L }
  ; composition = None
  ; focused = true
  }
;;

let events =
  [ Wire.Event.Combobox_selected (window, node, handler, 128L, "深い", snapshot) ]
;;

let%expect_test
    "editable choice codec preserves exact query revision and UTF-8 byte selection"
  =
  Eio_main.run (fun env ->
    let load name =
      let hex = Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / name) |> String.strip in
      String.init
        (String.length hex / 2)
        ~f:(fun i ->
          Char.of_int_exn (Int.of_string ("0x" ^ String.sub hex ~pos:(i * 2) ~len:2)))
    in
    let request_bytes = load "combobox-v1-request.hex"
    and event_bytes = load "combobox-v1-events.hex" in
    assert (String.equal request_bytes (Wire.Message.encode request |> Or_error.ok_exn));
    assert (
      String.equal
        event_bytes
        (Bin_prot.Utils.bin_dump [%bin_writer: Wire.Event.t list] events
         |> Bigstring.to_string));
    assert (
      List.equal Wire.Event.equal events (Wire.Event.decode event_bytes |> Or_error.ok_exn));
    for length = 0 to String.length event_bytes - 1 do
      assert (Result.is_error (Wire.Event.decode (String.prefix event_bytes length)))
    done;
    print_endline "COMBOBOX_CODEC_PASS");
  [%expect {| COMBOBOX_CODEC_PASS |}]
;;

let%expect_test "editable selection decoder rejects composing and multiline snapshots" =
  let check snapshot =
    let bytes =
      Bin_prot.Utils.bin_dump
        [%bin_writer: Wire.Event.t list]
        [ Combobox_selected (window, node, handler, 128L, "deep", snapshot) ]
      |> Bigstring.to_string
    in
    print_s [%sexp (Wire.Event.decode bytes |> Result.is_error : bool)]
  in
  check { snapshot with composition = Some { anchor = 0L; head = 2L } };
  check { snapshot with text = "two\nlines" };
  check { snapshot with selection = { anchor = 1L; head = 2L } };
  [%expect
    {|
    true
    true
    true
    |}]
;;
