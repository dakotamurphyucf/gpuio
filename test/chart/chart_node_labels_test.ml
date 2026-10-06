open Core
module L = Gpuio.Chart_node_labels
module W = Gpuio_protocol.Chart_node_labels_wire

let ok = Or_error.ok_exn
let id n = Gpuio.Chart_data.Node_id.of_int64 (Int64.of_int n) |> ok
let node n lines = L.Node.create ~node:(id n) lines |> ok

let%expect_test "node labels resolve colors and have independent paired wire bytes" =
  let color = Gpuio.Color.rgba ~red:0 ~green:0 ~blue:0 ~alpha:7 |> ok in
  let theme = Gpuio.Theme.create [ "label", color ] |> ok in
  let line =
    L.Line.create ~color:(Gpuio.Color.token_exn "label") ~font_size:16. "Hi" |> ok
  in
  let labels = L.create [ node 9 [ line; L.Line.create "" |> ok ] ] |> ok in
  let wire = L.Expert.to_wire labels ~theme |> ok in
  assert (W.valid wire);
  assert (Result.is_error (L.Expert.to_wire labels ~theme:Gpuio.Theme.default));
  let bytes = Bin_prot.Utils.bin_dump W.bin_writer_t wire |> Bigstring.to_string in
  print_endline
    (String.to_list bytes
     |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
     |> String.concat);
  [%expect {| 0109020248690107010000000000003040000000 |}]
;;

let%expect_test "labels validate identities UTF-8 line bounds and total text" =
  let line = L.Line.create "label" |> ok in
  List.iter
    [ "\000"; "\n"; "\r"; "\t"; "\127"; "\255"; String.make 257 'a' ]
    ~f:(fun text -> assert (Result.is_error (L.Line.create text)));
  List.iter [ Float.nan; Float.infinity; 7.99; 32.01 ] ~f:(fun font_size ->
    assert (Result.is_error (L.Line.create ~font_size "label")));
  List.iter [ 8.; 32. ] ~f:(fun font_size ->
    assert (Result.is_ok (L.Line.create ~font_size "λ")));
  assert (Result.is_ok (L.Node.create ~node:(id 1) []));
  assert (Result.is_error (L.Node.create ~node:(id 1) (List.init 5 ~f:(fun _ -> line))));
  assert (Result.is_error (L.create [ node 1 []; node 1 [] ]));
  assert (Result.is_error (L.create (List.init 129 ~f:(fun i -> node (i + 1) []))));
  let full = L.Line.create (String.make 256 'a') |> ok in
  let maximum = List.init 128 ~f:(fun i -> node (i + 1) [ full ]) in
  let labels = L.create maximum |> ok in
  assert (W.valid (L.Expert.to_wire labels ~theme:Gpuio.Theme.default |> ok));
  let over = node 1 [ full; line ] :: List.tl_exn maximum in
  assert (Result.is_error (L.create over));
  let raw : W.Node.t =
    { node = 1L; lines = [ { text = "ok"; color = None; font_size = None } ] }
  in
  assert (not (W.valid [ raw; raw ]));
  assert (not (W.valid [ { raw with node = 0L } ]));
  assert (
    not
      (W.valid
         [ { raw with lines = [ { text = "ok"; color = Some (-1L); font_size = None } ] }
         ]));
  print_endline
    "bounded labels: empty suppression, typed IDs, UTF-8, fonts, theme and 32 KiB limit";
  [%expect
    {| bounded labels: empty suppression, typed IDs, UTF-8, fonts, theme and 32 KiB limit |}]
;;

let%expect_test "chart styles resolve and validate node overrides" =
  let color = Gpuio.Color.rgb_exn 0x123456 in
  let theme = Gpuio.Theme.create [ "node", color ] |> ok in
  let labels =
    L.create
      [ node 4 [ L.Line.create ~color:(Gpuio.Color.token_exn "node") "Caption" |> ok ]
      ; node 9 []
      ]
    |> ok
  in
  let style = Gpuio.Chart_style.create ~node_labels:labels ~theme () |> ok in
  let wire = Gpuio.Chart_style.Expert.to_wire style in
  assert (Int64.equal wire.version (-3L));
  assert (W.equal wire.node_labels (L.Expert.to_wire labels ~theme |> ok));
  assert (Result.is_error (Gpuio.Chart_style.create ~node_labels:labels ()));
  assert (
    Result.is_error
      (Gpuio.Chart_style.Expert.of_wire
         { wire with node_labels = wire.node_labels @ wire.node_labels }));
  print_endline
    "style resolves node tokens, preserves explicit hiding, rejects duplicate overrides";
  [%expect
    {| style resolves node tokens, preserves explicit hiding, rejects duplicate overrides |}]
;;
