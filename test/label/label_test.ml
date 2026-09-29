open Core
open Gpuio
module W = Gpuio_protocol.Text_content_wire

let ok = Or_error.ok_exn
let all text = Label.Match.all text |> ok
let prefix text = Label.Match.prefix text |> ok

let wire t =
  Label.Expert.to_text_content
    t
    ~secondary:(Color.rgb_exn 0x777777)
    ~highlight:(Color.rgb_exn 0x112233)
  |> Text_content.Expert.to_wire ~theme:Theme.default
  |> ok
;;

let ranges t =
  (wire t).spans
  |> List.map ~f:(fun span ->
    ( Int64.to_int_exn span.start_byte
    , Int64.to_int_exn span.end_byte
    , if Int64.equal span.foreground 0x112233ffL then "match" else "secondary" ))
;;

let show t = print_s [%sexp (ranges t : (int * int * string) list)]

let%expect_test "inline secondary and matches share a single source with match precedence"
  =
  let t =
    Label.create ~secondary:"alpha alphabet" ~highlight:(all "ALPHA") "Alpha" |> ok
  in
  print_endline (Label.display_text t);
  show t;
  show (Label.create ~secondary:"Alpha" ~highlight:(prefix "alpha") "Alpha" |> ok);
  show (Label.create ~secondary:"Alpha" ~highlight:(prefix "alpha") "Other" |> ok);
  show (Label.create ~secondary:"" "" |> ok);
  [%expect
    {|
    Alpha alpha alphabet
    ((0 5 match) (5 6 secondary) (6 11 match) (11 12 secondary) (12 17 match)
     (17 20 secondary))
    ((0 5 match) (5 11 secondary))
    ((5 11 secondary))
    ((0 1 secondary))
    |}]
;;

let%expect_test "overlaps, Unicode expansions and original offsets" =
  List.iter
    [ "aaaaa", all "aaa"
    ; "İX İ", all "i"
    ; "İX", prefix "i"
    ; "İX", all "\u{0307}"
    ; "É世界 é", all "é"
    ; "AΣ σς", all "σ"
    ; "Straße", all "SS"
    ; "e\u{0301}", all "é"
    ; "İ", all "i\u{0307}"
    ; "abc", all ""
    ]
    ~f:(fun (text, highlight) -> show (Label.create ~highlight text |> ok));
  [%expect
    {|
    ((0 5 match))
    ((0 2 match) (4 6 match))
    ((0 2 match))
    ((0 2 match))
    ((0 2 match) (9 11 match))
    ((1 3 match) (4 6 match))
    ()
    ()
    ((0 2 match))
    ()
    |}]
;;

let%expect_test "masking retains only replacement scalars and suppresses all formatting" =
  let secret = "private-İ👩‍💻" in
  let t =
    Label.create ~secondary:"suffix" ~highlight:(all "private") ~masked:true secret |> ok
  in
  let value = Label.display_text t in
  assert (String.equal value (String.concat (List.init 19 ~f:(fun _ -> "•"))));
  assert (List.is_empty (ranges t));
  let serialized =
    Bin_prot.Utils.bin_dump W.bin_writer_t (wire t) |> Bigstring.to_string
  in
  List.iter [ secret; "private"; "suffix" ] ~f:(fun source ->
    assert (not (String.is_substring serialized ~substring:source));
    assert (
      not (String.is_substring (Sexp.to_string (Label.sexp_of_t t)) ~substring:source)));
  assert (String.equal (Label.display_text (Label.create "" ~masked:true |> ok)) "");
  print_endline
    "19 bullets; no spans; source and query absent from value and serialized content";
  [%expect
    {| 19 bullets; no spans; source and query absent from value and serialized content |}]
;;

let%expect_test "source/query/output/run limits fail explicitly at their boundaries" =
  assert (Or_error.is_error (Label.create "\255"));
  assert (Or_error.is_error (Label.create ~secondary:"\255" "valid"));
  assert (Or_error.is_error (Label.Match.all "\255"));
  assert (Or_error.is_ok (Label.Match.all (String.make 4096 'x')));
  assert (Or_error.is_error (Label.Match.all (String.make 4097 'x')));
  assert (Or_error.is_ok (Label.create (String.make 262144 'x')));
  assert (Or_error.is_error (Label.create (String.make 262145 'x')));
  assert (Or_error.is_ok (Label.create ~secondary:"" (String.make 262143 'x')));
  assert (Or_error.is_error (Label.create ~secondary:"" (String.make 262144 'x')));
  assert (Or_error.is_ok (Label.create ~masked:true (String.make 87381 'x')));
  assert (Or_error.is_error (Label.create ~masked:true (String.make 87382 'x')));
  let many = String.concat (List.init 4096 ~f:(fun _ -> "x ")) in
  assert (List.length (ranges (Label.create ~highlight:(all "x") many |> ok)) = 4096);
  assert (Or_error.is_error (Label.create ~highlight:(all "x") (many ^ "x")));
  let secondary = String.concat (List.init 2048 ~f:(fun _ -> "x ")) in
  assert (Or_error.is_error (Label.create ~secondary ~highlight:(all "x") ""));
  (* Adjacent/overlapping matches collapse before counting the run budget. *)
  assert (
    List.length
      (ranges (Label.create ~highlight:(all "xx") (String.make 262144 'x') |> ok))
    = 1);
  print_endline "UTF-8, source, query, bullet expansion and final-run budgets checked";
  [%expect {| UTF-8, source, query, bullet expansion and final-run budgets checked |}]
;;
