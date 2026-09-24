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

module W = Gpuio_protocol.Wire

let%expect_test "dialog results are validated against their original request" =
  let request =
    File_dialog.Request.Open (File_dialog.Open.create () |> Or_error.ok_exn)
  in
  List.iter
    [ []; [ "relative" ]; [ "/a"; "/b" ]; [ "/tmp/\000bad" ] ]
    ~f:(fun paths ->
      assert (Result.is_error (File_dialog.Expert.result_of_wire request (Selected paths))));
  assert (
    Result.is_ok (File_dialog.Expert.result_of_wire request (Selected [ "/tmp/\255" ])));
  assert (Result.is_ok (File_dialog.Expert.result_of_wire request Cancelled));
  let multiple =
    File_dialog.Request.Open (File_dialog.Open.create ~multiple:true () |> Or_error.ok_exn)
  in
  assert (
    Result.is_error
      (File_dialog.Expert.result_of_wire
         multiple
         (Selected (List.init 129 ~f:(fun _ -> "/a")))));
  [%expect {| |}]
;;

let%expect_test "independent file-dialog requests, outcomes and raw paths agree with Rust"
  =
  let window =
    Gpuio_protocol.Window_id.create ~slot:2L ~generation:3L |> Or_error.ok_exn
  in
  let directory = File_path.of_string "/tmp/\255" |> Or_error.ok_exn in
  let open_config =
    File_dialog.Open.create
      ~selection:Files_and_directories
      ~multiple:true
      ~title:"Open files"
      ~accept_label:"Choose"
      ~directory
      ()
    |> Or_error.ok_exn
  in
  let save_config =
    File_dialog.Save.create
      ~directory
      ~suggested_name:"report.sql.s"
      ~title:"Save report"
      ()
    |> Or_error.ok_exn
  in
  let requests =
    [ W.Message.File_dialog (128L, window, File_dialog.Expert.to_wire (Open open_config))
    ; File_dialog (129L, window, File_dialog.Expert.to_wire (Save save_config))
    ]
  in
  let events =
    [ W.Event.File_dialog_result (128L, window, Selected [ "/tmp/\255/report.txt" ])
    ; File_dialog_result (129L, window, Cancelled)
    ]
    @ List.mapi
        [ W.File_dialog.Error.Invalid_request
        ; Unsupported
        ; Busy
        ; Closed
        ; Not_ready
        ; Native_failure
        ; Limit_exceeded
        ]
        ~f:(fun i error ->
          W.Event.File_dialog_result (Int64.of_int (130 + i), window, Failed error))
  in
  Eio_main.run (fun env ->
    let read name = Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / name) |> String.strip in
    let unhex text =
      String.init
        (String.length text / 2)
        ~f:(fun i ->
          Char.of_int_exn (Int.of_string ("0x" ^ String.sub text ~pos:(i * 2) ~len:2)))
    in
    let expected =
      read "file-dialog-v1-requests.hex" |> String.split_lines |> List.map ~f:unhex
    in
    assert (
      List.equal
        String.equal
        expected
        (List.map requests ~f:(fun request -> W.Message.encode request |> Or_error.ok_exn)));
    let event_bytes = read "file-dialog-v1-events.hex" |> unhex in
    assert (
      String.equal
        event_bytes
        (Bin_prot.Utils.bin_dump [%bin_writer: W.Event.t list] events
         |> Bigstring.to_string));
    assert (List.equal W.Event.equal events (W.Event.decode event_bytes |> Or_error.ok_exn));
    for length = 0 to String.length event_bytes - 1 do
      assert (Result.is_error (W.Event.decode (String.prefix event_bytes length)))
    done);
  List.iter
    [ W.File_dialog.Result.Selected []
    ; Selected [ "relative" ]
    ; Selected [ "/tmp/\000bad" ]
    ]
    ~f:(fun result ->
      let bytes =
        Bin_prot.Utils.bin_dump
          [%bin_writer: W.Event.t list]
          [ File_dialog_result (128L, window, result) ]
        |> Bigstring.to_string
      in
      assert (Result.is_error (W.Event.decode bytes)));
  [%expect {| |}]
;;

let%expect_test "file-dialog result lengths are bounded before reading their payloads" =
  let window =
    Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn
  in
  let encode paths =
    Bin_prot.Utils.bin_dump
      [%bin_writer: W.Event.t list]
      [ File_dialog_result (1L, window, Selected paths) ]
    |> Bigstring.to_string
  in
  let nat0 value =
    Bin_prot.Utils.bin_dump
      Bin_prot.Type_class.bin_writer_nat0
      (Bin_prot.Nat0.of_int value)
    |> Bigstring.to_string
  in
  (* A tiny message claiming a huge list/path must fail without allocating that
     declared payload, even though its envelope is well below the byte cap. *)
  let prefix = String.drop_suffix (encode []) 1 in
  assert (Result.is_error (W.Event.decode (prefix ^ nat0 100_000_000)));
  assert (Result.is_error (W.Event.decode (prefix ^ nat0 1 ^ nat0 100_000_000)));
  List.iter
    [ List.init 129 ~f:(fun _ -> "/a")
    ; [ "/" ^ String.make 16_384 'a' ]
    ; List.init 17 ~f:(fun _ -> "/" ^ String.make 16_383 'a')
    ]
    ~f:(fun paths -> assert (Result.is_error (W.Event.decode (encode paths))));
  let boundary = List.init 16 ~f:(fun _ -> "/" ^ String.make 16_383 'a') in
  assert (Result.is_ok (W.Event.decode (encode boundary)));
  [%expect {| |}]
;;
