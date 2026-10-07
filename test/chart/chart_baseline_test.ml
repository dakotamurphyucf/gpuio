open Core
module D = Gpuio.Chart_data
module W = Gpuio_protocol.Chart_data_wire
module Fixture = Chart_background_test

let ok = Or_error.ok_exn

let entry id value =
  D.Bar_baseline.create ~series:(Fixture.sid 7L) ~datum:(Fixture.did id) value |> ok
;;

let%expect_test "baseline identities, independent bytes, clear and coexistence" =
  let source = Fixture.source () in
  let backgrounds =
    [ Fixture.background 9L (Gpuio.Background.solid (Fixture.color 9L))
    ; Fixture.background
        3L
        (Gpuio.Background.checkerboard (Fixture.color 10L) ~size:8. |> ok)
    ]
  in
  let colored = D.with_bar_backgrounds source backgrounds |> ok in
  let entries = [ entry 9L 1.; entry 3L (-1.) ] in
  let data = D.with_bar_baselines colored entries |> ok in
  assert (D.equal data (D.with_bar_baselines colored (List.rev entries) |> ok));
  assert (D.equal colored (D.with_bar_baselines data [] |> ok));
  assert (
    D.equal
      (D.with_bar_baselines source entries |> ok)
      (D.with_bar_backgrounds data [] |> ok));
  assert (D.Expert.equal_contents (D.Expert.contents source) (D.Expert.contents data));
  print_s
    [%sexp
      (D.bar_baseline data ~series:(Fixture.sid 7L) ~datum:(Fixture.did 9L)
       : float option)
    , (D.bar_baseline data ~series:(Fixture.sid 7L) ~datum:(Fixture.did 8L)
       : float option)];
  Eio_main.run (fun env ->
    let expected =
      Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / "chart-v3-baselines.hex")
      |> Fixture.from_hex
    in
    assert (String.equal expected (D.Expert.encode data |> ok));
    assert (D.equal data (D.Expert.decode expected |> ok));
    for len = 0 to String.length expected - 1 do
      assert (Result.is_error (D.Expert.decode (String.prefix expected len)))
    done;
    assert (Result.is_error (D.Expert.decode (expected ^ "\000")));
    Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / "chart-v2-data.hex")
    |> String.split_lines
    |> List.iter ~f:(fun line ->
      let _, hex = String.lsplit2_exn line ~on:' ' in
      assert (Result.is_error (D.Expert.decode (Fixture.from_hex hex)))));
  [%expect {| ((1) ()) |}]
;;

let%expect_test "invalid baselines cannot bypass domain or wire validation" =
  let source = Fixture.source () in
  List.iter [ Float.nan; Float.infinity; Float.neg_infinity; 1.1e100 ] ~f:(fun value ->
    assert (
      Result.is_error
        (D.Bar_baseline.create ~series:(Fixture.sid 7L) ~datum:(Fixture.did 9L) value)));
  List.iter
    [ [ entry 9L 1.; entry 9L 2. ]; [ entry 10L 1. ] ]
    ~f:(fun entries -> assert (Result.is_error (D.with_bar_baselines source entries)));
  let line =
    D.line
      [ D.Series.create ~id:(Fixture.sid 7L) ~name:"Line" [ Fixture.point 9L 0. 2. ] |> ok
      ]
    |> ok
  in
  assert (Result.is_error (D.with_bar_baselines line [ entry 9L 1. ]));
  let wire =
    D.Expert.to_wire (D.with_bar_baselines source [ entry 9L 1.; entry 3L (-1.) ] |> ok)
  in
  let first = List.hd_exn wire.bar_baselines in
  List.iter
    [ { wire with version = 2L }
    ; { wire with bar_baselines = List.rev wire.bar_baselines }
    ; { wire with bar_baselines = [ first; first ] }
    ; { wire with bar_baselines = [ { first with baseline = Float.nan } ] }
    ; { wire with bar_baselines = [ { first with datum = 100L } ] }
    ]
    ~f:(fun wire -> assert (Result.is_error (D.Expert.of_wire wire)));
  print_endline
    "scalars, references, ordering, duplicate pairs and previous schema rejected";
  [%expect
    {| scalars, references, ordering, duplicate pairs and previous schema rejected |}]
;;

let%expect_test "100k baselines roundtrip and retained memory accounting" =
  let points =
    List.init 100_000 ~f:(fun i ->
      Fixture.point (Int64.of_int (100_000 - i)) (Float.of_int i) 3.)
  in
  let source =
    D.bar [ D.Series.create ~id:(Fixture.sid 7L) ~name:"Dense" points |> ok ] |> ok
  in
  let entries =
    List.map points ~f:(fun p ->
      D.Bar_baseline.create ~series:(Fixture.sid 7L) ~datum:(D.Point.id p) 2. |> ok)
  in
  let data = D.with_bar_baselines source entries |> ok in
  assert (D.equal data (D.Expert.decode (D.Expert.encode data |> ok) |> ok));
  assert (D.Expert.retained_bytes data - D.Expert.retained_bytes source = 128 * 100_000);
  assert (Result.is_error (D.with_bar_baselines source (List.hd_exn entries :: entries)));
  print_s
    [%sexp
      (D.value_count data : int)
    , (List.length (D.Expert.to_wire data).bar_baselines : int)];
  [%expect {| (100000 100000) |}]
;;
