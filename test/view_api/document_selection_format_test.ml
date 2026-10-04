open Core
open Gpuio
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn

let%expect_test "copy format changes preserve the mounted source and clear explicitly" =
  let owner = Text_source.Expert.Owner.create () in
  let source =
    Text_source.Expert.handle
      ~owner
      (Gpuio_protocol.Resource_id.create ~slot:0L ~generation:1L |> ok)
  in
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let reconciler = Reconciler.create ~document_owner:owner window in
  let config selection_format =
    Document.Config.create ~source ~mode:Markdown ~selection_format () |> ok
  in
  let publish config =
    let update =
      Reconciler.prepare
        reconciler
        ~theme:Theme.default
        (Some (View.document ~key:(Key.of_string_exn "retained") config))
      |> ok
    in
    let operations =
      match Reconciler.message update with
      | Some (W.Message.Apply transaction) -> transaction.operations
      | None -> []
      | Some _ -> assert false
    in
    Reconciler.accept reconciler update |> ok;
    operations
  in
  let plain = config Plain_text in
  let markdown = config Markdown in
  assert (
    Document.Selection_format.equal (Document.Config.selection_format plain) Plain_text);
  assert (not (Document.Config.equal plain markdown));
  assert (not (List.is_empty (publish plain)));
  let first = publish markdown in
  let node =
    match first with
    | [ Set_document_selection_format (node, true) ] -> node
    | _ -> assert false
  in
  assert (List.is_empty (publish markdown));
  (match publish plain with
   | [ Set_document_selection_format (same, false) ] ->
     assert (Gpuio_protocol.Node_id.equal node same)
   | _ -> assert false);
  List.iter
    [ false, "74000100"; true, "74000101" ]
    ~f:(fun (flag, expected) ->
      let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok in
      let op = W.Op.Set_document_selection_format (node, flag) in
      let bytes = Bin_prot.Utils.bin_dump W.Op.bin_writer_t op |> Bigstring.to_string in
      let hex =
        String.to_list bytes
        |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
        |> String.concat
      in
      assert (String.equal hex expected));
  print_endline
    "Op116 paired bytes; retained node; no source republish; explicit reset; unchanged \
     is silent";
  [%expect
    {| Op116 paired bytes; retained node; no source republish; explicit reset; unchanged is silent |}]
;;
