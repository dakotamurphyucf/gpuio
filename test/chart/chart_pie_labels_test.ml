open Core
module P = Gpuio.Chart_pie_labels
module S = Gpuio.Chart_style
module W = Gpuio_protocol.Chart_style_wire

let ok = Or_error.ok_exn
let id n = Gpuio.Chart_data.Datum_id.of_int64 (Int64.of_int n) |> ok
let alpha n = Gpuio.Color.rgba ~red:0 ~green:0 ~blue:0 ~alpha:n |> ok

let%expect_test "pie caption and leader colors have paired bytes and theme resolution" =
  let theme = Gpuio.Theme.create [ "pie.line", alpha 7 ] |> ok in
  let entry =
    P.Entry.create
      ~slice:(id 7)
      ~text:"λ"
      ~line_color:(Gpuio.Color.token_exn "pie.line")
      ()
    |> ok
  in
  let pie_labels = P.create [ entry ] |> ok in
  assert (Result.is_error (S.create ~pie_labels ()));
  let wire =
    S.create ~theme ~pie_labels ~pie_label_line_color:(alpha 3) ()
    |> ok
    |> S.Expert.to_wire
  in
  assert (W.valid wire);
  let bytes = Bin_prot.Utils.bin_dump W.bin_writer_t wire |> Bigstring.to_string in
  print_endline
    (String.suffix bytes 10
     |> String.to_list
     |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
     |> String.concat);
  assert (
    Result.is_error (S.create ~pie_label_line_color:(Gpuio.Color.token_exn "missing") ()));
  assert (Result.is_error (P.create [ entry; entry ]));
  [%expect {| 01070102cebb01070103 |}]
;;

let%expect_test "caption collection bounds and hidden text preserve distinct IDs" =
  List.iter
    [ String.make 257 'x'; "\255"; "line\nline"; "\127" ]
    ~f:(fun text -> assert (Result.is_error (P.Entry.create ~slice:(id 1) ~text ())));
  let entries count text =
    List.init count ~f:(fun n -> P.Entry.create ~slice:(id (n + 1)) ~text () |> ok)
  in
  assert (Result.is_ok (P.create (entries 256 "")));
  assert (Result.is_error (P.create (entries 257 "")));
  assert (Result.is_ok (P.create (entries 128 (String.make 256 'x'))));
  assert (Result.is_error (P.create (entries 129 (String.make 256 'x'))));
  let wire = S.Expert.to_wire S.default in
  List.iter [ 0L; -1L ] ~f:(fun slice ->
    assert (
      not
        (W.valid { wire with pie_labels = [ { slice; text = None; line_color = None } ] })));
  List.iter [ -1L; 0x1_0000_0000L ] ~f:(fun color ->
    assert (not (W.valid { wire with pie_label_line_color = Some color }));
    assert (
      not
        (W.valid
           { wire with
             pie_labels = [ { slice = 1L; text = None; line_color = Some color } ]
           })));
  assert (not (W.valid { wire with version = -2L }));
  print_endline "256 distinct IDs; 32 KiB text; empty suppresses, omitted inherits";
  [%expect {| 256 distinct IDs; 32 KiB text; empty suppresses, omitted inherits |}]
;;
