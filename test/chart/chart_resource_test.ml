open Core
module W = Gpuio_protocol.Chart_resource_wire

let%expect_test "application chart envelopes preserve tags and validate correlation" =
  let module Wire = Gpuio_protocol.Wire in
  let encoded = Wire.Message.encode (Chart (7L, Create)) |> Or_error.ok_exn in
  assert (String.equal encoded "\021\007\000");
  assert (Result.is_error (Wire.Message.encode (Chart (0L, Create))));
  let events = [ Wire.Event.Chart_response (7L, Ack) ] in
  let bytes = Bin_prot.Utils.bin_dump Wire.Event.bin_writer_t (List.hd_exn events) in
  assert (String.equal (Bigstring.to_string bytes) "\061\007\001");
  print_endline "message 21, event 61; nonpositive correlation rejected";
  [%expect {| message 21, event 61; nonpositive correlation rejected |}]
;;

let hex bytes =
  String.to_list bytes
  |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
  |> String.concat
;;

let%expect_test "resource requests and responses match independent Rust fixtures" =
  let id = Gpuio_protocol.Resource_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  let requests : W.Request.t list =
    [ Create
    ; Begin { id; base = 1L; revision = 2L; generation = 1L; bytes = 400L }
    ; Chunk (id, 2L, 0L, "\000\255")
    ; Publish (id, 2L)
    ; Abort (id, 2L)
    ; Release id
    ]
  in
  List.iter requests ~f:(fun request ->
    print_endline
      (Bin_prot.Utils.bin_dump W.Request.bin_writer_t request
       |> Bigstring.to_string
       |> hex));
  let errors : W.Error.t list =
    [ Closed
    ; Resource_limit
    ; Stale_handle
    ; Invalid_revision
    ; Invalid_range
    ; Incomplete
    ; Busy
    ; Not_ready
    ; Invalid_data
    ; Cancelled
    ; Native_failure
    ]
  in
  let responses =
    W.Response.[ Created id; Ack ] @ List.map errors ~f:(fun e -> W.Response.Failed e)
  in
  List.iter responses ~f:(fun response ->
    print_endline
      (Bin_prot.Utils.bin_dump W.Response.bin_writer_t response
       |> Bigstring.to_string
       |> hex));
  [%expect
    {| 
    00
    010001010201fe9001
    02000102000200ff
    03000102
    04000102
    050001
    000001
    01
    0200
    0201
    0202
    0203
    0204
    0205
    0206
    0207
    0208
    0209
    020a |}]
;;
