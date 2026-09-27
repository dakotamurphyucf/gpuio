open Core
module S = Gpuio.Chart_sampling
module W = Gpuio_protocol.Chart_sampling_wire

let hex bytes =
  String.to_list bytes
  |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
  |> String.concat
;;

let%expect_test "explicit family policies and independently fixed wire bytes" =
  let policies =
    [ S.default
    ; S.create ~line:S.Line.exact ()
    ; S.create
        ~line:(S.Line.envelope ~max_buckets:1 |> Or_error.ok_exn)
        ~bars:(S.Bar.sum ~max_buckets:8 |> Or_error.ok_exn)
        ~candles:(S.Candlestick.ohlc ~max_buckets:8192 |> Or_error.ok_exn)
        ()
    ; S.create ~bars:(S.Bar.mean ~max_buckets:128 |> Or_error.ok_exn) ()
    ]
  in
  List.iter policies ~f:(fun policy ->
    let wire = S.Expert.to_wire policy in
    assert (S.equal policy (S.Expert.of_wire wire |> Or_error.ok_exn));
    print_endline
      (Bin_prot.Utils.bin_dump W.bin_writer_t wire |> Bigstring.to_string |> hex));
  assert (S.Bar.equal (S.bars S.default) S.Bar.exact);
  assert (S.Candlestick.equal (S.candles S.default) S.Candlestick.exact);
  assert (not (S.Line.equal (S.line S.default) S.Line.exact));
  [%expect
    {|
    0101fe00040000
    01000000
    010101010801fe0020
    0101fe000402fe800000 |}]
;;

let%expect_test "bucket bounds and decoded policy validation" =
  List.iter [ -1; 0; 8193; Int.max_value ] ~f:(fun max_buckets ->
    assert (Result.is_error (S.Line.envelope ~max_buckets));
    assert (Result.is_error (S.Bar.sum ~max_buckets));
    assert (Result.is_error (S.Bar.mean ~max_buckets));
    assert (Result.is_error (S.Candlestick.ohlc ~max_buckets)));
  let wire = S.Expert.to_wire S.default in
  List.iter
    [ { wire with version = 2L }
    ; { wire with line = Envelope 0L }
    ; { wire with bars = Sum Int64.max_value }
    ; { wire with bars = Mean (-1L) }
    ; { wire with candles = Ohlc 8193L }
    ]
    ~f:(fun wire -> assert (Result.is_error (S.Expert.of_wire wire)));
  print_endline
    "line, bar and OHLC policies validate [1,8192]; raw values cannot bypass it";
  [%expect
    {| line, bar and OHLC policies validate [1,8192]; raw values cannot bypass it |}]
;;
