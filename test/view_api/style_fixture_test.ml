open Core
open Gpuio_protocol

let fields : Wire.Field.t list =
  [ Display 2L
  ; Visibility 0L
  ; Direction 1L
  ; Wrap 1L
  ; Grow 1.
  ; Shrink 0.
  ; Basis Auto
  ; Align_items 4L
  ; Align_self 6L
  ; Align_content 7L
  ; Justify_content 6L
  ; Row_gap (Px 8.)
  ; Column_gap (Percent 2.)
  ; Grid_columns 3L
  ; Grid_rows 2L
  ; Grid_column_minimum 1L
  ; Grid_row_minimum 2L
  ; Width (Percent 100.)
  ; Height (Px 480.)
  ; Min_width (Px 100.)
  ; Min_height (Px 20.)
  ; Max_width (Px 800.)
  ; Max_height Auto
  ; Padding_top (Px 1.)
  ; Padding_right (Px 2.)
  ; Padding_bottom (Px 3.)
  ; Padding_left (Percent 4.)
  ; Margin_top (Px (-1.))
  ; Margin_right Auto
  ; Margin_bottom (Px (-3.))
  ; Margin_left (Percent 4.)
  ; Position 1L
  ; Top (Px 10.)
  ; Right Auto
  ; Bottom (Percent 20.)
  ; Left (Px (-5.))
  ; Background (Linear_gradient (45., Rgba 0xff0000ffL, 0., Rgba 0x0000ffffL, 1.))
  ; Foreground (Rgba 0x112233ffL)
  ; Opacity 0.75
  ; Border_top_width 1.
  ; Border_right_width 2.
  ; Border_bottom_width 3.
  ; Border_left_width 4.
  ; Top_left_radius 5.
  ; Top_right_radius 6.
  ; Bottom_left_radius 7.
  ; Bottom_right_radius 8.
  ; Border_color (Rgba 0xffffffffL)
  ; Shadows
      [ { color = Rgba 0x00000088L
        ; offset_x = -2.
        ; offset_y = 3.
        ; blur = 4.
        ; spread = -1.
        ; inset = true
        }
      ]
  ; Font_size 16.
  ; Font_family "λ family"
  ; Font_weight 600L
  ; Text_align 1L
  ; Line_height (Percent 150.)
  ; White_space 1L
  ; Text_overflow 1L
  ; Line_clamp 3L
  ; Text_decoration 3L
  ; Overflow_x 3L
  ; Overflow_y 2L
  ; Cursor 8L
  ; Pointer_events false
  ; User_select true
  ; Selection_color (Rgba 0xabcdef80L)
  ; Accessible_name "Copy response"
  ]
;;

let request =
  let id = Node_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  let window = Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  Wire.Message.Apply
    { window
    ; base = 0L
    ; revision = 1L
    ; operations =
        [ Create (id, Container, "", None)
        ; Set_style
            (id, [ Fields fields; State (1L, [ Background (Solid (Rgba 0xffffffffL)) ]) ])
        ; Set_root (Some id)
        ]
    }
;;

let%expect_test "all extended style tags have independent cross-language byte agreement" =
  Eio_main.run (fun env ->
    let hex =
      Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / "style-v1.hex") |> String.strip
    in
    let expected =
      String.init
        (String.length hex / 2)
        ~f:(fun i ->
          Char.of_int_exn (Int.of_string ("0x" ^ String.sub hex ~pos:(i * 2) ~len:2)))
    in
    let actual = Wire.Message.encode request |> Or_error.ok_exn in
    assert (String.equal actual expected);
    let buffer = Bigstring.of_string expected in
    let pos_ref = ref 0 in
    assert (Wire.Message.equal (Wire.Message.bin_read_t buffer ~pos_ref) request);
    assert (!pos_ref = String.length expected);
    print_endline "STYLE_CODEC_PASS");
  [%expect {| STYLE_CODEC_PASS |}]
;;
