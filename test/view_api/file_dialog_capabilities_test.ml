open Core
open Gpuio
module W = Gpuio_protocol.Wire

let%expect_test "capability modes, cardinality and independent wire fixtures" =
  let window =
    Gpuio_protocol.Window_id.create ~slot:2L ~generation:3L |> Or_error.ok_exn
  in
  let cases : W.File_dialog.Capabilities.t list =
    [ { files = Multiple
      ; directories = Multiple
      ; files_and_directories = Multiple
      ; save = true
      }
    ; { files = Multiple
      ; directories = Unsupported
      ; files_and_directories = Unsupported
      ; save = true
      }
    ; { files = Multiple
      ; directories = Multiple
      ; files_and_directories = Unsupported
      ; save = true
      }
    ; { files = Single
      ; directories = Unsupported
      ; files_and_directories = Single
      ; save = false
      }
    ]
  in
  let events =
    List.mapi cases ~f:(fun i caps ->
      W.Event.File_dialog_result (Int64.of_int (140 + i), window, Capabilities caps))
  in
  Eio_main.run (fun env ->
    let read name =
      let hex = Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / name) |> String.strip in
      String.init
        (String.length hex / 2)
        ~f:(fun i ->
          Char.of_int_exn (Int.of_string ("0x" ^ String.sub hex ~pos:(i * 2) ~len:2)))
    in
    let request = W.Message.File_dialog (140L, window, Capabilities) in
    assert (
      String.equal
        (W.Message.encode request |> Or_error.ok_exn)
        (read "file-dialog-capabilities-v1-request.hex"));
    let bytes = read "file-dialog-capabilities-v1-events.hex" in
    assert (
      String.equal
        bytes
        (Bin_prot.Utils.bin_dump [%bin_writer: W.Event.t list] events
         |> Bigstring.to_string));
    assert (List.equal W.Event.equal events (W.Event.decode bytes |> Or_error.ok_exn));
    for length = 0 to String.length bytes - 1 do
      assert (Result.is_error (W.Event.decode (String.prefix bytes length)))
    done;
    List.iter
      [ 7, 4; 8, 3; 11, 2 ]
      ~f:(fun (index, value) ->
        let invalid =
          String.mapi bytes ~f:(fun i byte ->
            if i = index then Char.of_int_exn value else byte)
        in
        assert (Result.is_error (W.Event.decode invalid))));
  List.iter cases ~f:(fun wire ->
    let caps =
      match File_dialog.Expert.capabilities_of_wire (Capabilities wire) with
      | Ok caps -> caps
      | Error error ->
        raise_s [%message "capability conversion" (error : File_dialog.Error.t)]
    in
    List.iter
      [ File_dialog.Open.Selection.Files, wire.files
      ; Directories, wire.directories
      ; Files_and_directories, wire.files_and_directories
      ]
      ~f:(fun (selection, support) ->
        let single, multiple =
          match support with
          | Unsupported -> false, false
          | Single -> true, false
          | Multiple -> true, true
        in
        assert (
          Bool.equal
            single
            (File_dialog.Capabilities.supports_open caps ~selection ~multiple:false));
        assert (
          Bool.equal
            multiple
            (File_dialog.Capabilities.supports_open caps ~selection ~multiple:true)));
    assert (Bool.equal wire.save (File_dialog.Capabilities.supports_save caps));
    let open_request =
      File_dialog.Request.Open (File_dialog.Open.create () |> Or_error.ok_exn)
    in
    assert (
      Result.is_error (File_dialog.Expert.result_of_wire open_request (Capabilities wire))));
  List.iter [ W.File_dialog.Result.Selected [ "/tmp/file" ]; Cancelled ] ~f:(fun result ->
    assert (Result.is_error (File_dialog.Expert.capabilities_of_wire result)));
  print_endline
    "capabilities: modes, cardinality, strict payload decoding and result-kind separation";
  [%expect
    {| capabilities: modes, cardinality, strict payload decoding and result-kind separation |}]
;;
