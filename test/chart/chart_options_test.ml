open Core
module O = Gpuio.Chart_options
module W = Gpuio_protocol.Chart_options_wire

let%expect_test "reversed Cartesian directions append stable paired wire tags" =
  List.iter
    [ O.Orientation.Vertical, 0
    ; Horizontal, 1
    ; Vertical_reversed, 2
    ; Horizontal_reversed, 3
    ]
    ~f:(fun (orientation, tag) ->
      let options =
        O.create ~cartesian:(O.Cartesian.create ~orientation () |> Or_error.ok_exn) ()
      in
      let bytes =
        Bin_prot.Utils.bin_dump W.bin_writer_t (O.Expert.to_wire options)
        |> Bigstring.to_string
      in
      assert (Char.to_int bytes.[9] = tag);
      assert (
        O.equal options (O.Expert.of_wire (O.Expert.to_wire options) |> Or_error.ok_exn)));
  print_endline "vertical=0 horizontal=1 vertical_reversed=2 horizontal_reversed=3";
  [%expect {| vertical=0 horizontal=1 vertical_reversed=2 horizontal_reversed=3 |}]
;;

let%expect_test "default chart options match independent fixed-width wire fixture" =
  let wire = O.Expert.to_wire O.default in
  assert (O.equal O.default (O.Expert.of_wire wire |> Or_error.ok_exn));
  let bytes = Bin_prot.Utils.bin_dump W.bin_writer_t wire |> Bigstring.to_string in
  print_endline
    (String.to_list bytes
     |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
     |> String.concat);
  [%expect
    {| 010101010500000000009a9999999999e93f0000000000000000000000000000000001040101666666666666e63f0000000000003040000000000000284003000601 |}]
;;

let%expect_test "typed construction rejects invalid geometry and formatting bounds" =
  List.iter [ -1; 7; Int.max_value ] ~f:(fun decimals ->
    assert (Result.is_error (O.Number_format.fixed ~decimals));
    assert (Result.is_error (O.Number_format.scientific ~decimals));
    assert (Result.is_error (O.Number_format.percent ~decimals)));
  List.iter
    [ Float.nan; Float.infinity; Float.neg_infinity; -0.01; 1.01 ]
    ~f:(fun value ->
      assert (Result.is_error (O.Cartesian.create ~bar_width:value ()));
      assert (Result.is_error (O.Candlestick.create ~body_width:value ()));
      assert (Result.is_error (O.Pie.create ~inner_radius:value ()));
      assert (Result.is_error (O.Pie.create ~pad_angle:value ())));
  List.iter [ -1; 0; 1; 13; Int.max_value ] ~f:(fun ticks ->
    assert (Result.is_error (O.Axes.create ~ticks ())));
  List.iter [ -1; 0; 13; Int.max_value ] ~f:(fun levels ->
    assert (Result.is_error (O.Radar.create ~levels ())));
  List.iter [ -1; 33; Int.max_value ] ~f:(fun iterations ->
    assert (Result.is_error (O.Sankey.create ~iterations ())));
  List.iter [ -1.; 65.; Float.nan; Float.infinity ] ~f:(fun value ->
    assert (Result.is_error (O.Sankey.create ~node_width:value ()));
    assert (Result.is_error (O.Sankey.create ~node_padding:value ())));
  let wire = O.Expert.to_wire O.default in
  List.iter
    [ { wire with version = 2L }
    ; { wire with axes = { wire.axes with x_format = Fixed 7L } }
    ; { wire with pie = { wire.pie with inner_radius = Float.nan } }
    ; { wire with sankey = { wire.sankey with iterations = Int64.max_value } }
    ]
    ~f:(fun value -> assert (Result.is_error (O.Expert.of_wire value)));
  let custom =
    O.create
      ~axes:
        (O.Axes.create
           ~ticks:12
           ~x_format:(O.Number_format.fixed ~decimals:6 |> Or_error.ok_exn)
           ~y_format:(O.Number_format.percent ~decimals:0 |> Or_error.ok_exn)
           ()
         |> Or_error.ok_exn)
      ~cartesian:
        (O.Cartesian.create
           ~curve:Step_after
           ~orientation:Horizontal
           ~dots:true
           ~bar_width:1.
           ()
         |> Or_error.ok_exn)
      ~pie:
        (O.Pie.create ~inner_radius:0.95 ~pad_angle:0.2 ~labels:false ()
         |> Or_error.ok_exn)
      ~radar:(O.Radar.create ~levels:12 () |> Or_error.ok_exn)
      ~sankey:
        (O.Sankey.create
           ~node_width:64.
           ~node_padding:0.
           ~iterations:32
           ~alignment:Left
           ~scale:Sqrt
           ()
         |> Or_error.ok_exn)
      ()
  in
  assert (O.equal custom (O.Expert.of_wire (O.Expert.to_wire custom) |> Or_error.ok_exn));
  print_endline
    "all seven families share validated options; invalid decoded records are rejected";
  [%expect
    {| all seven families share validated options; invalid decoded records are rejected |}]
;;
