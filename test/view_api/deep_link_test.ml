open Core
open Gpuio

let schemes = [ Deep_link.Scheme.of_string "Gpuio-Test" |> Or_error.ok_exn ]
let parse = Deep_link.of_string ~schemes

let%expect_test "application link parsing preserves routing and encoded data" =
  let text = "GPUIO-test://Document/a%2Fb/../%F0%9F%98%80?q=a+b&q=%26#L1?raw" in
  let link = parse text |> Result.ok |> Option.value_exn in
  assert (String.equal (Deep_link.to_string link) text);
  print_s
    [%sexp
      (Deep_link.scheme link : Deep_link.Scheme.t)
    , (Deep_link.route link : string)
    , (Deep_link.path link : string)
    , (Deep_link.query link : string option)
    , (Deep_link.fragment link : string option)];
  [%expect {| (gpuio-test Document /a%2Fb/../%F0%9F%98%80 (q=a+b&q=%26) (L1?raw)) |}]
;;

let%expect_test "malformed and unconfigured links cannot reach routing" =
  List.iter
    [ ""
    ; "/relative"
    ; "1bad://route"
    ; "other://route"
    ; "gpuio-test:opaque"
    ; "gpuio-test:///path"
    ; "gpuio-test://user@route/path"
    ; "gpuio-test://route:80"
    ; "gpuio-test://route/a b"
    ; "gpuio-test://route/\000"
    ; "gpuio-test://route/é"
    ; "gpuio-test://route/%"
    ; "gpuio-test://route/%0"
    ; "gpuio-test://route/%xz"
    ; "gpuio-test://route?x=%xx"
    ; "gpuio-test://route#x#y"
    ]
    ~f:(fun text ->
      match parse text with
      | Ok _ -> failwith "malformed link accepted"
      | Error error -> print_s [%sexp (error : Deep_link.Error.t)]);
  [%expect
    {|
    Invalid_scheme
    Invalid_scheme
    Invalid_scheme
    Unsupported_scheme
    Invalid_authority
    Invalid_authority
    Invalid_authority
    Invalid_authority
    Invalid_path
    Invalid_path
    Invalid_path
    Invalid_path
    Invalid_path
    Invalid_path
    Invalid_query
    Invalid_fragment
    |}]
;;

let%expect_test "empty components, byte limits and scheme validation" =
  let absent = parse "gpuio-test://route" |> Result.ok |> Option.value_exn in
  let present = parse "gpuio-test://route?#" |> Result.ok |> Option.value_exn in
  assert (Option.is_none (Deep_link.query absent));
  assert (Option.is_none (Deep_link.fragment absent));
  assert (String.is_empty (Deep_link.path absent));
  assert (Option.equal String.equal (Deep_link.query present) (Some ""));
  assert (Option.equal String.equal (Deep_link.fragment present) (Some ""));
  let prefix = "gpuio-test://route/" in
  let limit = prefix ^ String.make (Deep_link.max_bytes - String.length prefix) 'a' in
  assert (Result.is_ok (parse limit));
  assert (
    Result.equal
      Deep_link.equal
      Deep_link.Error.equal
      (parse (limit ^ "a"))
      (Error Too_long));
  List.iter
    [ ""; "0app"; "app_foo"; "a:b"; "é"; String.make 65 'a' ]
    ~f:(fun text -> assert (Result.is_error (Deep_link.Scheme.of_string text)));
  assert (Result.is_ok (Deep_link.Scheme.of_string (String.make 64 'a')));
  [%expect {| |}]
;;
