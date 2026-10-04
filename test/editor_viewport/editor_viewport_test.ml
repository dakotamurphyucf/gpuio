open Core
module V = Gpuio.Editor_viewport
module W = Gpuio_protocol.Wire
module VW = Gpuio_protocol.Editor_viewport_wire

let ok = Or_error.ok_exn

let geometry : VW.t =
  { offset = { x = 12.5; y = 40. }
  ; width = 200.
  ; height = 100.
  ; line_height = 20.
  ; first_buffer_line = 2L
  ; buffer_line_limit = 7L
  }
;;

let%expect_test "viewport geometry is bounded and distinguishes logical lines" =
  List.iter
    [ -1.; Float.nan; Float.infinity; 1e9 +. 1. ]
    ~f:(fun value ->
      assert (Result.is_error (V.Offset.create ~x:value ~y:0.));
      assert (Result.is_error (V.Offset.create ~x:0. ~y:value)));
  assert (Result.is_ok (V.Offset.create ~x:1e9 ~y:1e9));
  let value = V.Expert.of_wire geometry |> ok in
  print_s
    [%sexp
      (( V.Offset.x (V.offset value)
       , V.Offset.y (V.offset value)
       , V.first_buffer_line value
       , V.buffer_line_limit value )
       : float * float * int * int)];
  List.iter
    [ { geometry with line_height = 0. }
    ; { geometry with first_buffer_line = 8L }
    ; { geometry with buffer_line_limit = 262_146L }
    ; { geometry with width = Float.nan }
    ]
    ~f:(fun value -> assert (Result.is_error (V.Expert.of_wire value)));
  [%expect {| (12.5 40 2 7) |}]
;;

let%expect_test
    "independent viewport requests and event envelopes reject malformed observations"
  =
  Eio_main.run (fun env ->
    let load name =
      let hex = Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / name) |> String.strip in
      String.init
        (String.length hex / 2)
        ~f:(fun i ->
          Char.of_int_exn (Int.of_string ("0x" ^ String.sub hex ~pos:(i * 2) ~len:2)))
    in
    let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
    let node = Gpuio_protocol.Node_id.create ~slot:1L ~generation:2L |> ok in
    let request number command =
      W.Message.Editor_command (number, window, node, command)
    in
    assert (
      String.equal
        (W.Message.encode (request 7L Read_viewport) |> ok)
        (load "editor-viewport-read.hex"));
    assert (
      String.equal
        (W.Message.encode (request 8L (Scroll_viewport geometry.offset)) |> ok)
        (load "editor-viewport-scroll.hex"));
    let event number result = W.Event.Editor_result (number, window, node, result) in
    let events =
      [ event 7L (Viewport None)
      ; event 8L (Viewport (Some geometry))
      ; event 9L Viewport_scroll_accepted
      ]
    in
    let encode events =
      Bin_prot.Utils.bin_dump [%bin_writer: W.Event.t list] events |> Bigstring.to_string
    in
    let bytes = load "editor-viewport-events.hex" in
    assert (String.equal (encode events) bytes);
    assert (List.equal W.Event.equal events (W.Event.decode bytes |> ok));
    for n = 0 to String.length bytes - 1 do
      assert (Result.is_error (W.Event.decode (String.prefix bytes n)))
    done;
    assert (Result.is_error (W.Event.decode (bytes ^ "\000")));
    List.iter
      [ { geometry with line_height = 0. }
      ; { geometry with offset = { x = Float.nan; y = 0. } }
      ; { geometry with buffer_line_limit = 1L }
      ]
      ~f:(fun value ->
        assert (
          Result.is_error (W.Event.decode (encode [ event 8L (Viewport (Some value)) ])))));
  print_endline "commands 8/9 and replies 3/4; bounded metadata; strict envelope decode";
  [%expect {| commands 8/9 and replies 3/4; bounded metadata; strict envelope decode |}]
;;
