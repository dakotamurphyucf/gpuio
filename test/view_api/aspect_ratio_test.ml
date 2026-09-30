open Core
open Gpuio
open Gpuio_protocol

let%expect_test "aspect ratio validates safe native floats and paired bytes" =
  List.iter [ 0.5; 1.; 2. ] ~f:(fun ratio ->
    match
      Style.create_exn [ Aspect_ratio ratio ]
      |> Style.Expert.to_wire ~theme:Theme.default
      |> Or_error.ok_exn
    with
    | [ Wire.Style.Fields [ field ] ] ->
      Bin_prot.Utils.bin_dump Wire.Field.bin_writer_t field
      |> Bigstring.to_string
      |> String.iter ~f:(fun byte -> printf "%02x" (Char.to_int byte));
      print_endline ""
    | _ -> assert false);
  List.iter [ 0.000001; 1.; 1_000_000. ] ~f:(fun ratio ->
    assert (Result.is_ok (Style.create [ Aspect_ratio ratio ])));
  List.iter
    [ Float.nan; Float.infinity; Float.neg_infinity; -1.; 0.; 1e-7; 1_000_001. ]
    ~f:(fun ratio -> assert (Result.is_error (Style.create [ Aspect_ratio ratio ])));
  [%expect
    {|
    44000000000000e03f
    44000000000000f03f
    440000000000000040
    |}]
;;

let%expect_test "aspect ratio replacement and state-local unset preserve dimensions" =
  let base =
    Style.create_exn [ Width (Length.px_exn 120.); Aspect_ratio 1.; Aspect_ratio 2. ]
    |> fun t -> Style.with_state_exn t Hovered [ Aspect_ratio 0.5 ]
  in
  List.iter
    [ base
    ; Style.merge [ base; Style.unset Style.empty ~state:Hovered Aspect_ratio ]
    ; Style.merge [ base; Style.unset Style.empty Aspect_ratio ]
    ]
    ~f:(fun t ->
      Style.Expert.to_wire t ~theme:Theme.default
      |> Or_error.ok_exn
      |> [%sexp_of: Wire.Style.t list]
      |> print_s);
  [%expect
    {|
    ((Fields ((Width (Px 120)) (Aspect_ratio 2))) (State 2 ((Aspect_ratio 0.5))))
    ((Fields ((Width (Px 120)) (Aspect_ratio 2))) (State 2 ()))
    ((Fields ((Width (Px 120)))) (State 2 ((Aspect_ratio 0.5))))
    |}]
;;

let%expect_test "aspect ratio capability matches the native host" =
  let capability = Int64.shift_left 1L 50 in
  assert (Int64.equal (Int64.bit_and Wire.capabilities capability) capability);
  List.iter [ capability; Wire.capabilities ] ~f:(fun required ->
    Wire.Message.encode (Hello (Wire.version, required))
    |> Or_error.ok_exn
    |> String.iter ~f:(fun byte -> printf "%02x" (Char.to_int byte));
    print_endline "");
  [%expect
    {|
    0001fc0000000000000400
    0001fcffffffffffff0700
    |}]
;;
