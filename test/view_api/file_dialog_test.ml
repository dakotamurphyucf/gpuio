open Core
open Gpuio

let%expect_test "open config defaults and explicit platform-dependent selection" =
  let config = File_dialog.Open.create () |> Or_error.ok_exn in
  assert (File_dialog.Open.Selection.equal (File_dialog.Open.selection config) Files);
  assert (not (File_dialog.Open.multiple config));
  assert (Option.is_none (File_dialog.Open.directory config));
  assert (String.equal (File_dialog.Open.title config) "Open");
  assert (String.equal (File_dialog.Open.accept_label config) "Open");
  let directory = File_path.of_string "/tmp/\255" |> Or_error.ok_exn in
  let config =
    File_dialog.Open.create ~selection:Files_and_directories ~multiple:true ~directory ()
    |> Or_error.ok_exn
  in
  assert (
    File_dialog.Open.Selection.equal
      (File_dialog.Open.selection config)
      Files_and_directories);
  assert (File_dialog.Open.multiple config);
  assert (
    Option.equal File_path.equal (File_dialog.Open.directory config) (Some directory));
  [%expect {| |}]
;;

let%expect_test "dialog labels and filename hints have different validation rules" =
  let directory = File_path.of_string "/tmp" |> Or_error.ok_exn in
  List.iter
    [ ""; " "; "\000"; "\255"; String.make 4097 'a' ]
    ~f:(fun label ->
      assert (Result.is_error (File_dialog.Open.create ~title:label ()));
      assert (Result.is_error (File_dialog.Open.create ~accept_label:label ()));
      assert (
        Result.is_error
          (File_dialog.Save.create ~directory ~suggested_name:"file.txt" ~title:label ()));
      assert (
        Result.is_error
          (File_dialog.Save.create
             ~directory
             ~suggested_name:"file.txt"
             ~accept_label:label
             ())));
  List.iter
    [ ""; "."; ".."; "a/b"; "a\000b"; "\255"; String.make 256 'a' ]
    ~f:(fun suggested_name ->
      assert (Result.is_error (File_dialog.Save.create ~directory ~suggested_name ())));
  List.iter
    [ "a.sql.s"; ".hidden"; "λ.txt"; " "; String.make 255 'a' ]
    ~f:(fun suggested_name ->
      let config =
        File_dialog.Save.create ~directory ~suggested_name () |> Or_error.ok_exn
      in
      assert (String.equal (File_dialog.Save.suggested_name config) suggested_name);
      assert (File_path.equal (File_dialog.Save.directory config) directory));
  [%expect {| |}]
;;
