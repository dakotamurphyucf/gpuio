open! Core
open Expect_test_helpers_core
module Window_title = Gpuio_protocol.Window_title

let%expect_test "title validation preserves application text" =
  List.iter [ ""; "Agent — workspace"; "bad\000title" ] ~f:(fun text ->
    print_s [%sexp (Window_title.of_string text : Window_title.t Or_error.t)]);
  [%expect
    {|
    (Ok "")
    (Ok "Agent \226\128\148 workspace")
    (Error "Window title must not contain NUL bytes")
    |}]
;;

let%expect_test "titles use explicit value equality" =
  let first = Window_title.of_string "Conversation" |> Or_error.ok_exn in
  let second = Window_title.of_string "Conversation" |> Or_error.ok_exn in
  require [%here] (Window_title.equal first second);
  require_equal [%here] (module String) (Window_title.to_string first) "Conversation";
  [%expect ""]
;;
