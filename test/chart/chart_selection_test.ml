open Core
module S = Gpuio.Chart_selection
module D = Gpuio.Chart_data
module W = Gpuio_protocol.Chart_selection_wire

let ok = Or_error.ok_exn
let datum n = D.Datum_id.of_int64 n |> ok
let series = D.Series_id.of_int64 9L |> ok

let span =
  S.Span.create ~start_index:1 ~length:3 ~first:(datum 42L) ~last:(datum 7L) |> ok
;;

let single =
  S.Span.create ~start_index:2 ~length:1 ~first:(datum 17L) ~last:(datum 17L) |> ok
;;

let cases =
  [ S.cartesian ~series ~span:single ~aggregation:Exact |> ok
  ; S.cartesian ~series ~span ~aggregation:Sum |> ok
  ; S.cartesian ~series ~span ~aggregation:Mean |> ok
  ; S.slice (datum 7L)
  ; S.radar ~series ~axis:(datum 3L)
  ; S.candlestick ~span:single ~aggregated:false |> ok
  ; S.candlestick ~span ~aggregated:true |> ok
  ; S.node (D.Node_id.of_int64 1L |> ok)
  ; S.edge (D.Edge_id.of_int64 5L |> ok)
  ]
;;

let hex bytes =
  String.to_list bytes
  |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
  |> String.concat
;;

let unhex hex =
  String.init
    (String.length hex / 2)
    ~f:(fun i ->
      Int.of_string ("0x" ^ String.sub hex ~pos:(i * 2) ~len:2) |> Char.of_int_exn)
;;

let%expect_test "semantic targets and selection events match independent fixtures" =
  Eio_main.run (fun env ->
    let fixtures =
      Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / "chart-v1-selection.hex")
      |> String.split_lines
    in
    List.iter2_exn cases fixtures ~f:(fun selection fixture ->
      let _, expected = String.lsplit2_exn fixture ~on:' ' in
      let wire = S.Expert.to_wire selection in
      assert (S.equal selection (S.Expert.of_wire wire |> ok));
      let bytes = Bin_prot.Utils.bin_dump W.bin_writer_t wire |> Bigstring.to_string in
      assert (String.equal (hex bytes) expected);
      let encoded = unhex ("013e0001000100010101070202030201" ^ expected) in
      let events = Gpuio_protocol.Wire.Event.decode encoded |> ok in
      (match events with
       | [ Chart_event (_, _, _, _, _, 2L, 3L, Selection_changed (Some target)) ] ->
         assert (S.equal selection (S.Expert.of_wire target |> ok))
       | _ -> assert false);
      let event =
        Gpuio.Chart.Expert.event
          ~data_revision:2L
          ~data_generation:3L
          (Selection_changed (Some wire))
        |> ok
      in
      assert (
        Gpuio.Chart.Observation.equal
          event.observation
          (Selection_changed (Some selection)));
      for length = 0 to String.length encoded - 1 do
        assert (
          Result.is_error
            (Gpuio_protocol.Wire.Event.decode (String.prefix encoded length)))
      done));
  print_endline
    "all families, exact/aggregate provenance, event tag 2 and truncation checks pass";
  [%expect
    {| all families, exact/aggregate provenance, event tag 2 and truncation checks pass |}]
;;

let%expect_test "span boundaries and exact selection invariants are validated" =
  List.iter
    [ -1, 1; 100_000, 1; 99_999, 2; 0, 0; 0, Int.max_value ]
    ~f:(fun (start_index, length) ->
      assert (
        Result.is_error
          (S.Span.create ~start_index ~length ~first:(datum 42L) ~last:(datum 7L))));
  assert (Result.is_error (S.cartesian ~series ~span ~aggregation:Exact));
  assert (Result.is_error (S.candlestick ~span ~aggregated:false));
  List.iter
    [ W.Slice 0L
    ; W.Radar { series = 9L; axis = -1L }
    ; W.Candlestick
        { span = { start_index = Int64.max_value; length = 1L; first = 1L; last = 1L }
        ; aggregated = false
        }
    ; W.Candlestick
        { span = { start_index = 0L; length = Int64.max_value; first = 42L; last = 7L }
        ; aggregated = true
        }
    ]
    ~f:(fun wire -> assert (Result.is_error (S.Expert.of_wire wire)));
  assert (
    Result.is_error
      (Gpuio.Chart.Expert.event
         ~data_revision:0L
         ~data_generation:0L
         (Selection_changed None)));
  assert (
    Result.is_ok
      (Gpuio.Chart.Expert.event
         ~data_revision:1L
         ~data_generation:1L
         (Selection_changed None)));
  print_endline
    "ordered positions are bounded; IDs need not be numeric intervals; no selection \
     before data";
  [%expect
    {| ordered positions are bounded; IDs need not be numeric intervals; no selection before data |}]
;;
