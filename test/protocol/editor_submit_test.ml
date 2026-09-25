open Core
open Gpuio_protocol

let%expect_test "native submit has an independent Rust fixture" =
  let window = Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  let node = Node_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  let bytes =
    Wire.Message.encode (Editor_command (7L, window, node, Submit)) |> Or_error.ok_exn
  in
  print_s [%sexp (String.to_list bytes |> List.map ~f:Char.to_int : int list)];
  [%expect {| (6 7 0 1 0 1 5) |}]
;;

let%expect_test "native read snapshot has an independent Rust fixture" =
  let window = Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  let node = Node_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  let bytes =
    Wire.Message.encode (Editor_command (7L, window, node, Read_snapshot))
    |> Or_error.ok_exn
  in
  print_s [%sexp (String.to_list bytes |> List.map ~f:Char.to_int : int list)];
  [%expect {| (6 7 0 1 0 1 6) |}]
;;
