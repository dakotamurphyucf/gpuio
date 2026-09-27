open Core
module S = Gpuio.Chart_style
module W = Gpuio_protocol.Chart_style_wire

let%expect_test "style has an independent paired byte fixture and validated defaults" =
  let wire : W.t =
    { palette = [ 1L; 2L ]
    ; axis_color = 3L
    ; grid_color = 4L
    ; label_color = 5L
    ; selection_color = 6L
    ; gradient_end = Some 7L
    ; stroke_width = 2.
    ; point_radius = 3.
    ; bar_radius = 4.
    ; area_opacity = 0.5
    }
  in
  let style = S.Expert.of_wire wire |> Or_error.ok_exn in
  assert (W.equal wire (S.Expert.to_wire style));
  assert (W.valid (S.Expert.to_wire S.default));
  let bytes = Bin_prot.Utils.bin_dump W.bin_writer_t wire |> Bigstring.to_string in
  print_endline
    (String.to_list bytes
     |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
     |> String.concat);
  [%expect
    {| 020102030405060107000000000000004000000000000008400000000000001040000000000000e03f |}]
;;

let%expect_test "theme resolution and style bounds cannot be bypassed" =
  let theme =
    Gpuio.Theme.create [ "chart.accent", Gpuio.Color.rgb_exn 0xabcdef ] |> Or_error.ok_exn
  in
  let style =
    S.create
      ~theme
      ~palette:[ Gpuio.Color.token_exn "chart.accent" ]
      ~gradient_end:(Gpuio.Color.token_exn "chart.accent")
      ()
    |> Or_error.ok_exn
  in
  let wire = S.Expert.to_wire style in
  print_s [%sexp (wire.palette : int64 list), (wire.gradient_end : int64 option)];
  assert (Result.is_error (S.create ~palette:[] ()));
  assert (
    Result.is_error
      (S.create ~palette:(List.init 33 ~f:(fun _ -> Gpuio.Color.rgb_exn 0)) ()));
  assert (
    Result.is_error (S.create ~theme ~label_color:(Gpuio.Color.token_exn "missing") ()));
  List.iter [ Float.nan; Float.infinity; -1.; Float.neg_infinity ] ~f:(fun n ->
    assert (Result.is_error (S.create ~stroke_width:n ()));
    assert (Result.is_error (S.create ~point_radius:n ()));
    assert (Result.is_error (S.create ~bar_radius:n ()));
    assert (Result.is_error (S.create ~area_opacity:n ())));
  List.iter
    [ { wire with palette = [ -1L ] }
    ; { wire with gradient_end = Some 0x1_0000_0000L }
    ; { wire with stroke_width = 8.01 }
    ; { wire with point_radius = 12.01 }
    ; { wire with bar_radius = 32.01 }
    ; { wire with area_opacity = 1.01 }
    ]
    ~f:(fun wire -> assert (Result.is_error (S.Expert.of_wire wire)));
  [%expect {| ((2882400255) (2882400255)) |}]
;;
