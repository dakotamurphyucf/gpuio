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
