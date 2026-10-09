open Core
open Gpuio
module D = Document.Style
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn

let hex value =
  Bin_prot.Utils.bin_dump W.Op.bin_writer_t value
  |> Bigstring.to_string
  |> String.to_list
  |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
  |> String.concat
;;

let%expect_test "document styling validates values and part ownership" =
  List.iter [ Float.nan; Float.infinity; -1.; 65. ] ~f:(fun n ->
    assert (Result.is_error (D.create ~paragraph_gap_rem:n ())));
  assert (
    Result.is_error (D.create ~colors:[ Link, Color.rgb_exn 0; Link, Color.rgb_exn 1 ] ()));
  assert (
    Result.is_error
      (D.Heading_sizes.create ~h1:0. ~h2:20. ~h3:18. ~h4:16. ~h5:14. ~h6:12.));
  assert (Result.is_error (D.Inline_code.create ~font_weight:0 ()));
  assert (Result.is_error (D.Inline_code.create ~fade_out:Float.nan ()));
  assert (Result.is_error (D.Underline.create ~thickness:33. ()));
  List.iter
    [ Style.Property.Height (Length.px_exn 50.)
    ; Overflow Scroll
    ; User_select false
    ; Visibility Hidden
    ; Position Absolute
    ]
    ~f:(fun property ->
      assert (Result.is_error (D.create ~code_block:(Style.create_exn [ property ]) ())));
  let good =
    D.create
      ~paragraph_gap_rem:0.
      ~heading_base_font_size:512.
      ~table_cell:
        (Style.create_exn
           [ Padding (Length.px_exn 8.)
           ; Border_width 1.
           ; Foreground (Color.token_exn "reader")
           ])
      ()
    |> ok
  in
  assert (Result.is_error (D.Expert.to_wire good ~theme:Theme.default));
  let theme = Theme.create [ "reader", Color.rgb_exn 0x334455 ] |> ok in
  ignore (D.Expert.to_wire good ~theme |> ok);
  print_endline
    "Bounds, duplicates and unsupported ownership reject; hidden part tokens resolve";
  [%expect
    {| Bounds, duplicates and unsupported ownership reject; hidden part tokens resolve |}]
;;

let%expect_test "style-only and theme-only updates preserve document publication" =
  let owner = Text_source.Expert.Owner.create () in
  let source =
    Text_source.Expert.handle
      ~owner
      (Gpuio_protocol.Resource_id.create ~slot:0L ~generation:1L |> ok)
  in
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let reconciler = Reconciler.create ~document_owner:owner window in
  let styling =
    D.create ~colors:[ Link, Color.token_exn "reader" ] ~paragraph_gap_rem:2. () |> ok
  in
  let config text_style =
    Document.Config.create ~source ~mode:Markdown ?text_style () |> ok
  in
  assert (
    Result.is_error (Document.Config.create ~source ~mode:Diff ~text_style:styling ()));
  let publish theme config =
    let update =
      Reconciler.prepare reconciler ~theme (Some (View.document config)) |> ok
    in
    let operations =
      match Reconciler.message update with
      | Some (W.Message.Apply t) -> t.operations
      | None -> []
      | Some _ -> assert false
    in
    Reconciler.accept reconciler update |> ok;
    operations
  in
  let theme n = Theme.create [ "reader", Color.rgb_exn n ] |> ok in
  ignore (publish (theme 0) (config None));
  let extract = function
    | [ W.Op.Set_document_text_style (node, Some style) ] -> node, style
    | _ -> assert false
  in
  let node, first = publish (theme 0) (config (Some styling)) |> extract in
  assert (List.is_empty (publish (theme 0) (config (Some styling))));
  let next, second = publish (theme 0xff00ff) (config (Some styling)) |> extract in
  assert (
    Gpuio_protocol.Node_id.equal node next && not (W.Document_style.equal first second));
  (match publish (theme 0) (config None) with
   | [ W.Op.Set_document_text_style (same, None) ] ->
     assert (Gpuio_protocol.Node_id.equal same node)
   | _ -> assert false);
  let value = D.Expert.to_wire D.default ~theme:Theme.default |> ok in
  assert (
    String.equal
      (hex (W.Op.Set_document_text_style (node, Some value)))
      "76000101000000000000000000000000000000");
  assert (String.equal (hex (W.Op.Set_document_text_style (node, None))) "76000100");
  print_endline
    "Op118 paired bytes; theme updates; explicit reset; no source republish; unchanged \
     silent";
  [%expect
    {| Op118 paired bytes; theme updates; explicit reset; no source republish; unchanged silent |}]
;;
