open Core
open Gpuio
module P = Table_presentation
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn

let%expect_test "native presentation validates state and geometry boundaries" =
  List.iter
    [ Style.Property.Width (Length.px_exn 12.)
    ; Padding (Length.px_exn 3.)
    ; Opacity 0.
    ; Disabled false
    ; Pointer_events false
    ; Visibility Hidden
    ; Overflow_x Scroll
    ; Border_bottom_width 4.
    ; Line_height (Length.px_exn 50.)
    ]
    ~f:(fun property ->
      let style = Style.create_exn [ property ] in
      assert (Or_error.is_error (P.Header.create style));
      assert (Or_error.is_error (P.Row.create style)));
  List.iter [ Style.State.Focused; Selected ] ~f:(fun state ->
    let style =
      Style.with_state_exn Style.empty state [ Foreground (Color.rgb_exn 0xff0000) ]
    in
    assert (Or_error.is_ok (P.Row.create style));
    assert (Or_error.is_error (P.Header.create style)));
  List.iter [ Style.State.Checked; Indeterminate ] ~f:(fun state ->
    let style =
      Style.with_state_exn Style.empty state [ Foreground (Color.rgb_exn 0xff0000) ]
    in
    assert (Or_error.is_error (P.Row.create style)));
  let style =
    Style.create_exn
      [ Font_weight 600; Background (Background.solid (Color.rgb_exn 0x112233)) ]
  in
  assert (Or_error.is_ok (P.Row.create style) && Or_error.is_ok (P.Header.create style));
  print_endline
    "Native geometry/input policy protected; header and row state scopes explicit";
  [%expect
    {| Native geometry/input policy protected; header and row state scopes explicit |}]
;;

let%expect_test "header and row operations have paired independent fixtures" =
  let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok in
  let header =
    [ W.Style.Fields [ Font_weight 400L ]; State (2L, [ Foreground (Rgba 1L) ]) ]
  in
  let row =
    [ W.Style.Fields [ Foreground (Rgba 1L) ]; State (7L, [ Foreground (Rgba 2L) ]) ]
  in
  Eio_main.run (fun env ->
    List.iter
      [ "table-header-style.hex", W.Op.Set_table_header_style (node, header)
      ; "table-header-style-clear.hex", W.Op.Set_table_header_style (node, [])
      ; "table-row-style.hex", W.Op.Set_table_row_style (node, row)
      ; "table-row-style-clear.hex", W.Op.Set_table_row_style (node, [])
      ]
      ~f:(fun (file, op) ->
        let bytes = Bin_prot.Utils.bin_dump W.Op.bin_writer_t op |> Bigstring.to_string in
        let hex =
          String.to_list bytes
          |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
          |> String.concat
        in
        assert (
          String.equal
            hex
            (Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / file) |> String.strip))));
  print_endline "Appended tags 114/115; explicit empty-style reset";
  [%expect {| Appended tags 114/115; explicit empty-style reset |}]
;;
