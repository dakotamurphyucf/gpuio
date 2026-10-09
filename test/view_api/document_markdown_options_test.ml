open Core
open Gpuio
module M = Document.Markdown_options
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn

let hex value =
  Bin_prot.Utils.bin_dump W.Op.bin_writer_t value
  |> Bigstring.to_string
  |> String.to_list
  |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
  |> String.concat
;;

let%expect_test
    "parser options have paired bytes, mode admission and independent reconciliation"
  =
  let owner = Text_source.Expert.Owner.create () in
  let source =
    Text_source.Expert.handle
      ~owner
      (Gpuio_protocol.Resource_id.create ~slot:0L ~generation:1L |> ok)
  in
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let reconciler = Reconciler.create ~document_owner:owner window in
  let options = M.create ~frontmatter:Code_block ~mdx:true () in
  let descriptions = M.create ~frontmatter:Description_list () in
  assert (M.Frontmatter.equal (M.frontmatter descriptions) Description_list);
  assert (M.Frontmatter.equal (M.frontmatter options) Code_block && M.mdx options);
  List.iter [ Document.Mode.Html; Diff; Code Document.Language.ocaml ] ~f:(fun mode ->
    assert (
      Result.is_error (Document.Config.create ~source ~mode ~markdown_options:options ()));
    ignore (Document.Config.create ~source ~mode () |> ok));
  let publish markdown_options =
    let config =
      Document.Config.create ~source ~mode:Markdown ~markdown_options () |> ok
    in
    let update =
      Reconciler.prepare reconciler ~theme:Theme.default (Some (View.document config))
      |> ok
    in
    let operations =
      match Reconciler.message update with
      | None -> []
      | Some (W.Message.Apply t) -> t.operations
      | Some _ -> assert false
    in
    Reconciler.accept reconciler update |> ok;
    operations
  in
  ignore (publish M.default);
  let node =
    match publish options with
    | [ W.Op.Set_document_markdown_options (node, actual) ] ->
      assert (
        Gpuio_protocol.Document_wire.Markdown_options.equal
          actual
          (M.Expert.to_wire options));
      node
    | _ -> assert false
  in
  assert (List.is_empty (publish options));
  (match publish M.default with
   | [ W.Op.Set_document_markdown_options (same, actual) ] ->
     assert (Gpuio_protocol.Node_id.equal node same);
     assert (
       Gpuio_protocol.Document_wire.Markdown_options.equal
         actual
         (M.Expert.to_wire M.default))
   | _ -> assert false);
  assert (
    String.equal
      (hex (W.Op.Set_document_markdown_options (node, M.Expert.to_wire options)))
      "7700010101");
  assert (
    String.equal
      (hex (W.Op.Set_document_markdown_options (node, M.Expert.to_wire M.default)))
      "7700010000");
  assert (
    String.equal
      (hex (W.Op.Set_document_markdown_options (node, M.Expert.to_wire descriptions)))
      "7700010200");
  print_endline
    "Op119 paired bytes; only Markdown; option changes never republish source; same \
     silent; reset explicit";
  [%expect
    {| Op119 paired bytes; only Markdown; option changes never republish source; same silent; reset explicit |}]
;;
