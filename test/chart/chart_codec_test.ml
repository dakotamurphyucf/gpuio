open Core
module D = Gpuio.Chart_data
module W = Gpuio_protocol.Chart_data_wire

let datum n = D.Datum_id.of_int64 n |> Or_error.ok_exn
let sid n = D.Series_id.of_int64 n |> Or_error.ok_exn
let nid n = D.Node_id.of_int64 n |> Or_error.ok_exn
let eid n = D.Edge_id.of_int64 n |> Or_error.ok_exn
let point id x y label = D.Point.create ~id:(datum id) ~x ~y ~label () |> Or_error.ok_exn
let series id name points = D.Series.create ~id:(sid id) ~name points |> Or_error.ok_exn

let fixture () =
  [ ( "cartesian"
    , D.cartesian
        [ Line (series 1L "Rate" [ point 1L 0. (Some (-2.)) "α"; point 2L 1. None "" ])
        ; Area (series 2L "Load" [ point 1L 0. (Some 3.) "" ])
        ; Bar (series 3L "Count" [ point 1L 0. (Some 0.) "0" ])
        ] )
  ; ( "pie"
    , D.pie
        (List.map
           [ 7L, "Cache", 2.; 8L, "Other", 0. ]
           ~f:(fun (id, label, value) ->
             D.Slice.create ~id:(datum id) ~label ~value |> Or_error.ok_exn)) )
  ; ( "radar"
    , D.radar
        ~axes:
          (List.map
             [ 1L, "A", 10.; 2L, "B", 20.; 3L, "C", 30. ]
             ~f:(fun (id, label, maximum) ->
               D.Radar_axis.create ~id:(datum id) ~label ~maximum |> Or_error.ok_exn))
        [ D.Radar_series.create
            ~id:(sid 9L)
            ~name:"Agent"
            [ datum 3L, 15.; datum 1L, 5.; datum 2L, 0. ]
          |> Or_error.ok_exn
        ] )
  ; ( "candlestick"
    , D.candlestick
        [ D.Candle.create
            ~id:(datum 1L)
            ~x:1.
            ~label:"Day"
            ~open_:(-2.)
            ~high:0.
            ~low:(-4.)
            ~close:(-1.)
          |> Or_error.ok_exn
        ] )
  ; ( "sankey"
    , D.sankey
        ~nodes:
          [ D.Node.create ~id:(nid 1L) ~label:"Start" |> Or_error.ok_exn
          ; D.Node.create ~id:(nid 2L) ~label:"End" |> Or_error.ok_exn
          ]
        ~edges:
          [ D.Edge.create ~id:(eid 5L) ~source:(nid 1L) ~target:(nid 2L) ~value:1.5
            |> Or_error.ok_exn
          ] )
  ]
  |> List.map ~f:(fun (name, data) -> name, Or_error.ok_exn data)
;;

let bytes_of_hex hex =
  String.init
    (String.length hex / 2)
    ~f:(fun i ->
      Char.of_int_exn (Int.of_string ("0x" ^ String.sub hex ~pos:(2 * i) ~len:2)))
;;

let raw wire = Bin_prot.Utils.bin_dump W.bin_writer_t wire |> Bigstring.to_string

let%expect_test "independent all-family fixtures fix schema tags, field order and UTF-8" =
  Eio_main.run (fun env ->
    let lines =
      Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / "chart-v2-data.hex")
      |> String.split_lines
    in
    List.iter2_exn (fixture ()) lines ~f:(fun (name, data) line ->
      let expected_name, hex = String.lsplit2_exn line ~on:' ' in
      assert (String.equal name expected_name);
      let expected = bytes_of_hex hex in
      let encoded = D.Expert.encode data |> Or_error.ok_exn in
      assert (String.equal encoded expected);
      assert (D.equal data (D.Expert.decode expected |> Or_error.ok_exn));
      for len = 0 to String.length expected - 1 do
        assert (Result.is_error (D.Expert.decode (String.prefix expected len)))
      done;
      assert (Result.is_error (D.Expert.decode (expected ^ "\000")));
      print_s [%sexp (name : string), (D.value_count data : int)]));
  [%expect
    {|
    (cartesian 4)
    (pie 2)
    (radar 3)
    (candlestick 1)
    (sankey 1) |}]
;;

let%expect_test "decoded raw wire values must pass domain constructors" =
  let cases =
    [ { W.version = 3L; bar_backgrounds = []; contents = Pie [] }
    ; { version = 2L
      ; bar_backgrounds = []
      ; contents = Pie [ { id = 0L; label = "A"; value = 0. } ]
      }
    ; { version = 2L
      ; bar_backgrounds = []
      ; contents = Pie [ { id = 1L; label = "A"; value = -1. } ]
      }
    ; { version = 2L
      ; bar_backgrounds = []
      ; contents = Pie [ { id = 1L; label = "A"; value = Float.nan } ]
      }
    ; { version = 2L
      ; bar_backgrounds = []
      ; contents =
          Cartesian
            [ Bar
                { id = 1L
                ; name = "B"
                ; points = [ { id = 1L; x = 0.; y = None; label = "" } ]
                }
            ]
      }
    ; { version = 2L
      ; bar_backgrounds = []
      ; contents =
          Cartesian
            [ Line
                { id = 1L
                ; name = "B"
                ; points =
                    [ { id = 1L; x = 0.; y = None; label = "" }
                    ; { id = 2L; x = 0.; y = None; label = "" }
                    ]
                }
            ]
      }
    ; { version = 2L
      ; bar_backgrounds = []
      ; contents =
          Candlestick
            [ { id = 1L; x = 0.; label = ""; open_ = 1.; high = 0.; low = 0.; close = 0. }
            ]
      }
    ; { version = 2L
      ; bar_backgrounds = []
      ; contents =
          Sankey
            ( [ { id = 1L; label = "N" } ]
            , [ { id = 1L; source = 1L; target = 2L; value = 1. } ] )
      }
    ; { version = 2L
      ; bar_backgrounds = []
      ; contents =
          Radar
            ( [ { id = 1L; label = "A"; maximum = 1. }
              ; { id = 2L; label = "B"; maximum = 1. }
              ; { id = 3L; label = "C"; maximum = 1. }
              ]
            , [ { id = 1L; name = "R"; values = [ 1L, 2.; 2L, 0.; 3L, 0. ] } ] )
      }
    ]
  in
  print_s
    [%sexp
      (List.map cases ~f:(fun wire -> Result.is_error (D.Expert.decode (raw wire)))
       : bool list)];
  [%expect {| (true true true true true true true true true) |}]
;;

let%expect_test "decoder rejects huge/truncated lengths, bad tags and aggregate budgets" =
  let list_count n =
    Bin_prot.Utils.bin_dump Bin_prot.Type_class.bin_writer_nat0 (Bin_prot.Nat0.of_int n)
    |> Bigstring.to_string
  in
  let malicious =
    [ "\002\000\033"
    ; "\002\000\001\000\001\001A" ^ list_count 100001
    ; "\002\001" ^ list_count 257
    ; "\002\255"
    ; "\002\001\001\001\001\255" ^ String.make 8 '\000'
    ]
  in
  print_s
    [%sexp
      (List.map malicious ~f:(fun bytes -> Result.is_error (D.Expert.decode bytes))
       : bool list)];
  let points count label =
    List.init count ~f:(fun i ->
      { W.Point.id = Int64.of_int (i + 1); x = Float.of_int i; y = Some 0.; label })
  in
  let series id points = { W.Series.id; name = "S"; points } in
  let oversized =
    { W.version = 2L
    ; bar_backgrounds = []
    ; contents =
        Cartesian
          [ Line (series 1L (points 50_000 "")); Line (series 2L (points 50_001 "")) ]
    }
  in
  let text =
    { W.version = 2L
    ; bar_backgrounds = []
    ; contents = Cartesian [ Line (series 1L (points 32_768 (String.make 256 'x'))) ]
    }
  in
  print_s
    [%sexp
      (Result.is_error (D.Expert.decode (raw oversized)) : bool)
    , (Result.is_error (D.Expert.decode (raw text)) : bool)];
  [%expect
    {|
    (true true true true true)
    (true true) |}]
;;
