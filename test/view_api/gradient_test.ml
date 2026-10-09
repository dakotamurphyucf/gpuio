open Core
open Gpuio
module Wire = Gpuio_protocol.Wire

let fill background =
  let styles =
    Style.create_exn [ Background background ]
    |> Style.Expert.to_wire ~theme:Theme.default
    |> Or_error.ok_exn
  in
  match styles with
  | [ Wire.Style.Fields [ Background fill ] ] -> fill
  | _ -> assert false
;;

let%expect_test "explicit gradient spaces preserve legacy bytes and resolve theme colors" =
  let from = Color.rgb_exn 0xff0000, 0. in
  let to_ = Color.rgb_exn 0x0000ff, 1. in
  let legacy = Background.linear_gradient ~angle:90. ~from ~to_ |> Or_error.ok_exn in
  List.iter [ Background.Color_space.Srgb; Oklab ] ~f:(fun space ->
    let gradient =
      Background.linear_gradient_in space ~angle:90. ~from ~to_ |> Or_error.ok_exn
    in
    print_s
      [%sexp
        (space : Background.Color_space.t), (Background.equal legacy gradient : bool)];
    let bytes =
      Bin_prot.Utils.bin_dump Wire.Fill.bin_writer_t (fill gradient)
      |> Bigstring.to_string
    in
    print_endline (String.concat_map bytes ~f:(fun ch -> sprintf "%02x" (Char.to_int ch))));
  let missing =
    Background.linear_gradient_in
      Oklab
      ~angle:90.
      ~from:(Color.token_exn "no-such-color", 0.)
      ~to_
    |> Or_error.ok_exn
  in
  print_s
    [%sexp
      (Or_error.is_error
         (Style.create_exn [ Background missing ]
          |> Style.Expert.to_wire ~theme:Theme.default)
       : bool)];
  [%expect
    {|
    (Srgb true)
    01000000000080564000fcff0000ff00000000000000000000000000fdffff0000000000000000f03f
    (Oklab false)
    0201000000000080564000fcff0000ff00000000000000000000000000fdffff0000000000000000f03f
    true
    |}]
;;

let%expect_test "both spaces reject invalid angle and stop geometry" =
  let color = Color.rgb_exn 0xff0000 in
  let invalid =
    [ Float.nan, 0., 1.
    ; Float.infinity, 0., 1.
    ; -1., 0., 1.
    ; 361., 0., 1.
    ; 90., Float.nan, 1.
    ; 90., 0., Float.infinity
    ; 90., -0.1, 1.
    ; 90., 0., 1.1
    ; 90., 0.7, 0.2
    ]
  in
  List.iter [ Background.Color_space.Srgb; Oklab ] ~f:(fun space ->
    List.iter invalid ~f:(fun (angle, start, stop) ->
      assert (
        Or_error.is_error
          (Background.linear_gradient_in
             space
             ~angle
             ~from:(color, start)
             ~to_:(color, stop))));
    List.iter [ 0.; 360. ] ~f:(fun angle ->
      assert (
        Or_error.is_ok
          (Background.linear_gradient_in
             space
             ~angle
             ~from:(color, 0.5)
             ~to_:(color, 0.5)))));
  print_s [%sexp (List.length invalid : int)];
  [%expect {| 9 |}]
;;
