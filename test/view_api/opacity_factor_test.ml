open Core
open Gpuio
module W = Gpuio_protocol.Wire

let%expect_test "opacity factor has independent bytes and rejects ambiguous targets" =
  List.iter [ 0.; 0.5; 1. ] ~f:(fun value ->
    let target = Animation.Target.create [ Opacity_factor, value ] |> Or_error.ok_exn in
    let config = Animation.Config.create ~target () |> Or_error.ok_exn in
    let wire = Animation.Expert.to_wire config ~generation:1L |> Or_error.ok_exn in
    let field = List.hd_exn wire.targets in
    Bin_prot.Utils.bin_dump W.Animation.Target.bin_writer_t field
    |> Bigstring.to_string
    |> String.iter ~f:(fun byte -> printf "%02x" (Char.to_int byte));
    print_endline "");
  List.iter [ Float.nan; Float.infinity; Float.neg_infinity; -0.1; 1.1 ] ~f:(fun value ->
    assert (Result.is_error (Animation.Target.create [ Opacity_factor, value ])));
  List.iter
    [ [ Animation.Property.Opacity, 0.4; Opacity_factor, 0.5 ]
    ; [ Opacity_factor, 0.5; Opacity, 0.4 ]
    ; [ Opacity_factor, 0.5; Opacity_factor, 0.4 ]
    ]
    ~f:(fun fields -> assert (Result.is_error (Animation.Target.create fields)));
  [%expect
    {|
    0b0000000000000000
    0b000000000000e03f
    0b000000000000f03f
    |}]
;;

let%expect_test "opacity factor is negotiated with the paired native host" =
  let capability = Int64.shift_left 1L 51 in
  assert (Int64.equal (Int64.bit_and W.capabilities capability) capability);
  List.iter [ capability; W.capabilities ] ~f:(fun required ->
    W.Message.encode (Hello (W.version, required))
    |> Or_error.ok_exn
    |> String.iter ~f:(fun byte -> printf "%02x" (Char.to_int byte));
    print_endline "");
  [%expect
    {|
    0001fc0000000000000800
    0001fcffffffffffff1f00
    |}]
;;
