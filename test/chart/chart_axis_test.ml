open Core
module A = Gpuio.Chart_axis
module G = Gpuio.Chart_grid
module S = Gpuio.Chart_style

let ok = Or_error.ok_exn
let alpha n = Gpuio.Color.rgba ~red:0 ~green:0 ~blue:0 ~alpha:n |> ok

let category n =
  Gpuio.Chart_data.Category_id.of_int64 (Int64.of_int n) |> ok |> A.Tick_position.category
;;

let hex bytes =
  Bigstring.to_string bytes
  |> String.to_list
  |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
  |> String.concat
;;

let%expect_test "axis and grid bytes pair independently and resolve every theme color" =
  let theme = Gpuio.Theme.create [ "tick", alpha 7 ] |> ok in
  let tick =
    A.Tick.create
      ~position:(A.Tick_position.value 0.5 |> ok)
      ~text:"λ"
      ~color:(Gpuio.Color.token_exn "tick")
      ~font_size:16.
      ~align:Right
      ()
    |> ok
  in
  let inherited = A.Tick.create ~position:(category 9) ~text:"" () |> ok in
  let axis =
    A.create
      ~line:false
      ~position:0.25
      ~ticks:[ tick; inherited ]
      ~tick_count:4
      ~label_side:Before
      ~label_align:Center
      ~label_gap:2.
      ~label_width:90.
      ~font_size:12.
      ~line_width:2.
      ~line_color:(alpha 3)
      ~label_color:(alpha 4)
      ()
    |> ok
  in
  assert (Result.is_error (S.create ~x_axis:axis ()));
  let grid =
    G.create
      ~x:[ A.Tick_position.fraction 0.75 |> ok ]
      ~y:[]
      ~dashes:[ 3.; 2.; 1. ]
      ~width:2.
      ~color:(alpha 5)
      ()
    |> ok
  in
  let resolved = S.create ~theme ~x_axis:axis ~grid () |> ok |> S.Expert.to_wire in
  print_endline
    (Bin_prot.Utils.bin_dump Gpuio_protocol.Chart_axis_wire.bin_writer_t resolved.x_axis
     |> hex);
  print_endline
    (Bin_prot.Utils.bin_dump Gpuio_protocol.Chart_grid_wire.bin_writer_t resolved.grid
     |> hex);
  let token = Gpuio.Color.token_exn "missing" in
  List.iter
    [ A.create ~line_color:token () |> ok; A.create ~label_color:token () |> ok ]
    ~f:(fun axis -> assert (Result.is_error (S.create ~y_axis:axis ())));
  assert (Result.is_error (S.create ~grid:(G.create ~color:token () |> ok) ()));
  [%expect
    {|
    000101000000000000d03f010200000000000000e03f02cebb010701000000000000304003010900000000010401020100000000000000400100000000008056400000000000002840000000000000004001030104
    010102000000000000e83f01000300000000000008400000000000000040000000000000f03f00000000000000400105
    |}]
;;

let%expect_test "axis tick and grid bounds distinguish omission from empty lists" =
  List.iter [ Float.nan; Float.infinity; Float.neg_infinity; -0.01; 1.01 ] ~f:(fun n ->
    assert (Result.is_error (A.Tick_position.fraction n));
    assert (Result.is_error (A.create ~position:n ())));
  List.iter [ Float.nan; Float.infinity; 1.01e100; -1.01e100 ] ~f:(fun n ->
    assert (Result.is_error (A.Tick_position.value n)));
  let position = A.Tick_position.value 1. |> ok in
  List.iter
    [ "\255"; "\n"; "\127"; String.make 257 'x' ]
    ~f:(fun text -> assert (Result.is_error (A.Tick.create ~position ~text ())));
  List.iter [ Float.nan; Float.infinity; 7.99; 32.01 ] ~f:(fun font_size ->
    assert (Result.is_error (A.Tick.create ~position ~text:"t" ~font_size ()));
    assert (Result.is_error (A.create ~font_size ())));
  List.iter [ Float.nan; Float.infinity; -1.; 64.01 ] ~f:(fun label_gap ->
    assert (Result.is_error (A.create ~label_gap ())));
  List.iter [ 7.99; 256.01 ] ~f:(fun label_width ->
    assert (Result.is_error (A.create ~label_width ())));
  List.iter [ 0.49; 8.01; Float.nan ] ~f:(fun width ->
    assert (Result.is_error (A.create ~line_width:width ()));
    assert (Result.is_error (G.create ~width ())));
  List.iter [ 0.49; 128.01; Float.nan; Float.infinity ] ~f:(fun n ->
    assert (Result.is_error (G.create ~dashes:[ n ] ())));
  let tick = A.Tick.create ~position ~text:(String.make 256 'x') () |> ok in
  assert (
    Result.is_ok (A.create ~ticks:(List.init 64 ~f:(fun _ -> tick)) ~tick_count:64 ()));
  assert (Result.is_error (A.create ~ticks:(List.init 65 ~f:(fun _ -> tick)) ()));
  List.iter [ 1; 65 ] ~f:(fun tick_count ->
    assert (Result.is_error (A.create ~tick_count ())));
  assert (
    Result.is_ok
      (G.create
         ~x:(List.init 64 ~f:(fun _ -> position))
         ~dashes:(List.init 16 ~f:(fun _ -> 0.5))
         ()));
  assert (Result.is_error (G.create ~y:(List.init 65 ~f:(fun _ -> position)) ()));
  assert (Result.is_error (G.create ~dashes:(List.init 17 ~f:(fun _ -> 1.)) ()));
  let wire =
    S.create ~x_axis:(A.create ~ticks:[] () |> ok) ~grid:(G.create ~y:[] () |> ok) ()
    |> ok
    |> S.Expert.to_wire
  in
  assert (Option.is_some wire.x_axis.ticks);
  assert (Option.is_none wire.y_axis.ticks);
  assert (Option.is_none wire.grid.x);
  assert (Option.is_some wire.grid.y);
  assert (not (Gpuio_protocol.Chart_style_wire.valid { wire with version = -3L }));
  print_endline "bounded ticks, text, dimensions, dashes; explicit empty preserved";
  [%expect {| bounded ticks, text, dimensions, dashes; explicit empty preserved |}]
;;
