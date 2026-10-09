open Core
open Gpuio
module Wire = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok
let key = Key.of_string_exn "public-label"
let color = Color.rgb_exn 0x010203

let appearance =
  Presentation.Appearance.create
    ~surface:color
    ~raised:color
    ~foreground:color
    ~muted:(Color.token_exn "label-muted")
    ~border:color
    ~on_solid:color
    ~accent:(Color.token_exn "label-match")
    ~success:color
    ~warning:color
    ~danger:color
;;

let theme muted matched =
  Theme.create
    [ "label-muted", Color.rgb_exn muted; "label-match", Color.rgb_exn matched ]
  |> ok
;;

let operations update =
  match Reconciler.message update with
  | Some (Wire.Message.Apply tx) -> tx.operations
  | None -> []
  | Some _ -> assert false
;;

let commit t theme view =
  let update = Reconciler.prepare t ~theme (Some view) |> ok in
  Reconciler.accept t update |> ok;
  operations update
;;

let content operations =
  List.find_map_exn operations ~f:(function
    | Wire.Op.Set_styled_text (node, content) -> Some (node, content)
    | _ -> None)
;;

let%expect_test
    "public helper resolves appearance, preserves Text identity and submits only masked \
     source"
  =
  let highlight = Label.Match.all "ALPHA" |> ok in
  let config = Label.create ~secondary:"alpha" ~highlight "Alpha" |> ok in
  let view = Presentation.styled_label appearance ~key config in
  let description = View.Expert.describe view in
  assert (List.is_empty description.children);
  assert (View.Expert.Kind.equal description.kind Text);
  assert (String.equal description.text "Alpha alpha");
  assert (Option.is_none (View.Expert.describe (Presentation.label "plain")).text_content);
  let light = theme 0x777777 0x112233
  and dark = theme 0xaaaaaa 0xaabbcc in
  let t = Reconciler.create window in
  let node, initial = commit t light view |> content in
  assert (
    List.equal
      Int64.equal
      (List.map initial.spans ~f:(fun span -> span.foreground))
      [ 0x112233ffL; 0x777777ffL; 0x112233ffL ]);
  let next = commit t dark view in
  assert (List.length next = 1);
  let same_node, updated = content next in
  assert (Gpuio_protocol.Node_id.equal node same_node);
  assert (
    List.equal
      Int64.equal
      (List.map updated.spans ~f:(fun span -> span.foreground))
      [ 0xaabbccffL; 0xaaaaaaffL; 0xaabbccffL ]);
  let masked = Label.create ~secondary:"alpha" ~highlight ~masked:true "Alpha" |> ok in
  let hidden = Presentation.styled_label appearance ~key masked in
  let same_node, wire = commit t dark hidden |> content in
  assert (Gpuio_protocol.Node_id.equal node same_node);
  assert (String.equal wire.text (Label.display_text masked));
  assert (List.is_empty wire.spans);
  assert (not (String.is_substring wire.text ~substring:"Alpha"));
  assert (List.is_empty (commit t dark hidden));
  print_endline
    "one Text leaf; appearance colors resolve; recolor and mask retain identity; masked \
     wire has only bullets";
  [%expect
    {| one Text leaf; appearance colors resolve; recolor and mask retain identity; masked wire has only bullets |}]
;;
