open Core
open Gpuio
open Gpuio_protocol

let%expect_test "border patterns have independent native field fixtures" =
  List.iter [ Style.Border_style.Solid; Dashed ] ~f:(fun pattern ->
    let styles =
      Style.create_exn [ Border_style pattern ]
      |> Style.Expert.to_wire ~theme:Theme.default
      |> Or_error.ok_exn
    in
    match styles with
    | [ Wire.Style.Fields [ field ] ] ->
      Bin_prot.Utils.bin_dump Wire.Field.bin_writer_t field
      |> Bigstring.to_string
      |> String.iter ~f:(fun byte -> printf "%02x" (Char.to_int byte));
      print_endline ""
    | _ -> assert false);
  [%expect
    {|
    4300
    4301
    |}]
;;

let%expect_test "border state replacement and unset preserve independent geometry" =
  let base =
    Style.create_exn [ Border_width 3.; Border_style Solid; Border_style Dashed ]
    |> fun t ->
    Style.with_state_exn t Hovered [ Border_style Solid ]
    |> fun t -> Style.with_state_exn t Focused [ Border_style Dashed ]
  in
  let show styles =
    Style.merge styles
    |> Style.Expert.to_wire ~theme:Theme.default
    |> Or_error.ok_exn
    |> [%sexp_of: Wire.Style.t list]
    |> print_s
  in
  show [ base ];
  show [ base; Style.unset Style.empty ~state:Hovered Border_style ];
  show [ base; Style.unset Style.empty Border_style ];
  show [ base; Style.create_exn [ Border_style Solid; Border_width 0. ] ];
  [%expect
    {|
    ((Fields
      ((Border_top_width 3) (Border_right_width 3) (Border_bottom_width 3)
       (Border_left_width 3) (Border_style 1)))
     (State 1 ((Border_style 1))) (State 2 ((Border_style 0))))
    ((Fields
      ((Border_top_width 3) (Border_right_width 3) (Border_bottom_width 3)
       (Border_left_width 3) (Border_style 1)))
     (State 1 ((Border_style 1))) (State 2 ()))
    ((Fields
      ((Border_top_width 3) (Border_right_width 3) (Border_bottom_width 3)
       (Border_left_width 3)))
     (State 1 ((Border_style 1))) (State 2 ((Border_style 0))))
    ((Fields
      ((Border_top_width 0) (Border_right_width 0) (Border_bottom_width 0)
       (Border_left_width 0) (Border_style 0)))
     (State 1 ((Border_style 1))) (State 2 ((Border_style 0))))
    |}]
;;

let%expect_test "border styles have a distinct paired host capability" =
  let capability = Int64.shift_left 1L 49 in
  assert (Int64.equal (Int64.bit_and Wire.capabilities capability) capability);
  List.iter [ capability; Wire.capabilities ] ~f:(fun required ->
    Wire.Message.encode (Hello (Wire.version, required))
    |> Or_error.ok_exn
    |> String.iter ~f:(fun byte -> printf "%02x" (Char.to_int byte));
    print_endline "");
  [%expect
    {|
    0001fc0000000000000200
    0001fcffffffffffff1f00
    |}]
;;
