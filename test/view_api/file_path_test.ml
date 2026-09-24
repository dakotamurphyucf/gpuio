open Core
open Gpuio

let%expect_test "native paths preserve exact bytes without assuming UTF-8 or normalizing" =
  List.iter [ "/"; "//a/../b"; "/tmp/\255.txt" ] ~f:(fun bytes ->
    let path = File_path.of_string bytes |> Or_error.ok_exn in
    assert (String.equal (File_path.to_string path) bytes));
  List.iter [ ""; "a/b"; "~/file"; "/tmp/\000secret" ] ~f:(fun bytes ->
    assert (Result.is_error (File_path.of_string bytes)));
  let limit = "/" ^ String.make (File_path.max_bytes - 1) 'a' in
  assert (Result.is_ok (File_path.of_string limit));
  assert (Result.is_error (File_path.of_string (limit ^ "a")));
  [%expect {| |}]
;;

let%expect_test "native non-UTF8 path has the same raw string encoding in Rust" =
  let path = File_path.of_string "/tmp/\255.txt" |> Or_error.ok_exn in
  let encoded =
    Bin_prot.Utils.bin_dump String.bin_writer_t (File_path.to_string path)
    |> Bigstring.to_string
  in
  assert (String.equal encoded "\010/tmp/\255.txt");
  [%expect {| |}]
;;
