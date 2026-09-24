open Core
open Gpuio.Drag_and_drop
module Wire = Gpuio_protocol.Wire.Drag_and_drop

let file ?is_directory path =
  Gpuio.File_path.of_string path |> Or_error.ok_exn |> File.create ?is_directory
;;

let decode reader bytes =
  Or_error.try_with (fun () ->
    let buffer = Bigstring.of_string bytes in
    let pos_ref = ref 0 in
    let result = reader.Bin_prot.Type_class.read buffer ~pos_ref in
    if !pos_ref <> String.length bytes then failwith "trailing bytes";
    result)
;;

let encode writer value = Bin_prot.Utils.bin_dump writer value |> Bigstring.to_string

let%expect_test "independent binary fixtures match Rust, including raw byte payloads" =
  let kind = Custom_kind.of_string "k/v1" |> Or_error.ok_exn in
  let source label payload ~disabled ~allow_desktop_files =
    Source.create ~label ~payload ~disabled ~allow_desktop_files ()
    |> Or_error.ok_exn
    |> Expert.source_to_wire
  in
  let sources =
    [ source
        "S"
        (Payload.text "hi λ" |> Or_error.ok_exn)
        ~disabled:false
        ~allow_desktop_files:false
    ; source
        "F"
        (Payload.files
           [ file ~is_directory:false "/a\255"; file ~is_directory:true "/dir" ]
         |> Or_error.ok_exn)
        ~disabled:false
        ~allow_desktop_files:true
    ; source
        "C"
        (Payload.custom ~kind ~data:"\000\255\128A" |> Or_error.ok_exn)
        ~disabled:true
        ~allow_desktop_files:false
    ]
  in
  let target =
    Target.create ~label:"T" ~accept:[ Text; Files; Custom kind ] ()
    |> Or_error.ok_exn
    |> Expert.target_to_wire
  in
  Eio_main.run (fun env ->
    let read name = Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / name) |> String.strip in
    let unhex text =
      String.init
        (String.length text / 2)
        ~f:(fun i ->
          Char.of_int_exn (Int.of_string ("0x" ^ String.sub text ~pos:(i * 2) ~len:2)))
    in
    let fixtures =
      read "drag-drop-v1-sources.hex" |> String.split_lines |> List.map ~f:unhex
    in
    List.iter2_exn sources fixtures ~f:(fun source bytes ->
      assert (String.equal (encode Wire.Source.bin_writer_t source) bytes);
      assert (
        Wire.Source.equal source (decode Wire.Source.bin_reader_t bytes |> Or_error.ok_exn));
      for length = 0 to String.length bytes - 1 do
        assert (
          Result.is_error (decode Wire.Source.bin_reader_t (String.prefix bytes length)))
      done;
      assert (Result.is_error (decode Wire.Source.bin_reader_t (bytes ^ "\000")));
      let domain = Expert.payload_of_wire source.payload |> Or_error.ok_exn in
      assert (Wire.Payload.equal source.payload (Expert.payload_to_wire domain)));
    let bytes = read "drag-drop-v1-target.hex" |> unhex in
    assert (String.equal (encode Wire.Target.bin_writer_t target) bytes);
    assert (
      Wire.Target.equal target (decode Wire.Target.bin_reader_t bytes |> Or_error.ok_exn));
    for length = 0 to String.length bytes - 1 do
      assert (
        Result.is_error (decode Wire.Target.bin_reader_t (String.prefix bytes length)))
    done);
  [%expect {| |}]
;;

let%expect_test "wire readers reject allocation bombs and malformed whole values" =
  let bomb =
    encode Bin_prot.Type_class.bin_writer_nat0 (Bin_prot.Nat0.of_int 100_000_000)
  in
  let source payload = "\001S" ^ payload ^ "\000\000" in
  assert (Result.is_error (decode Wire.Source.bin_reader_t bomb));
  List.iter [ "\000"; "\001"; "\001\001"; "\002"; "\002\001k" ] ~f:(fun prefix ->
    assert (Result.is_error (decode Wire.Source.bin_reader_t (source (prefix ^ bomb)))));
  List.iter
    [ "\000\001\255"
    ; "\000\001\000"
    ; "\001\000"
    ; "\001\001\001a\000"
    ; "\001\001\001/\002"
    ; "\001\001\001/\001\002"
    ; "\002\001:\000"
    ; "\003"
    ]
    ~f:(fun payload ->
      assert (Result.is_error (decode Wire.Source.bin_reader_t (source payload))));
  assert (
    Result.is_error (decode Wire.Source.bin_reader_t "\001S\001\001\001/\000\000\001"));
  List.iter
    [ "\001T\000\000"
    ; "\001T\002\000\000\000"
    ; "\001T\001\003\000"
    ; "\001T\001\000\002"
    ; "\001T" ^ bomb
    ]
    ~f:(fun bytes -> assert (Result.is_error (decode Wire.Target.bin_reader_t bytes)));
  [%expect {| |}]
;;

let%expect_test "wire readers enforce aggregate bytes before reading further paths" =
  let encode_payload payload =
    encode
      Wire.Source.bin_writer_t
      { label = "S"; payload; disabled = false; allow_desktop_files = false }
  in
  let long = Wire.File.{ path = String.make 16_384 '/'; is_directory = None } in
  assert (
    Result.is_ok
      (decode
         Wire.Source.bin_reader_t
         (encode_payload (Files (List.init 16 ~f:(fun _ -> long))))));
  assert (
    Result.is_error
      (decode
         Wire.Source.bin_reader_t
         (encode_payload (Files (List.init 17 ~f:(fun _ -> long))))));
  List.iter
    [ Wire.Payload.Text (String.make Payload.max_bytes 'a')
    ; Custom { kind = "k"; data = String.make Payload.max_bytes '\255' }
    ]
    ~f:(fun payload ->
      assert (Result.is_ok (decode Wire.Source.bin_reader_t (encode_payload payload))));
  (* Expert conversion also validates manually constructed wire data; decoding
     is not the only possible entry to the domain. *)
  List.iter
    [ Wire.Payload.Files []
    ; Files [ { path = "relative"; is_directory = None } ]
    ; Text "\255"
    ; Custom { kind = ":"; data = "" }
    ]
    ~f:(fun payload -> assert (Result.is_error (Expert.payload_of_wire payload)));
  [%expect {| |}]
;;

let%expect_test "native paths and opaque bytes retain their distinct meanings" =
  let files = [ file "//tmp/../\255"; file ~is_directory:false "/tmp/a" ] in
  let payload = Payload.files files |> Or_error.ok_exn in
  (match payload with
   | Files actual -> assert (List.equal File.equal actual files)
   | Text _ | Custom _ -> assert false);
  let kind = Custom_kind.of_string "com.example.task/v1" |> Or_error.ok_exn in
  let bytes = "\000\255\128A" in
  let payload = Payload.custom ~kind ~data:bytes |> Or_error.ok_exn in
  (match payload with
   | Custom { kind = actual_kind; data } ->
     assert (Custom_kind.equal actual_kind kind);
     assert (String.equal data bytes)
   | Text _ | Files _ -> assert false);
  assert (Payload.data_bytes payload = 4);
  [%expect {| |}]
;;

let%expect_test "payload bounds reject the whole value at count and byte limits" =
  assert (Result.is_ok (Payload.text ""));
  assert (Result.is_ok (Payload.text (String.make Payload.max_bytes 'a')));
  List.iter
    [ "a\000b"; "\255"; String.make (Payload.max_bytes + 1) 'a' ]
    ~f:(fun text -> assert (Result.is_error (Payload.text text)));
  assert (Result.is_error (Payload.files []));
  let short = file "/a" in
  assert (Result.is_ok (Payload.files (List.init 128 ~f:(fun _ -> short))));
  assert (Result.is_error (Payload.files (List.init 129 ~f:(fun _ -> short))));
  let long = file (String.make 16_384 '/') in
  let boundary = Payload.files (List.init 16 ~f:(fun _ -> long)) |> Or_error.ok_exn in
  assert (Payload.data_bytes boundary = Payload.max_bytes);
  assert (Result.is_error (Payload.files (List.init 17 ~f:(fun _ -> long))));
  let kind = Custom_kind.of_string "bytes" |> Or_error.ok_exn in
  assert (Result.is_ok (Payload.custom ~kind ~data:(String.make Payload.max_bytes '\255')));
  assert (
    Result.is_error
      (Payload.custom ~kind ~data:(String.make (Payload.max_bytes + 1) '\255')));
  [%expect {| |}]
;;

let%expect_test "desktop file offering requires metadata for every entry" =
  let known = file ~is_directory:false "/tmp/a" in
  let unknown = file "/tmp/b" in
  let payload = Payload.files [ known; unknown ] |> Or_error.ok_exn in
  assert (Result.is_ok (Source.create ~label:"Files" ~payload ()));
  assert (
    Result.is_error (Source.create ~label:"Files" ~payload ~allow_desktop_files:true ()));
  let payload = Payload.files [ known ] |> Or_error.ok_exn in
  assert (
    Result.is_ok (Source.create ~label:"Files" ~payload ~allow_desktop_files:true ()));
  let payload = Payload.text "x" |> Or_error.ok_exn in
  assert (
    Result.is_error (Source.create ~label:"Text" ~payload ~allow_desktop_files:true ()));
  [%expect {| |}]
;;

let%expect_test "targets accept exact formats and disabled always rejects" =
  let a = Custom_kind.of_string "example/a" |> Or_error.ok_exn in
  let b = Custom_kind.of_string "example/A" |> Or_error.ok_exn in
  let payload = Payload.custom ~kind:a ~data:"" |> Or_error.ok_exn in
  let target =
    Target.create ~label:"Target" ~accept:[ Text; Custom a ] () |> Or_error.ok_exn
  in
  assert (Target.accepts target payload);
  let target =
    Target.create ~label:"Target" ~accept:[ Text; Custom a ] ~disabled:true ()
    |> Or_error.ok_exn
  in
  assert (not (Target.accepts target payload));
  let target = Target.create ~label:"Target" ~accept:[ Custom b ] () |> Or_error.ok_exn in
  assert (not (Target.accepts target payload));
  List.iter
    [ []; [ Format.Files; Files ] ]
    ~f:(fun accept -> assert (Result.is_error (Target.create ~label:"Target" ~accept ())));
  let accept =
    List.init 17 ~f:(fun i ->
      Format.Custom (Custom_kind.of_string ("kind/" ^ Int.to_string i) |> Or_error.ok_exn))
  in
  assert (Result.is_ok (Target.create ~label:"Target" ~accept:(List.take accept 16) ()));
  assert (Result.is_error (Target.create ~label:"Target" ~accept ()));
  [%expect {| |}]
;;

let%expect_test "labels and custom kinds validate their separate vocabularies" =
  List.iter
    [ ""; "with space"; "λ"; "a\000b"; "a:b"; String.make 129 'x' ]
    ~f:(fun name -> assert (Result.is_error (Custom_kind.of_string name)));
  List.iter
    [ "application/x-a.v1+bytes"; "A_b/1"; String.make 128 'x' ]
    ~f:(fun name -> assert (Result.is_ok (Custom_kind.of_string name)));
  List.iter
    [ ""; " \t\n\r\011\012"; "a\000b"; "\255"; String.make 4097 'x' ]
    ~f:(fun label -> assert (Result.is_error (Target.create ~label ~accept:[ Text ] ())));
  List.iter
    [ "λ"; "\194\160"; String.make 4096 'x' ]
    ~f:(fun label -> assert (Result.is_ok (Target.create ~label ~accept:[ Text ] ())));
  [%expect {| |}]
;;
