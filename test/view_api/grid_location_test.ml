open Core
open Gpuio
module Wire = Gpuio_protocol.Wire
module G = Style.Grid_location

let line value = G.Edge.Line (G.Line.of_int_exn value)
let span value = G.Span.of_int_exn value
let wire location = Wire.Field.Grid_location (G.Expert.to_wire location)

let%expect_test "grid locations validate endpoints and match independent native bytes" =
  List.iter [ -1025; -1; 1; 1025 ] ~f:(fun value ->
    assert (G.Line.to_int (G.Line.of_int_exn value) = value));
  List.iter [ Int.min_value; -1026; 0; 1026; Int.max_value ] ~f:(fun value ->
    assert (Result.is_error (G.Line.of_int value)));
  List.iter [ 1; 1024 ] ~f:(fun value -> assert (G.Span.to_int (span value) = value));
  List.iter [ Int.min_value; -1; 0; 1025; Int.max_value ] ~f:(fun value ->
    assert (Result.is_error (G.Span.of_int value)));
  List.iter
    [ G.create ()
    ; G.create ~column:G.Axis.full ()
    ; G.create
        ~column:(G.Axis.span (span 2))
        ~row:(G.Axis.create ~start:(line 2) ~end_:(line 4))
        ()
    ]
    ~f:(fun location ->
      Bin_prot.Utils.bin_dump Wire.Field.bin_writer_t (wire location)
      |> Bigstring.to_string
      |> String.iter ~f:(fun byte -> printf "%02x" (Char.to_int byte));
      print_endline "");
  [%expect
    {|
    4600000000
    46010101ffff0000
    460202020201020104
    |}]
;;

let%expect_test "grid style replacement is atomic and unset is state-local" =
  let base = G.create ~column:G.Axis.full ~row:(G.Axis.span (span 3)) () in
  let replacement = G.create ~column:(G.Axis.span (span 2)) () in
  let styles =
    Style.create_exn [ Grid_location base ]
    |> fun t -> Style.with_state_exn t Hovered [ Grid_location replacement ]
  in
  let fields style = Style.Expert.to_wire style ~theme:Theme.default |> Or_error.ok_exn in
  let check actual expected = assert ([%equal: Wire.Style.t list] actual expected) in
  check (fields styles) [ Fields [ wire base ]; State (2L, [ wire replacement ]) ];
  check
    (fields
       (Style.merge [ styles; Style.unset Style.empty ~state:Hovered Grid_location ]))
    [ Fields [ wire base ]; State (2L, []) ];
  check
    (fields (Style.merge [ styles; Style.create_exn [ Grid_location replacement ] ]))
    [ Fields [ wire replacement ]; State (2L, [ wire replacement ]) ];
  check
    (fields (Style.merge [ styles; Style.unset Style.empty Grid_location ]))
    [ Fields []; State (2L, [ wire replacement ]) ];
  assert (G.Axis.equal (G.row replacement) G.Axis.auto);
  print_endline "atomic replacement; base and hover unset remain independent";
  [%expect {| atomic replacement; base and hover unset remain independent |}]
;;

let%expect_test "grid placement requires the matching native capability" =
  let capability = Int64.shift_left 1L 55 in
  assert (Int64.equal (Int64.bit_and Wire.capabilities capability) capability);
  List.iter [ capability; Wire.capabilities ] ~f:(fun required ->
    Wire.Message.encode (Hello (Wire.version, required))
    |> Or_error.ok_exn
    |> String.iter ~f:(fun byte -> printf "%02x" (Char.to_int byte));
    print_endline "");
  [%expect
    {|
    0003fc0000000000008000
    0003fcffffffffffffff7f
    |}]
;;
