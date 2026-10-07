open Core
module A = Gpuio.Chart_appearance
module W = Gpuio_protocol.Chart_appearance_wire
module C = Gpuio.Color
module B = Gpuio.Background

let ok = Or_error.ok_exn
let sid n = Gpuio.Chart_data.Series_id.of_int64 (Int64.of_int n) |> ok
let did n = Gpuio.Chart_data.Datum_id.of_int64 (Int64.of_int n) |> ok
let alpha n = C.rgba ~red:0 ~green:0 ~blue:0 ~alpha:n |> ok
let resolve t = A.Expert.to_wire t ~theme:Gpuio.Theme.default |> ok

let%expect_test "chart style resolves appearance and rejects forged nested values" =
  let module S = Gpuio.Chart_style in
  let token = C.token_exn "chart-appearance-style" in
  let appearance =
    A.create ~series:[ A.Series.create ~series:(sid 1) ~legend:token () ] () |> ok
  in
  assert (Result.is_error (S.create ~appearance ()));
  let theme = Gpuio.Theme.create [ "chart-appearance-style", alpha 7 ] |> ok in
  let style = S.create ~appearance ~theme () |> ok |> S.Expert.to_wire in
  assert (W.equal style.appearance (A.Expert.to_wire appearance ~theme |> ok));
  assert (Int64.equal style.version (-8L));
  let first = List.hd_exn style.appearance.series in
  assert (
    Result.is_error
      (S.Expert.of_wire
         { style with
           appearance =
             { style.appearance with series = [ { first with legend = Some (-1L) } ] }
         }));
  assert (Result.is_error (S.Expert.of_wire { style with version = -4L }));
  assert (Result.is_error (S.Expert.of_wire { style with version = -7L }));
  let bytes =
    Bin_prot.Utils.bin_dump
      Gpuio_protocol.Chart_view_wire.Observation.bin_writer_t
      (Failed Invalid_config)
    |> Bigstring.to_string
  in
  assert (String.equal bytes "\001\004");
  print_s
    [%sexp "style schema -8; resolved nested colors; invalid config observation 01 04"];
  [%expect
    {| "style schema -8; resolved nested colors; invalid config observation 01 04" |}]
;;

let%expect_test "appearance independently paired bytes preserve omission and variants" =
  let gradient =
    B.linear_gradient_in Oklab ~angle:90. ~from:(alpha 3, 0.25) ~to_:(alpha 4, 0.75) |> ok
  in
  let path =
    A.Path.create
      ~stroke:(A.Stroke.create ~visible:false ~width:2. (B.solid (alpha 2)) |> ok)
      ~fill:gradient
      ~curve:Step_after
      ()
  in
  let marker =
    A.Marker.create
      ~visible:true
      ~radius:24.
      ~fill:(alpha 5)
      ~stroke:(alpha 6)
      ~stroke_width:8.
      ()
    |> ok
  in
  let corners =
    A.Corners.create ~top_left:1. ~top_right:2. ~bottom_right:3. ~bottom_left:4. () |> ok
  in
  let bar fill = A.Bar.create ~fill ~corners () in
  let data =
    [ A.Bar_fill.background gradient
    ; A.Bar_fill.base_to_tip ~from:(alpha 7) ~to_:(alpha 8)
    ; A.Bar_fill.domain ~from:(alpha 9) ~to_:(alpha 10)
    ; A.Bar_fill.values ~from:(-2., alpha 11) ~to_:(3., alpha 12) |> ok
    ]
    |> List.mapi ~f:(fun i fill ->
      A.Datum.create ~series:(sid 1) ~datum:(did (i + 1)) ~bar:(bar fill) ())
  in
  let t =
    A.create
      ~series:[ A.Series.create ~series:(sid 1) ~path ~marker ~legend:(alpha 13) () ]
      ~data
      ~aggregates:Uniform
      ()
    |> ok
    |> resolve
  in
  assert (W.valid t);
  let bytes = Bin_prot.Utils.bin_dump W.bin_writer_t t in
  Bigstring.to_string bytes
  |> String.to_list
  |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
  |> String.concat
  |> print_endline;
  [%expect
    {| 01010101000100000000000000400002010101000000000080564003000000000000d03f04000000000000e83f01020101010100000000000038400105010601000000000000204000010d040101000101000101000000000080564003000000000000d03f04000000000000e83f01000000000000f03f000000000000004000000000000008400000000000001040010200010101070801000000000000f03f000000000000004000000000000008400000000000001040010300010102090a01000000000000f03f00000000000000400000000000000840000000000000104001040001010300000000000000c00b00000000000008400c01000000000000f03f00000000000000400000000000000840000000000000104001 |}]
;;

let%expect_test "constructor bounds and identity namespaces" =
  List.iter [ Float.nan; Float.infinity; Float.neg_infinity; -0.1; 32.1 ] ~f:(fun n ->
    assert (Result.is_error (A.Corners.all n)));
  List.iter [ Float.nan; Float.infinity; 0.99; 24.01 ] ~f:(fun n ->
    assert (Result.is_error (A.Marker.create ~radius:n ())));
  List.iter [ Float.nan; Float.infinity; -0.01; 8.01 ] ~f:(fun n ->
    assert (Result.is_error (A.Marker.create ~stroke_width:n ())));
  List.iter [ Float.nan; Float.infinity; 0.49; 8.01 ] ~f:(fun n ->
    assert (Result.is_error (A.Stroke.create ~width:n (B.solid (alpha 1)))));
  List.iter
    [ Float.nan; Float.infinity; Float.neg_infinity; -1.01e100; 0.; 1. ]
    ~f:(fun lo ->
      assert (Result.is_error (A.Bar_fill.values ~from:(lo, alpha 1) ~to_:(0., alpha 2))));
  assert (Result.is_ok (A.Bar_fill.values ~from:(-1e100, alpha 1) ~to_:(1e100, alpha 2)));
  let d = A.Datum.create ~series:(sid 1) ~datum:(did 1) () in
  let s = A.Series.create ~series:(sid 1) () in
  assert (Result.is_error (A.create ~series:[ s; s ] ()));
  assert (Result.is_error (A.create ~data:[ d; d ] ()));
  assert (
    Result.is_ok
      (A.create
         ~series:[ s ]
         ~data:[ d; A.Datum.create ~series:(sid 2) ~datum:(did 1) () ]
         ()));
  let series = List.init 128 ~f:(fun i -> A.Series.create ~series:(sid (i + 1)) ()) in
  let data =
    List.init 1024 ~f:(fun i -> A.Datum.create ~series:(sid 1) ~datum:(did (i + 1)) ())
  in
  assert (Result.is_ok (A.create ~series ~data ()));
  assert (
    Result.is_error (A.create ~series:(A.Series.create ~series:(sid 129) () :: series) ()));
  assert (
    Result.is_error
      (A.create ~data:(A.Datum.create ~series:(sid 2) ~datum:(did 1) () :: data) ()));
  print_s [%sexp "finite bounds, duplicate rejection and series-scoped datum IDs pass"];
  [%expect {| "finite bounds, duplicate rejection and series-scoped datum IDs pass" |}]
;;

let%expect_test "all nested theme colors resolve even when hidden or unused" =
  let token = C.token_exn "chart-appearance-test-color" in
  let brush = B.linear_gradient ~angle:180. ~from:(token, 0.) ~to_:(token, 1.) |> ok in
  let path =
    A.Path.create ~stroke:(A.Stroke.create ~visible:false brush |> ok) ~fill:brush ()
  in
  let marker = A.Marker.create ~visible:false ~fill:token ~stroke:token () |> ok in
  let fills =
    [ A.Bar_fill.background brush
    ; A.Bar_fill.base_to_tip ~from:token ~to_:token
    ; A.Bar_fill.domain ~from:token ~to_:token
    ; A.Bar_fill.values ~from:(0., token) ~to_:(1., token) |> ok
    ]
  in
  List.iter fills ~f:(fun fill ->
    let bar = A.Bar.create ~fill () in
    let appearance =
      A.create
        ~series:[ A.Series.create ~series:(sid 1) ~path ~marker ~bar ~legend:token () ]
        ~data:[ A.Datum.create ~series:(sid 2) ~datum:(did 1) ~marker ~bar () ]
        ()
      |> ok
    in
    assert (Result.is_error (A.Expert.to_wire appearance ~theme:Gpuio.Theme.default));
    let theme = Gpuio.Theme.create [ "chart-appearance-test-color", alpha 17 ] |> ok in
    let resolved = A.Expert.to_wire appearance ~theme |> ok in
    assert (W.valid resolved);
    let series = List.hd_exn resolved.series in
    assert (Option.equal Int64.equal series.legend (Some 17L));
    assert (Option.equal Int64.equal (Option.value_exn series.marker).fill (Some 17L)));
  assert (W.equal (resolve A.empty) W.empty);
  print_s [%sexp "hidden paths, markers, legend and all bar fill modes resolve"];
  [%expect {| "hidden paths, markers, legend and all bar fill modes resolve" |}]
;;

let%expect_test "each dormant color position rejects an unknown token independently" =
  let token = C.token_exn "chart-appearance-unknown" in
  let solid = B.solid token in
  let gradient_from =
    B.linear_gradient ~angle:90. ~from:(token, 0.) ~to_:(alpha 1, 1.) |> ok
  in
  let gradient_to =
    B.linear_gradient ~angle:90. ~from:(alpha 1, 0.) ~to_:(token, 1.) |> ok
  in
  let paths =
    [ A.Path.create ~stroke:(A.Stroke.create ~visible:false solid |> ok) ()
    ; A.Path.create ~fill:solid ()
    ; A.Path.create ~fill:gradient_from ()
    ; A.Path.create ~fill:gradient_to ()
    ]
  in
  let markers =
    [ A.Marker.create ~visible:false ~fill:token () |> ok
    ; A.Marker.create ~stroke:token () |> ok
    ]
  in
  let fills =
    [ A.Bar_fill.background solid
    ; A.Bar_fill.base_to_tip ~from:token ~to_:(alpha 1)
    ; A.Bar_fill.base_to_tip ~from:(alpha 1) ~to_:token
    ; A.Bar_fill.domain ~from:token ~to_:(alpha 1)
    ; A.Bar_fill.domain ~from:(alpha 1) ~to_:token
    ; A.Bar_fill.values ~from:(0., token) ~to_:(1., alpha 1) |> ok
    ; A.Bar_fill.values ~from:(0., alpha 1) ~to_:(1., token) |> ok
    ]
  in
  let series =
    List.map paths ~f:(fun path -> A.Series.create ~series:(sid 1) ~path ())
    @ List.map markers ~f:(fun marker -> A.Series.create ~series:(sid 1) ~marker ())
    @ List.map fills ~f:(fun fill ->
      A.Series.create ~series:(sid 1) ~bar:(A.Bar.create ~fill ()) ())
    @ [ A.Series.create ~series:(sid 1) ~legend:token () ]
  in
  let data =
    List.map markers ~f:(fun marker ->
      A.Datum.create ~series:(sid 1) ~datum:(did 1) ~marker ())
    @ List.map fills ~f:(fun fill ->
      A.Datum.create ~series:(sid 1) ~datum:(did 1) ~bar:(A.Bar.create ~fill ()) ())
  in
  let appearances =
    List.map series ~f:(fun s -> A.create ~series:[ s ] () |> ok)
    @ List.map data ~f:(fun d -> A.create ~data:[ d ] () |> ok)
  in
  List.iter appearances ~f:(fun appearance ->
    assert (Result.is_error (A.Expert.to_wire appearance ~theme:Gpuio.Theme.default)));
  print_s [%sexp (List.length appearances : int)];
  [%expect {| 23 |}]
;;

let%expect_test "maximum wire appearance fits an independently bounded envelope" =
  let c = 0xffff_ffffL in
  let brush =
    W.Brush.Linear
      { oklab = true; angle = 360.; from = c; start = 0.; to_ = c; stop = 1. }
  in
  let marker =
    { W.Marker.visible = Some true
    ; radius = Some 24.
    ; fill = Some c
    ; stroke = Some c
    ; stroke_width = Some 8.
    }
  in
  let bar =
    { W.Bar.fill = Some (Background brush)
    ; corners =
        Some
          { W.Corners.top_left = 32.
          ; top_right = 32.
          ; bottom_right = 32.
          ; bottom_left = 32.
          }
    }
  in
  let path =
    { W.Path.stroke = Some { W.Stroke.visible = true; width = Some 8.; brush }
    ; fill = Some brush
    ; curve = Some Natural
    }
  in
  let series =
    List.init 128 ~f:(fun i ->
      { W.Series.series = Int64.(max_value - of_int i)
      ; path = Some path
      ; marker = Some marker
      ; bar = Some bar
      ; legend = Some c
      })
  in
  let data =
    List.init 1024 ~f:(fun i ->
      { W.Datum.series = Int64.max_value
      ; datum = Int64.(max_value - of_int i)
      ; marker = Some marker
      ; bar = Some bar
      })
  in
  let t = { W.series; data; aggregates = Uniform } in
  assert (W.valid t);
  let bytes = W.bin_size_t t in
  assert (bytes <= 192 * 1024);
  print_s [%sexp (bytes : int)];
  [%expect {| 173447 |}]
;;

let%expect_test "native pattern brushes resolve once and have paired encodings" =
  let brushes =
    [ B.pattern_slash (alpha 0) ~width:2. ~interval:4. |> ok
    ; B.checkerboard (alpha 0) ~size:8. |> ok
    ]
  in
  List.iter brushes ~f:(fun fill ->
    let value =
      A.create
        ~series:
          [ A.Series.create
              ~series:(sid 1)
              ~path:(A.Path.create ~fill ())
              ~bar:(A.Bar.create ~fill:(A.Bar_fill.background fill) ())
              ()
          ]
        ()
      |> ok
      |> resolve
    in
    let series = List.hd_exn value.series in
    let brush = (Option.value_exn series.path).fill |> Option.value_exn in
    assert (
      W.Bar_fill.equal
        (Option.value_exn (Option.value_exn series.bar).fill)
        (Background brush));
    Bin_prot.Utils.bin_dump W.Brush.bin_writer_t brush
    |> Bigstring.to_string
    |> String.concat_map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
    |> print_endline);
  [%expect
    {|
    020000000000000000400000000000001040
    03000000000000002040
    |}]
;;
