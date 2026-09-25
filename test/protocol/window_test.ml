open Core
open Gpuio_protocol

let id = Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn

let hex bytes =
  String.to_list bytes
  |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
  |> String.concat
;;

let%expect_test "window command and configured-open independent fixtures" =
  let messages : Wire.Message.t list =
    [ Window_command (7L, id, Observe)
    ; Window_command (7L, id, Set_title "x")
    ; Window_command (7L, id, Resize (640., 400.))
    ; Window_command (7L, id, Activate)
    ; Window_command (7L, id, Zoom)
    ; Window_command (7L, id, Toggle_fullscreen)
    ; Window_command (7L, id, Set_edited true)
    ; Open_configured
        ( 7L
        , id
        , { title = "x"
          ; width = 640.
          ; height = 400.
          ; focus = false
          ; chrome = Hidden
          ; resizable = false
          } )
    ]
  in
  List.iter messages ~f:(fun message ->
    print_endline (hex (Wire.Message.encode message |> Or_error.ok_exn)));
  [%expect
    {| 
0b07000100
0b070001010178
0b0700010200000000000084400000000000007940
0b07000103
0b07000104
0b07000105
0b0700010601
0c070001017800000000000084400000000000007940000100
|}]
;;

let%expect_test "invalid outgoing window data is rejected" =
  List.iter
    [ Wire.Window.Command.Resize (Float.nan, 400.)
    ; Resize (0., 400.)
    ; Set_title "bad\000title"
    ]
    ~f:(fun command ->
      print_s
        [%sexp
          (Result.is_error (Wire.Message.encode (Window_command (7L, id, command)))
           : bool)]);
  [%expect
    {|
    true
    true
    true
    |}]
;;

let%expect_test "lifecycle events decode the independent native fixture" =
  (* Three events: close generation 1/slot 0, quit and reopen. *)
  let events = Wire.Event.decode "\003\031\000\001\032\033" |> Or_error.ok_exn in
  print_s [%sexp (events : Wire.Event.t list)];
  [%expect
    {| ((Close_requested ((slot 0) (generation 1))) Quit_requested Reopen_requested) |}]
;;
