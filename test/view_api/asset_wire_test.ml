open Core
module W = Gpuio_protocol.Wire
module A = W.Asset

let%expect_test "asset upload commands and replies agree with independent Rust fixtures" =
  let id = Gpuio_protocol.Resource_id.create ~slot:2L ~generation:3L |> Or_error.ok_exn in
  let requests =
    List.mapi
      [ A.Format.Png; Jpeg; Webp; Gif; Svg; Bmp; Tiff; Ico; Pnm ]
      ~f:(fun i format ->
        W.Message.Asset (Int64.of_int (128 + i), Begin (format, 16777216L)))
    @ [ Asset (140L, Append (id, 128L, "\000\255\128A"))
      ; Asset (141L, Finish id)
      ; Asset (142L, Release id)
      ]
  in
  let events =
    [ W.Event.Asset_response (128L, Begun id); Asset_response (140L, Ack) ]
    @ List.mapi
        [ A.Error.Closed
        ; Invalid_size
        ; Resource_limit
        ; Stale_handle
        ; Not_uploading
        ; Invalid_chunk
        ; Incomplete
        ; Not_ready
        ; Native_failure
        ]
        ~f:(fun i error -> W.Event.Asset_response (Int64.of_int (150 + i), Failed error))
  in
  Eio_main.run (fun env ->
    let read name = Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / name) |> String.strip in
    let unhex text =
      String.init
        (String.length text / 2)
        ~f:(fun i ->
          Char.of_int_exn (Int.of_string ("0x" ^ String.sub text ~pos:(2 * i) ~len:2)))
    in
    let expected =
      read "assets-v1-requests.hex" |> String.split_lines |> List.map ~f:unhex
    in
    assert (
      List.equal
        String.equal
        expected
        (List.map requests ~f:(fun request -> W.Message.encode request |> Or_error.ok_exn)));
    let bytes = read "assets-v1-events.hex" |> unhex in
    let actual =
      Bin_prot.Utils.bin_dump [%bin_writer: W.Event.t list] events |> Bigstring.to_string
    in
    assert (String.equal actual bytes);
    assert (List.equal W.Event.equal events (W.Event.decode bytes |> Or_error.ok_exn));
    for length = 0 to String.length bytes - 1 do
      assert (Result.is_error (W.Event.decode (String.prefix bytes length)))
    done;
    assert (Result.is_error (W.Event.decode (bytes ^ "\000"))));
  assert (Result.is_error (W.Message.encode (Asset (0L, Finish id))));
  assert (
    Result.is_error
      (W.Message.encode
         (Asset (1L, Append (id, 0L, String.make (A.max_chunk_bytes + 1) '\000')))));
  let invalid_reply =
    Bin_prot.Utils.bin_dump [%bin_writer: W.Event.t list] [ Asset_response (0L, Ack) ]
    |> Bigstring.to_string
  in
  assert (Result.is_error (W.Event.decode invalid_reply));
  [%expect {| |}]
;;
