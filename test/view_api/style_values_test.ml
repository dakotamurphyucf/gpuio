open Core
open Gpuio
open Gpuio_protocol

let%expect_test "native alignment aliases retain distinct flex-relative encodings" =
  let properties : Style.Property.t list =
    [ Align_items Flex_start
    ; Align_items Flex_end
    ; Align_self Flex_start
    ; Align_self Flex_end
    ; Align_content Flex_start
    ; Align_content Flex_end
    ; Justify_content Start
    ; Justify_content End
    ; Direction Row
    ; Direction Column
    ; Wrap No_wrap
    ; Wrap Wrap
    ; Wrap Wrap_reverse
    ; Position Relative
    ; Position Absolute
    ; Text_align Left
    ; Text_align Center
    ; Text_align Right
    ; User_select false
    ; User_select true
    ]
  in
  List.iter properties ~f:(fun property ->
    match
      Style.create_exn [ property ]
      |> Style.Expert.to_wire ~theme:Theme.default
      |> Or_error.ok_exn
    with
    | [ Wire.Style.Fields [ field ] ] ->
      let bytes =
        Bin_prot.Utils.bin_dump Wire.Field.bin_writer_t field |> Bigstring.to_string
      in
      print_endline
        (String.concat_map bytes ~f:(fun ch -> sprintf "%02x" (Char.to_int ch)))
    | _ -> assert false);
  [%expect
    {|
    0702
    0703
    0802
    0803
    0902
    0903
    0a00
    0a01
    0200
    0201
    0300
    0301
    0302
    1f00
    1f01
    3400
    3401
    3402
    3e00
    3e01
    |}]
;;

let%expect_test "unset removes only its layer and a later declaration can replace it" =
  let original =
    Style.create_exn [ Align_content Center ]
    |> fun t -> Style.with_state_exn t Hovered [ Align_content Flex_end ]
  in
  let remove_hover = Style.unset Style.empty ~state:Hovered Align_content in
  let remove_base = Style.unset Style.empty Align_content in
  let show styles =
    Style.merge styles
    |> Style.Expert.to_wire ~theme:Theme.default
    |> Or_error.ok_exn
    |> List.map ~f:(fun style ->
      let state, fields =
        match style with
        | Wire.Style.Fields fields -> "base", fields
        | State (2L, fields) -> "hover", fields
        | _ -> assert false
      in
      let value =
        match fields with
        | [] -> "absent"
        | [ Wire.Field.Align_content value ] -> Int64.to_string value
        | _ -> assert false
      in
      state ^ ":" ^ value)
    |> String.concat ~sep:" "
    |> print_endline
  in
  show [ original; remove_hover ];
  show [ original; remove_base ];
  show [ original; remove_base; remove_hover ];
  show [ original; remove_base; Style.create_exn [ Align_content Stretch ] ];
  [%expect
    {|
    base:4 hover:absent
    base:absent hover:3
    base:absent hover:absent
    base:5 hover:3
    |}]
;;

let%expect_test "finite grid and text choices have stable native field bytes" =
  let properties : Style.Property.t list =
    [ Grid_column_minimum Zero
    ; Grid_column_minimum Min_content
    ; Grid_column_minimum Max_content
    ; Grid_row_minimum Zero
    ; Grid_row_minimum Min_content
    ; Grid_row_minimum Max_content
    ; White_space Normal
    ; White_space No_wrap
    ; Text_decoration None
    ; Text_decoration Underline
    ; Text_decoration Strikethrough
    ; Text_decoration Underline_and_strikethrough
    ]
  in
  List.iter properties ~f:(fun property ->
    let fields =
      Style.create_exn [ property ]
      |> Style.Expert.to_wire ~theme:Theme.default
      |> Or_error.ok_exn
    in
    match fields with
    | [ Wire.Style.Fields [ field ] ] ->
      let bytes =
        Bin_prot.Utils.bin_dump Wire.Field.bin_writer_t field |> Bigstring.to_string
      in
      print_endline
        (String.concat_map bytes ~f:(fun ch -> sprintf "%02x" (Char.to_int ch)))
    | _ -> assert false);
  [%expect
    {|
    0f00
    0f01
    0f02
    1000
    1001
    1002
    3600
    3601
    3900
    3901
    3902
    3903
    |}]
;;

let cursors : Style.Cursor.t list =
  [ Arrow
  ; Ibeam
  ; Pointer
  ; Crosshair
  ; Move
  ; Not_allowed
  ; Resize_horizontal
  ; Resize_vertical
  ; Grab
  ; Grabbing
  ; Ibeam_vertical
  ; Resize_column
  ; Resize_row
  ; Resize_nw_se
  ; Resize_ne_sw
  ; Resize_left
  ; Resize_right
  ; Resize_up
  ; Resize_down
  ; Alias
  ; Copy
  ; Context_menu
  ]
;;

let%expect_test "public cursor and overflow values agree with independent native bytes" =
  let style property =
    Style.create_exn [ property ]
    |> Style.Expert.to_wire ~theme:Theme.default
    |> Or_error.ok_exn
  in
  let styles =
    List.concat_map cursors ~f:(fun cursor -> style (Cursor cursor))
    @ List.concat_map
        [ Style.Text_overflow.Clip; Ellipsis; Ellipsis_start ]
        ~f:(fun overflow -> style (Text_overflow overflow))
  in
  let node = Node_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  let request =
    Wire.Message.Apply
      { window = Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Create (node, Text, "long/path/to/main.ml", None)
          ; Set_style (node, styles)
          ; Set_root (Some node)
          ]
      }
  in
  Eio_main.run (fun env ->
    let hex =
      Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / "style-values-v1.hex") |> String.strip
    in
    let expected =
      String.init
        (String.length hex / 2)
        ~f:(fun i ->
          Char.of_int_exn (Int.of_string ("0x" ^ String.sub hex ~pos:(i * 2) ~len:2)))
    in
    let actual = Wire.Message.encode request |> Or_error.ok_exn in
    assert (String.equal actual expected));
  assert (Int64.equal (Int64.bit_and Wire.capabilities 17592186044416L) 17592186044416L);
  print_s [%sexp (List.length cursors : int), (List.length styles : int)];
  [%expect {| (22 25) |}]
;;

let%expect_test "pointer occlusion is explicit, typed and base-only with independent tags"
  =
  List.iter [ Style.Pointer_occlusion.None; Pointer; Pointer_and_scroll ] ~f:(fun mode ->
    let fields =
      Style.create_exn [ Pointer_occlusion mode ]
      |> Style.Expert.to_wire ~theme:Theme.default
      |> Or_error.ok_exn
    in
    match fields with
    | [ Wire.Style.Fields [ field ] ] ->
      let bytes =
        Bin_prot.Utils.bin_dump Wire.Field.bin_writer_t field |> Bigstring.to_string
      in
      print_endline
        (String.concat_map bytes ~f:(fun ch -> sprintf "%02x" (Char.to_int ch)))
    | _ -> assert false);
  print_s
    [%sexp
      (Or_error.is_error
         (Style.with_state Style.empty Hovered [ Pointer_occlusion Pointer ])
       : bool)];
  assert (Int64.equal (Int64.bit_and Wire.capabilities 70368744177664L) 70368744177664L);
  [%expect
    {|
    4200
    4201
    4202
    true
    |}]
;;
