open Core
module S = Gpuio.Chart_style
module W = Gpuio_protocol.Chart_style_wire

let%expect_test "style has an independent paired byte fixture and validated defaults" =
  let wire : W.t =
    { version = -8L
    ; palette = [ 1L; 2L ]
    ; axis_color = 3L
    ; grid_color = 4L
    ; label_color = 5L
    ; selection_color = 6L
    ; gradient_end = Some 7L
    ; stroke_width = 2.
    ; point_radius = 3.
    ; bar_radius = 4.
    ; area_opacity = 0.5
    ; node_labels = []
    ; pie_labels = []
    ; pie_label_line_color = None
    ; x_axis =
        Gpuio.Chart_axis.Expert.to_wire
          Gpuio.Chart_axis.default
          ~theme:Gpuio.Theme.default
        |> Or_error.ok_exn
    ; y_axis =
        Gpuio.Chart_axis.Expert.to_wire
          Gpuio.Chart_axis.default
          ~theme:Gpuio.Theme.default
        |> Or_error.ok_exn
    ; grid =
        Gpuio.Chart_grid.Expert.to_wire
          Gpuio.Chart_grid.default
          ~theme:Gpuio.Theme.default
        |> Or_error.ok_exn
    ; inspection =
        Gpuio.Chart_inspection.Expert.to_wire
          Gpuio.Chart_inspection.default
          ~theme:Gpuio.Theme.default
        |> Or_error.ok_exn
    ; appearance = Gpuio_protocol.Chart_appearance_wire.empty
    ; ordinal = None
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
    {| fff8020102030405060107000000000000004000000000000008400000000000001040000000000000e03f000101010000000000008071400000000000002040000000000000204000000000000018400000000000002840000000000000314000000000000000000000000000000000000000f03f00000001010000000000003040000000000000000000000000000101000000000000000000000000002640000000000000f03f00000101000000000000000000000000002640000000000000f03f0000000000000000000000f03f00000000 |}]
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

let%expect_test
    "ordinal mapping uses namespaced identity and resolves tokens in the style theme"
  =
  let ok = Or_error.ok_exn in
  let series n = S.Key.series (Gpuio.Chart_data.Series_id.of_int64 n |> ok) in
  let slice n = S.Key.slice (Gpuio.Chart_data.Datum_id.of_int64 n |> ok) in
  let node n = S.Key.node (Gpuio.Chart_data.Node_id.of_int64 n |> ok) in
  let a = Gpuio.Color.token_exn "first" in
  let b = Gpuio.Color.token_exn "second" in
  let unknown = Gpuio.Color.token_exn "unknown" in
  let domain = [ series 9L; slice 9L; node 9L; S.Key.rising; S.Key.falling ] in
  let ordinal = S.Ordinal.create ~domain ~range:[ a; b ] ~unknown () |> ok in
  List.iteri domain ~f:(fun i key ->
    assert (
      Option.equal
        Gpuio.Color.equal
        (S.Ordinal.find ordinal key)
        (Some (if i % 2 = 0 then a else b))));
  assert (
    Option.equal Gpuio.Color.equal (S.Ordinal.find ordinal (series 3L)) (Some unknown));
  let without_unknown = S.Ordinal.create ~domain ~range:[ a; b ] () |> ok in
  assert (Option.is_none (S.Ordinal.find without_unknown (series 3L)));
  assert (Result.is_error (S.create ~ordinal ()));
  let rgba n = Gpuio.Color.rgba ~red:0 ~green:0 ~blue:0 ~alpha:n |> ok in
  let theme =
    Gpuio.Theme.create [ "first", rgba 10; "second", rgba 20; "unknown", rgba 30 ] |> ok
  in
  let wire = S.create ~ordinal ~theme () |> ok |> S.Expert.to_wire in
  let resolved = Option.value_exn wire.ordinal in
  assert (W.valid_ordinal resolved);
  print_s [%sexp (resolved : W.Ordinal.t)];
  assert (
    Result.is_error (S.Ordinal.create ~domain:[ series 1L; series 1L ] ~range:[ a ] ()));
  assert (Result.is_error (S.Ordinal.create ~domain ~range:[] ()));
  assert (
    Result.is_error (S.Ordinal.create ~domain ~range:(List.init 33 ~f:(fun _ -> a)) ()));
  let maximum = List.init 1024 ~f:(fun i -> series (Int64.of_int (i + 1))) in
  assert (Result.is_ok (S.Ordinal.create ~domain:maximum ~range:[ a ] ()));
  assert (Result.is_error (S.Ordinal.create ~domain:(slice 1L :: maximum) ~range:[ a ] ()));
  [%expect
    {|
    ((domain ((Series 9) (Slice 9) (Node 9) Rising Falling)) (range (10 20))
     (unknown (30)))
    |}]
;;

let%expect_test "ordinal style bytes pair with the independent native fixture" =
  let wire : W.t =
    { version = -8L
    ; palette = [ 1L; 2L ]
    ; axis_color = 3L
    ; grid_color = 4L
    ; label_color = 5L
    ; selection_color = 6L
    ; gradient_end = Some 7L
    ; stroke_width = 2.
    ; point_radius = 3.
    ; bar_radius = 4.
    ; area_opacity = 0.5
    ; node_labels = []
    ; pie_labels = []
    ; pie_label_line_color = None
    ; x_axis =
        Gpuio.Chart_axis.Expert.to_wire
          Gpuio.Chart_axis.default
          ~theme:Gpuio.Theme.default
        |> Or_error.ok_exn
    ; y_axis =
        Gpuio.Chart_axis.Expert.to_wire
          Gpuio.Chart_axis.default
          ~theme:Gpuio.Theme.default
        |> Or_error.ok_exn
    ; grid =
        Gpuio.Chart_grid.Expert.to_wire
          Gpuio.Chart_grid.default
          ~theme:Gpuio.Theme.default
        |> Or_error.ok_exn
    ; inspection =
        Gpuio.Chart_inspection.Expert.to_wire
          Gpuio.Chart_inspection.default
          ~theme:Gpuio.Theme.default
        |> Or_error.ok_exn
    ; appearance = Gpuio_protocol.Chart_appearance_wire.empty
    ; ordinal =
        Some
          { domain = [ Series 9L; Slice 9L; Node 9L; Rising; Falling ]
          ; range = [ 10L; 20L ]
          ; unknown = Some 30L
          }
    }
  in
  assert (Result.is_ok (S.Expert.of_wire wire));
  let bytes = Bin_prot.Utils.bin_dump W.bin_writer_t wire |> Bigstring.to_string in
  print_endline
    (String.to_list bytes
     |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
     |> String.concat);
  [%expect
    {| fff8020102030405060107000000000000004000000000000008400000000000001040000000000000e03f01050009010902090304020a14011e0101010000000000008071400000000000002040000000000000204000000000000018400000000000002840000000000000314000000000000000000000000000000000000000f03f00000001010000000000003040000000000000000000000000000101000000000000000000000000002640000000000000f03f00000101000000000000000000000000002640000000000000f03f0000000000000000000000f03f00000000 |}]
;;
