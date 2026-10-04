open Core
open Gpuio
module W = Gpuio_protocol.Document_wire

let%expect_test "HTML mode appends its wire tag and accepts reader presentation" =
  let owner = Text_source.Expert.Owner.create () in
  let id = Gpuio_protocol.Resource_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  let source = Text_source.Expert.handle ~owner id in
  let config =
    Document.Config.create ~source ~mode:Html ~max_lines:4 () |> Or_error.ok_exn
  in
  let wire = Document.Expert.to_wire config ~owner:(Some owner) ~asset_owner:None in
  assert (W.Mode.equal wire.mode Html);
  List.iter
    [ W.Mode.Markdown, "\000"; Code "ml", "\001\002ml"; Diff, "\002"; Html, "\003" ]
    ~f:(fun (mode, expected) ->
      let bytes =
        Bin_prot.Utils.bin_dump W.Mode.bin_writer_t mode |> Bigstring.to_string
      in
      assert (String.equal bytes expected));
  let window =
    Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn
  in
  let reconciler = Reconciler.create ~document_owner:owner window in
  ignore
    (Reconciler.prepare
       reconciler
       ~theme:Theme.default
       (Some (View.document ~on_preview:Fn.id config))
     |> Or_error.ok_exn);
  assert (
    Result.is_error
      (Document.Config.create ~source ~mode:Html ~layout:(Viewport 100.) ~max_lines:2 ()));
  print_endline "Existing tags preserved; HTML tag 3; Flow preview accepts observations";
  [%expect {| Existing tags preserved; HTML tag 3; Flow preview accepts observations |}]
;;
