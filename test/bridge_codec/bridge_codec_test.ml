open Core
open Gpuio_protocol

let window = Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn
let root = Node_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn
let child = Node_id.create ~slot:1L ~generation:2L |> Or_error.ok_exn
let handler = Handler_id.create ~slot:2L ~generation:3L |> Or_error.ok_exn

let request =
  Wire.Message.Apply
    { window
    ; base = 127L
    ; revision = 128L
    ; operations =
        [ Create (root, Container, "", None)
        ; Create (child, Button, "λ 🦀", Some handler)
        ; Set_style
            ( root
            , [ Width (Percent 100.)
              ; Padding 12.
              ; Background (Rgba 0x112233ffL)
              ; Hover_background (Token 1L)
              ; Opacity 0.75
              ] )
        ; Splice (root, 0L, 0L, [ child ])
        ; Set_root (Some root)
        ]
    }
;;

let events : Wire.Event.t list =
  [ Welcome (Wire.version, Wire.capabilities)
  ; Opened (1L, window)
  ; Accepted (window, 128L)
  ; Rendered (window, 128L)
  ; Press (window, child, handler, 128L)
  ; Rejected (window, 129L, Invalid_tree)
  ; Frame_requested (9L, window, 128L)
  ; Failed (10L, Busy)
  ; Overloaded window
  ; Closed (11L, window)
  ; Stopped
  ]
;;

let load_hex path =
  let hex = Eio.Path.load path |> String.strip in
  String.init
    (String.length hex / 2)
    ~f:(fun i ->
      Char.of_int_exn (Int.of_string ("0x" ^ String.sub hex ~pos:(i * 2) ~len:2)))
;;

let%expect_test "independently constructed OCaml and Rust fixtures agree" =
  Eio_main.run (fun env ->
    let cwd = Eio.Stdenv.cwd env in
    let request_bytes = load_hex Eio.Path.(cwd / "bridge-v1-request.hex") in
    let event_bytes = load_hex Eio.Path.(cwd / "bridge-v1-events.hex") in
    let encoded = Wire.Message.encode request |> Or_error.ok_exn in
    Expect_test_helpers_core.require_equal [%here] (module String) request_bytes encoded;
    let pos_ref = ref 0 in
    let decoded = Wire.Message.bin_read_t (Bigstring.of_string request_bytes) ~pos_ref in
    Expect_test_helpers_core.require [%here] (Wire.Message.equal request decoded);
    Expect_test_helpers_core.require [%here] (!pos_ref = String.length request_bytes);
    let decoded_events = Wire.Event.decode event_bytes |> Or_error.ok_exn in
    if not (List.equal Wire.Event.equal decoded_events events)
    then print_s [%sexp (decoded_events : Wire.Event.t list)];
    Expect_test_helpers_core.require
      [%here]
      (List.equal Wire.Event.equal decoded_events events);
    let encoded_events =
      Bin_prot.Utils.bin_dump [%bin_writer: Wire.Event.t list] events
      |> Bigstring.to_string
    in
    Expect_test_helpers_core.require_equal
      [%here]
      (module String)
      event_bytes
      encoded_events;
    for length = 0 to String.length event_bytes - 1 do
      Expect_test_helpers_core.require
        [%here]
        (Or_error.is_error (Wire.Event.decode (String.prefix event_bytes length)))
    done;
    print_s [%sexp ("BRIDGE_CODEC_PASS" : string)]);
  [%expect {| BRIDGE_CODEC_PASS |}]
;;

let%expect_test "invalid generations, unbounded envelopes and trailing bytes fail" =
  print_s [%sexp (Or_error.is_error (Node_id.create ~slot:0L ~generation:0L) : bool)];
  List.iter [ "\001\001\001\000\000"; "\254\001\001"; "\001\009\000" ] ~f:(fun bytes ->
    print_s [%sexp (Or_error.is_error (Wire.Event.decode bytes) : bool)]);
  [%expect
    {| 
    true
    true
    true
    true |}]
;;
