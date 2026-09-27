open Core
module Wire = Gpuio_protocol.Desktop_wire

let encode writer value = Bin_prot.Utils.bin_dump writer value |> Bigstring.to_string

let%expect_test "desktop envelopes preserve correlation and pending signal tags" =
  let module Bridge = Gpuio_protocol.Wire in
  assert (
    String.equal
      (Bridge.Message.encode (Desktop (7L, Capabilities)) |> Or_error.ok_exn)
      "\019\007\001");
  assert (Result.is_error (Bridge.Message.encode (Desktop (0L, Capabilities))));
  assert (Result.is_error (Bridge.Message.encode (Desktop (7L, Register_scheme "UPPER"))));
  let events = Bridge.Event.decode "\002\057\007\000\058" |> Or_error.ok_exn in
  print_s [%sexp (events : Bridge.Event.t list)];
  assert (Result.is_error (Bridge.Event.decode "\001\057\000\000"));
  [%expect {| ((Desktop_response 7 Configured) Desktop_pending) |}]
;;

let%expect_test "desktop requests match independently defined Rust fixtures" =
  let cases : (Wire.Request.t * string) list =
    [ ( Configure { identifier = "com.example"; name = "Demo"; schemes = [ "gpuio" ] }
      , "\000\011com.example\004Demo\001\005gpuio" )
    ; Capabilities, "\001"
    ; Take_links, "\002"
    ; Activate true, "\003\001"
    ; Reveal_file "/tmp/\255", "\004\006/tmp/\255"
    ; Open_file "/a", "\005\002/a"
    ; Register_scheme "gpuio", "\006\005gpuio"
    ]
  in
  List.iter cases ~f:(fun (request, bytes) ->
    assert (Wire.Request.valid request);
    assert (String.equal (encode Wire.Request.bin_writer_t request) bytes));
  [%expect {| |}]
;;

let decode_response bytes =
  let pos_ref = ref 0 in
  let value = Wire.Response.bin_read_t (Bigstring.of_string bytes) ~pos_ref in
  assert (!pos_ref = String.length bytes);
  assert (Wire.Response.valid value);
  value
;;

let%expect_test "native batch preserves malformed input separately from overflow" =
  print_s [%sexp (decode_response "\002\002\013gpuio://doc/1\001%\002" : Wire.Response.t)];
  print_s [%sexp (decode_response "\005\004" : Wire.Response.t)];
  print_s [%sexp (decode_response "\004" : Wire.Response.t)];
  print_s [%sexp (decode_response "\001\001\000\001\001\001\000" : Wire.Response.t)];
  [%expect
    {|
    (Links ((links (gpuio://doc/1 %)) (dropped 2)))
    (Failed Unavailable)
    Registered
    (Capabilities
     ((incoming_links true) (runtime_registration false)
      (application_activation true) (file_reveal true) (file_open true)
      (document_metadata false)))
    |}]
;;

let%expect_test "identity and batch validation rejects malformed or over-budget values" =
  let valid_identity : Wire.Identity.t =
    { identifier = "com.example"; name = "Demo"; schemes = [] }
  in
  List.iter
    [ "example"; "com..app"; "com.-app"; "com.app-"; "com.App"; "com.a_b" ]
    ~f:(fun identifier ->
      assert (not (Wire.Identity.valid { valid_identity with identifier })));
  List.iter
    [ ""; "   "; "a\000b"; "a\nb"; "\255"; String.make 257 'x' ]
    ~f:(fun name -> assert (not (Wire.Identity.valid { valid_identity with name })));
  assert (not (Wire.Identity.valid { valid_identity with schemes = [ "a"; "a" ] }));
  assert (not (Wire.Request.valid (Reveal_file "relative")));
  assert (not (Wire.Request.valid (Register_scheme "UPPER")));
  let batch : Wire.Link_batch.t =
    { links = List.init 16 ~f:(fun _ -> String.make Wire.max_link_bytes 'x')
    ; dropped = Int64.max_value
    }
  in
  assert (Wire.Link_batch.valid batch);
  assert (not (Wire.Link_batch.valid { batch with links = "x" :: batch.links }));
  assert (not (Wire.Link_batch.valid { batch with dropped = -1L }));
  assert (not (Wire.Link_batch.valid { batch with links = List.init 65 ~f:(fun _ -> "") }));
  [%expect {| |}]
;;

let%expect_test "public identity preserves declared scheme order and normalizes schemes" =
  let schemes =
    List.map [ "My-App"; "Another" ] ~f:(fun s ->
      Gpuio.Deep_link.Scheme.of_string s |> Or_error.ok_exn)
  in
  let identity =
    Gpuio.Desktop.Identity.create
      ~identifier:"org.example.my-app"
      ~name:"Example 🎨"
      ~schemes
      ()
    |> Or_error.ok_exn
  in
  assert (String.equal (Gpuio.Desktop.Identity.name identity) "Example 🎨");
  print_s [%sexp (Gpuio.Desktop.Expert.identity_to_wire identity : Wire.Identity.t)];
  assert (
    Result.is_error
      (Gpuio.Desktop.Identity.create
         ~identifier:"org.example"
         ~name:"Demo"
         ~schemes:(schemes @ schemes)
         ()));
  [%expect
    {|
    ((identifier org.example.my-app) (name "Example \240\159\142\168")
     (schemes (my-app another)))
    |}]
;;
