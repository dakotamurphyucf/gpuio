open Core
open Gpuio
module W = Gpuio_protocol.Wire
module P = Gpuio_protocol.Document_preview_wire

let ok = Or_error.ok_exn

let%expect_test "preview limits reconcile independently and fence observations" =
  let owner = Text_source.Expert.Owner.create () in
  let source_id = Gpuio_protocol.Resource_id.create ~slot:0L ~generation:1L |> ok in
  let source = Text_source.Expert.handle ~owner source_id in
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let reconciler = Reconciler.create ~document_owner:owner window in
  let config ?(mode = Document.Mode.Markdown) ?(layout = Document.Layout.Flow) limit =
    Document.Config.create ~source ~mode ~layout ?max_lines:limit ()
  in
  List.iter [ 0; -1; 4097 ] ~f:(fun n -> assert (Result.is_error (config (Some n))));
  assert (Result.is_error (config ~mode:(Code Document.Language.ocaml) (Some 2)));
  assert (Result.is_error (config ~layout:(Viewport 200.) (Some 2)));
  let publish limit =
    let update =
      Reconciler.prepare
        reconciler
        ~theme:Theme.default
        (Some (View.document ~on_preview:Fn.id (config limit |> ok)))
      |> ok
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
  let first = publish (Some 2) in
  let node, handler =
    List.find_map_exn first ~f:(function
      | W.Op.Create (node, _, _, Some handler) -> Some (node, handler)
      | _ -> None)
  in
  let initial =
    List.find_map_exn first ~f:(function
      | W.Op.Set_document_preview (_, config) -> Some config
      | _ -> None)
  in
  assert (Int64.equal initial.epoch 1L && initial.observe);
  let event epoch state =
    W.Event.Document_preview_observed
      ( window
      , node
      , handler
      , 1L
      , source_id
      , { P.Event.config_epoch = epoch
        ; source_revision = 1L
        ; source_generation = 1L
        ; state
        } )
  in
  assert (Option.is_some (Reconciler.dispatch reconciler (event 1L (Rich true))));
  (match publish None with
   | [ Set_document_preview (same, config) ] ->
     assert (
       Gpuio_protocol.Node_id.equal same node
       && Int64.equal config.epoch 2L
       && Option.is_none config.max_lines)
   | _ -> assert false);
  assert (Option.is_none (Reconciler.dispatch reconciler (event 1L (Rich true))));
  assert (Option.is_some (Reconciler.dispatch reconciler (event 2L (Rich false))));
  assert (Option.is_none (Reconciler.dispatch reconciler (event 2L (Rich true))));
  assert (List.is_empty (publish None));
  let packet =
    W.Op.Set_document_preview
      (node, { P.Config.epoch = 1L; max_lines = Some 2L; observe = true })
  in
  let bytes = Bin_prot.Utils.bin_dump W.Op.bin_writer_t packet |> Bigstring.to_string in
  let hex =
    String.to_list bytes
    |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
    |> String.concat
  in
  assert (String.equal hex "75000101010201");
  print_endline
    "Op117 paired bytes; validated Markdown Flow limits; retained source; epoch fence; \
     unchanged silence";
  [%expect
    {| Op117 paired bytes; validated Markdown Flow limits; retained source; epoch fence; unchanged silence |}]
;;

let%expect_test "preview event has paired bytes and rejects invalid provenance" =
  let id = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok in
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let handler = Gpuio_protocol.Handler_id.create ~slot:0L ~generation:1L |> ok in
  let source = Gpuio_protocol.Resource_id.create ~slot:0L ~generation:1L |> ok in
  let value =
    { P.Event.config_epoch = 1L
    ; source_revision = 1L
    ; source_generation = 1L
    ; state = Rich true
    }
  in
  let event =
    W.Event.Document_preview_observed (window, id, handler, 1L, source, value)
  in
  let bytes = Bin_prot.Utils.bin_dump W.Event.bin_writer_t event in
  let hex =
    Bigstring.to_string bytes
    |> String.to_list
    |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
    |> String.concat
  in
  assert (String.equal hex "4c0001000100010100010101010301");
  assert (W.Event.equal event (W.Event.bin_read_t bytes ~pos_ref:(ref 0)));
  assert (
    Result.is_error
      (Document.Preview.Expert.event_of_wire { value with source_generation = -1L }));
  print_endline "Event76 paired bytes; source provenance validated";
  [%expect {| Event76 paired bytes; source provenance validated |}]
;;
