open Core
module Extension = Gpuio.Extension

let%expect_test "opaque extension bytes use string encoding rather than integer lists" =
  let signal = Gpuio_protocol.Extension_wire.Signal.Data "\000\255\128" in
  let encoded =
    Bin_prot.Utils.bin_dump Gpuio_protocol.Extension_wire.Signal.bin_writer_t signal
    |> Bigstring.to_string
  in
  assert (String.equal encoded "\000\003\000\255\128");
  print_endline "opaque byte fixture: 000300ff80";
  [%expect {| opaque byte fixture: 000300ff80 |}]
;;

let%expect_test "qualified component schemas reject ambiguous identities" =
  let create name version fingerprint =
    Extension.Schema.create ~name ~version ~fingerprint |> Result.is_ok
  in
  let fingerprint = String.make 64 'a' in
  List.iter
    [ "example.counter", 1, fingerprint
    ; "counter", 1, fingerprint
    ; "example..counter", 1, fingerprint
    ; "Example.counter", 1, fingerprint
    ; "example.counter", 0, fingerprint
    ; "example.counter", 65536, fingerprint
    ; "example.counter", 1, String.make 64 'A'
    ; "example.counter", 1, "short"
    ]
    ~f:(fun (name, version, digest) ->
      print_s [%sexp (create name version digest : bool)]);
  [%expect
    {|
    true
    false
    false
    false
    false
    false
    false
    false
    |}]
;;

module Properties = struct
  type t =
    { amount : int
    ; label : string
    }
  [@@deriving bin_io, equal, sexp_of]

  let validate t =
    if t.amount >= 0 && t.amount <= 10 && String.length t.label <= 8
    then Ok ()
    else Or_error.error_string "invalid counter properties"
  ;;
end

let%expect_test "bounded typed codecs enforce exact bytes and domain invariants" =
  let codec =
    Extension.Codec.bin_prot ~max_bytes:16 Properties.bin_t ~validate:Properties.validate
    |> Or_error.ok_exn
  in
  let value : Properties.t = { amount = 7; label = "hi" } in
  let bytes = Extension.Codec.encode codec value |> Or_error.ok_exn in
  assert (String.equal bytes "\007\002hi");
  print_s [%sexp (Extension.Codec.decode codec bytes : Properties.t Or_error.t)];
  List.iter
    [ "\011\002hi"; bytes ^ "x"; "\007\003hi"; String.make 17 'x' ]
    ~f:(fun bytes ->
      print_s [%sexp (Extension.Codec.decode codec bytes |> Result.is_error : bool)]);
  print_s
    [%sexp
      (Extension.Codec.encode codec { value with amount = 11 } |> Result.is_error : bool)];
  [%expect
    {|
    (Ok ((amount 7) (label hi)))
    true
    true
    true
    true
    true
    |}]
;;

let%expect_test
    "oversize inputs never invoke a decoder and callback exceptions stay errors"
  =
  let called = ref false in
  let codec =
    Extension.Codec.create
      ~max_bytes:2
      ~encode:(fun () -> Ok "abc")
      ~decode:(fun _ ->
        called := true;
        failwith "package decoder failed")
    |> Or_error.ok_exn
  in
  assert (Result.is_error (Extension.Codec.decode codec "abc"));
  assert (not !called);
  assert (Result.is_error (Extension.Codec.decode codec "a"));
  assert !called;
  assert (Result.is_error (Extension.Codec.encode codec ()));
  print_endline "bounded decoder and exception containment passed";
  [%expect {| bounded decoder and exception containment passed |}]
;;

let%expect_test "typed definitions bound commands and reject malformed native events" =
  let schema =
    Extension.Schema.create
      ~name:"example.counter"
      ~version:1
      ~fingerprint:(String.make 64 'a')
    |> Or_error.ok_exn
  in
  let int_codec =
    Extension.Codec.bin_prot ~max_bytes:16 Int.bin_t ~validate:(fun _ -> Ok ())
    |> Or_error.ok_exn
  in
  let definition =
    Extension.Definition.create
      ~schema
      ~properties:int_codec
      ~commands:int_codec
      ~events:int_codec
    |> Or_error.ok_exn
  in
  let command = Extension.Command.create ~sequence:2L 9 |> Or_error.ok_exn in
  let instance =
    Extension.Instance.create definition ~generation:1L ~label:"Count" ~command 7
    |> Or_error.ok_exn
  in
  let wire = Extension.Instance.Expert.to_wire instance in
  assert (String.equal wire.properties "\007");
  assert (Int64.equal (Option.value_exn wire.command).sequence 2L);
  print_s
    [%sexp
      (Extension.Instance.Expert.event instance (Data "\008") : int Extension.Event.t)];
  print_s
    [%sexp
      (Extension.Instance.Expert.event instance (Data "\008x") : int Extension.Event.t)];
  print_s
    [%sexp
      (Extension.Instance.Expert.event instance (Command_completed 2L)
       : int Extension.Event.t)];
  assert (Result.is_error (Extension.Command.create ~sequence:0L ()));
  assert (
    Result.is_error (Extension.Instance.create definition ~generation:0L ~label:"Count" 7));
  let wide =
    Extension.Codec.bin_prot ~max_bytes:65536 Int.bin_t ~validate:(fun _ -> Ok ())
    |> Or_error.ok_exn
  in
  assert (
    Result.is_error
      (Extension.Definition.create
         ~schema
         ~properties:wide
         ~commands:wide
         ~events:int_codec));
  [%expect
    {|
    (Data 8)
    (Failed Invalid_event)
    (Command_completed 2)
    |}]
;;
