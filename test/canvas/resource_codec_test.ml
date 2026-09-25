open Core
module Wire = Gpuio_protocol.Canvas_resource_wire

let%expect_test "canvas upload framing uses exact revisions and binary chunks" =
  let id = Gpuio_protocol.Resource_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  let requests =
    [ Wire.Request.Create
    ; Begin { id; base = 1L; revision = 2L; generation = 1L; bytes = 400L }
    ; Chunk (id, 2L, 0L, "\000\255")
    ; Publish (id, 2L)
    ; Abort (id, 2L)
    ; Release id
    ]
  in
  List.iter requests ~f:(fun request ->
    let bytes = Bin_prot.Utils.bin_dump Wire.Request.bin_writer_t request in
    let position = ref 0 in
    assert (Wire.Request.equal request (Wire.Request.bin_read_t bytes ~pos_ref:position));
    assert (!position = Bigstring.length bytes);
    let framed =
      Gpuio_protocol.Wire.Message.encode (Canvas (7L, request)) |> Or_error.ok_exn
    in
    assert (String.equal framed ("\013\007" ^ Bigstring.to_string bytes));
    print_endline
      (Bigstring.to_string bytes
       |> String.to_list
       |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
       |> String.concat));
  [%expect
    {|
    00
    010001010201fe9001
    02000102000200ff
    03000102
    04000102
    050001
    |}]
;;

let%expect_test "canvas responses and envelope limits agree with native" =
  let module Event = Gpuio_protocol.Wire.Event in
  List.iter
    [ "\001\039\007\000\000\001"; "\001\039\007\001"; "\001\039\007\002\010" ]
    ~f:(fun bytes ->
      print_s [%sexp (Event.decode bytes |> Or_error.ok_exn : Event.t list)]);
  let id = Gpuio_protocol.Resource_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  assert (Or_error.is_error (Gpuio_protocol.Wire.Message.encode (Canvas (0L, Create))));
  assert (
    Or_error.is_error
      (Gpuio_protocol.Wire.Message.encode
         (Canvas (1L, Chunk (id, 1L, 0L, String.make (Wire.max_chunk_bytes + 1) 'x')))));
  [%expect
    {|
    ((Canvas_response 7 (Created ((slot 0) (generation 1)))))
    ((Canvas_response 7 Ack))
    ((Canvas_response 7 (Failed Unavailable_image)))
    |}]
;;
