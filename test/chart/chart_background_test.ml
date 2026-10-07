open Core
module D = Gpuio.Chart_data
module W = Gpuio_protocol.Chart_data_wire
module B = Gpuio.Background

let ok = Or_error.ok_exn
let sid n = D.Series_id.of_int64 n |> ok
let did n = D.Datum_id.of_int64 n |> ok
let color n = Gpuio.Color.rgba ~red:0 ~green:0 ~blue:0 ~alpha:(Int64.to_int_exn n) |> ok

let background ?(series = 7L) id brush =
  D.Bar_background.create ~series:(sid series) ~datum:(did id) brush
;;

let point id x y = D.Point.create ~id:(did id) ~x ~y:(Some y) () |> ok

let source () =
  D.bar
    [ D.Series.create ~id:(sid 7L) ~name:"Bars" [ point 9L 0. 2.; point 3L 1. (-3.) ]
      |> ok
    ]
  |> ok
;;

let from_hex text =
  let hex = String.strip text in
  String.init
    (String.length hex / 2)
    ~f:(fun i ->
      Char.of_int_exn (Int.of_string ("0x" ^ String.sub hex ~pos:(2 * i) ~len:2)))
;;

let%expect_test "historical schema-1 payloads cannot omit the new sidecar" =
  Eio_main.run (fun env ->
    Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / "chart-v1-data.hex")
    |> String.split_lines
    |> List.iter ~f:(fun line ->
      let _, hex = String.lsplit2_exn line ~on:' ' in
      assert (Result.is_error (D.Expert.decode (from_hex hex)))));
  print_endline "all historical data fixtures rejected";
  [%expect {| all historical data fixtures rejected |}]
;;

let%expect_test "background pairs canonicalize without changing source identity or order" =
  let original = source () in
  let entries =
    [ background 9L (B.solid (color 9L))
    ; background 3L (B.checkerboard (color 10L) ~size:8. |> ok)
    ]
  in
  let styled = D.with_bar_backgrounds original entries |> ok in
  assert (D.equal styled (D.with_bar_backgrounds original (List.rev entries) |> ok));
  assert (D.Expert.equal_contents (D.Expert.contents original) (D.Expert.contents styled));
  assert (D.equal original (D.with_bar_backgrounds styled [] |> ok));
  Eio_main.run (fun env ->
    let expected =
      Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / "chart-v3-backgrounds.hex") |> from_hex
    in
    assert (String.equal expected (D.Expert.encode styled |> ok));
    assert (D.equal styled (D.Expert.decode expected |> ok));
    for len = 0 to String.length expected - 1 do
      assert (Result.is_error (D.Expert.decode (String.prefix expected len)))
    done;
    assert (Result.is_error (D.Expert.decode (expected ^ "\000"))));
  print_s [%sexp ((D.Expert.to_wire styled).bar_backgrounds : W.Bar_background.t list)];
  [%expect
    {|
    (((series 7) (datum 3) (brush (Checkerboard 10 8)))
     ((series 7) (datum 9) (brush (Solid 9))))
    |}]
;;

let%expect_test "sidecar validation does not bypass source, brush or theme invariants" =
  let original = source () in
  let brush = B.solid (color 9L) in
  let one = background 9L brush in
  List.iter
    [ [ one; one ]; [ background 10L brush ]; [ background ~series:8L 9L brush ] ]
    ~f:(fun entries -> assert (Result.is_error (D.with_bar_backgrounds original entries)));
  let line =
    D.line [ D.Series.create ~id:(sid 7L) ~name:"Line" [ point 9L 0. 2. ] |> ok ] |> ok
  in
  assert (Result.is_error (D.with_bar_backgrounds line [ one ]));
  let token = Gpuio.Color.token_exn "dense-custom" in
  let entry = background 9L (B.solid token) in
  assert (Result.is_error (D.with_bar_backgrounds original [ entry ]));
  let theme = Gpuio.Theme.create [ "dense-custom", color 9L ] |> ok in
  let styled = D.with_bar_backgrounds original ~theme [ entry ] |> ok in
  assert (D.equal styled (D.with_bar_backgrounds original [ one ] |> ok));
  let wire = D.Expert.to_wire styled in
  let first = List.hd_exn wire.bar_backgrounds in
  List.iter
    [ { wire with version = 1L }
    ; { wire with bar_backgrounds = [ first; first ] }
    ; { wire with bar_backgrounds = [ { first with datum = 100L } ] }
    ; { wire with
        bar_backgrounds = [ { first with brush = Checkerboard (9L, Float.nan) } ]
      }
    ]
    ~f:(fun wire -> assert (Result.is_error (D.Expert.of_wire wire)));
  print_endline
    "invalid pairs, nonbar references, old schemas, brushes and unknown tokens rejected";
  [%expect
    {| invalid pairs, nonbar references, old schemas, brushes and unknown tokens rejected |}]
;;

let%expect_test
    "100k dense backgrounds roundtrip and remain accounted outside view metadata"
  =
  let count = 100_000 in
  let points =
    List.init count ~f:(fun i ->
      point (Int64.of_int (count - i)) (Float.of_int i) (Float.of_int (i mod 7)))
  in
  let original =
    D.bar [ D.Series.create ~id:(sid 7L) ~name:"Dense" points |> ok ] |> ok
  in
  let entries =
    List.mapi points ~f:(fun i p ->
      background
        (D.Datum_id.to_int64 (D.Point.id p))
        (B.solid (color (Int64.of_int (i mod 256)))))
  in
  let styled = D.with_bar_backgrounds original entries |> ok in
  let bytes = D.Expert.encode styled |> ok in
  assert (String.length bytes < W.max_bytes);
  assert (D.equal styled (D.Expert.decode bytes |> ok));
  assert (D.value_count styled = count);
  assert (D.Expert.retained_bytes styled - D.Expert.retained_bytes original = 512 * count);
  assert (
    Result.is_error (D.with_bar_backgrounds original (List.hd_exn entries :: entries)));
  print_s
    [%sexp
      (D.value_count styled : int)
    , (List.length (D.Expert.to_wire styled).bar_backgrounds : int)];
  [%expect {| (100000 100000) |}]
;;

let%expect_test
    "combined source and brush bytes can exceed the envelope despite legal counts"
  =
  let count = 100_000 in
  let series_id = sid Int64.max_value in
  let points =
    List.init count ~f:(fun i ->
      D.Point.create
        ~id:(did Int64.(max_value - of_int i))
        ~x:(Float.of_int i)
        ~y:(Some 1.)
        ~label:(String.make 83 'x')
        ()
      |> ok)
  in
  let original =
    D.bar [ D.Series.create ~id:series_id ~name:"Dense" points |> ok ] |> ok
  in
  assert (String.length (D.Expert.encode original |> ok) < W.max_bytes);
  let white = Gpuio.Color.rgb_exn 0xffffff in
  let brush = B.linear_gradient ~angle:180. ~from:(white, 0.) ~to_:(white, 1.) |> ok in
  let entries =
    List.map points ~f:(fun point ->
      D.Bar_background.create ~series:series_id ~datum:(D.Point.id point) brush)
  in
  print_s [%sexp (Result.is_error (D.with_bar_backgrounds original entries) : bool)];
  [%expect {| true |}]
;;

let%expect_test "categorical gaps may own brushes without becoming defined observations" =
  let category_id = D.Category_id.of_int64 1L |> ok in
  let categories = [ D.Category.create ~id:category_id ~label:"Missing" |> ok ] in
  let points =
    [ D.Categorical_point.create ~id:(did 3L) ~category:category_id ~value:None () |> ok ]
  in
  let series = D.Categorical_series.create ~id:(sid 7L) ~name:"Missing" points |> ok in
  let original = D.categorical ~categories [ Bar series ] |> ok in
  let styled =
    D.with_bar_backgrounds original [ background 3L (B.solid (color 9L)) ] |> ok
  in
  assert (D.Expert.equal_contents (D.Expert.contents original) (D.Expert.contents styled));
  assert (D.equal styled (D.Expert.decode (D.Expert.encode styled |> ok) |> ok));
  print_s [%sexp (D.value_count styled : int)];
  [%expect {| 1 |}]
;;
