open Core
open Gpuio
module T = Text_content
module W = Gpuio_protocol.Text_content_wire

let ok = Or_error.ok_exn

let span start_byte end_byte =
  T.Span.create ~start_byte ~end_byte ~foreground:(Color.rgb_exn 0xabcdef) |> ok
;;

let fixture : W.t =
  { text = "Aé世界"
  ; spans =
      [ { start_byte = 1L; end_byte = 3L; foreground = 0x11223344L }
      ; { start_byte = 3L; end_byte = 9L; foreground = 0xaabbccddL }
      ]
  }
;;

let%expect_test "independent text-content bytes and concrete color round trip" =
  let content = T.Expert.of_wire fixture |> ok in
  let wire = T.Expert.to_wire content ~theme:Theme.default |> ok in
  assert (W.equal fixture wire);
  let bytes = Bin_prot.Utils.bin_dump W.bin_writer_t wire in
  let pos_ref = ref 0 in
  let decoded = W.bin_read_t bytes ~pos_ref in
  assert (!pos_ref = Bigstring.length bytes && W.equal decoded fixture);
  Bigstring.to_string bytes
  |> String.iter ~f:(fun byte -> printf "%02x" (Char.to_int byte));
  print_endline "";
  [%expect {| 0941c3a9e4b896e7958c020103fd443322110309fcddccbbaa00000000 |}]
;;

let%expect_test "text spans bind sorted byte ranges to actual scalar boundaries" =
  List.iter
    [ -1, 1; 0, 0; 2, 1; 0, 262145; Int.max_value, Int.max_value ]
    ~f:(fun (start_byte, end_byte) ->
      assert (
        Or_error.is_error
          (T.Span.create ~start_byte ~end_byte ~foreground:(Color.rgb_exn 0))));
  List.iter
    [ [ span 2 3 ]
    ; [ span 1 2 ]
    ; [ span 1 10 ]
    ; [ span 3 9; span 1 3 ]
    ; [ span 1 6; span 3 9 ]
    ]
    ~f:(fun spans -> assert (Or_error.is_error (T.create ~spans "Aé世界")));
  let good = T.create ~spans:[ span 1 3; span 3 9 ] "Aé世界" |> ok in
  assert (String.equal (T.text good) "Aé世界");
  assert (List.length (T.spans good) = 2);
  assert (Or_error.is_error (T.create "\255"));
  assert (Or_error.is_error (T.create ~spans:[ span 0 1 ] ""));
  ignore (T.create "" |> ok : T.t);
  (* Emoji joining and combining marks are multiple scalars; spans must not split
     UTF-8 scalars, but may style individual scalars within a grapheme. *)
  ignore (T.create ~spans:[ span 0 4; span 4 7; span 7 11 ] "👩‍💻" |> ok : T.t);
  ignore (T.create ~spans:[ span 0 1; span 1 3 ] "é" |> ok : T.t);
  print_endline
    "numeric, text, order, overlap and scalar validation; gaps, adjacency, empty and \
     multiscalar graphemes admitted";
  [%expect
    {| numeric, text, order, overlap and scalar validation; gaps, adjacency, empty and multiscalar graphemes admitted |}]
;;

let%expect_test "bounds are enforced before conversion and wire values are revalidated" =
  let text = String.make W.max_text_bytes 'x' in
  let spans = List.init W.max_spans ~f:(fun i -> span i (i + 1)) in
  ignore (T.create ~spans text |> ok : T.t);
  assert (Or_error.is_error (T.create (text ^ "x")));
  assert (Or_error.is_error (T.create ~spans:(spans @ [ span 4096 4097 ]) text));
  List.iter
    [ Int64.min_value; -1L; 0x1_0000_0000L; Int64.max_value ]
    ~f:(fun foreground ->
      let wire =
        { fixture with spans = [ { start_byte = 1L; end_byte = 3L; foreground } ] }
      in
      assert (not (W.valid wire));
      assert (Or_error.is_error (T.Expert.of_wire wire)));
  let invalid =
    { fixture with
      spans = [ { start_byte = Int64.min_value; end_byte = 9L; foreground = 0L } ]
    }
  in
  assert (Or_error.is_error (T.Expert.of_wire invalid));
  print_endline
    "exact source/span limits pass; oversized and malformed wire values cannot enter the \
     public type";
  [%expect
    {| exact source/span limits pass; oversized and malformed wire values cannot enter the public type |}]
;;

let%expect_test "theme resolution updates colors without rewriting source or byte ranges" =
  let color = Color.token_exn "label-secondary" in
  let span = T.Span.create ~start_byte:1 ~end_byte:3 ~foreground:color |> ok in
  let content = T.create ~spans:[ span ] "Aé世界" |> ok in
  let empty = Theme.create [] |> ok in
  assert (Or_error.is_error (T.Expert.to_wire content ~theme:empty));
  List.iter [ 0x123456; 0xfedcba ] ~f:(fun rgb ->
    let theme = Theme.create [ "label-secondary", Color.rgb_exn rgb ] |> ok in
    let wire = T.Expert.to_wire content ~theme |> ok in
    assert (String.equal wire.text "Aé世界");
    let run = List.hd_exn wire.spans in
    assert (Int64.equal run.start_byte 1L && Int64.equal run.end_byte 3L);
    assert (Int64.equal run.foreground Int64.(bit_or (shift_left (of_int rgb) 8) 0xffL)));
  print_endline
    "missing token is recoverable; explicit theme changes preserve Unicode source and \
     run offsets";
  [%expect
    {| missing token is recoverable; explicit theme changes preserve Unicode source and run offsets |}]
;;
