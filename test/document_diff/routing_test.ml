open Core
open Gpuio
module Wire = Gpuio_protocol.Wire
module W = Gpuio_protocol.Document_diff_wire

let ok = Or_error.ok_exn
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok
let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok
let handler = Gpuio_protocol.Handler_id.create ~slot:0L ~generation:1L |> ok
let source_id = Gpuio_protocol.Resource_id.create ~slot:0L ~generation:1L |> ok
let owner = Text_source.Expert.Owner.create ()
let source = Text_source.Expert.handle ~owner source_id

let hex bytes =
  String.to_list bytes
  |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
  |> String.concat
;;

let sample ?(config_epoch = 1L) () : W.Event.t =
  { config_epoch
  ; source_revision = 1L
  ; source_generation = 1L
  ; observation =
      Line
        { file =
            { index = 0L; key = Path "x"; before_path = Some "x"; after_path = Some "x" }
        ; before = Some 1L
        ; after = Some 2L
        ; start_byte = 5L
        ; end_byte = 7L
        ; text = "λ"
        }
  }
;;

let event ?(epoch = 1L) ?(resource = source_id) ?(event_handler = handler) () =
  Wire.Event.Document_diff_event
    (window, node, event_handler, 1L, resource, sample ~config_epoch:epoch ())
;;

let%expect_test "live envelopes append op58/event65 without changing previous packets" =
  let config = Document.Diff.Expert.to_wire Document.Diff.Config.default in
  let packet =
    Wire.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Set_document_diff (node, 2L, Some config)
          ; Set_document_diff (node, 3L, None)
          ]
      }
  in
  print_endline (Wire.Message.encode packet |> ok |> hex);
  let observation : W.Event.t =
    { config_epoch = 2L
    ; source_revision = 7L
    ; source_generation = 3L
    ; observation = Show_more { visible = 0L; hidden = 7L; applied_limit = None }
    }
  in
  let events =
    [ Wire.Event.Document_diff_event (window, node, handler, 1L, source_id, observation) ]
  in
  let bytes =
    Bin_prot.Utils.bin_dump [%bin_writer: Wire.Event.t list] events |> Bigstring.to_string
  in
  print_endline (hex bytes);
  assert (List.equal Wire.Event.equal events (Wire.Event.decode bytes |> ok));
  for length = 0 to String.length bytes - 1 do
    assert (Result.is_error (Wire.Event.decode (String.prefix bytes length)))
  done;
  assert (Result.is_error (Wire.Event.decode (bytes ^ "\000")));
  [%expect
    {|
    0300010001023a0001020100000000fec800013a00010300
    014100010001000101000102070301000700
    |}]
;;

let%expect_test "navigation and diff share a handler with monotonic configuration epochs" =
  let r = Reconciler.create ~document_owner:owner window in
  let cfg ?diff ?(label = "Document") () =
    Document.Config.create ~source ~mode:Diff ?diff ~label () |> ok
  in
  let prepare ?(tag = "diff") ?(label = "Document") ?diff () =
    Reconciler.prepare
      r
      ~theme:Theme.default
      (Some
         (View.document
            (cfg ?diff ~label ())
            ~on_navigate:(fun _ -> "navigation")
            ?on_diff:(Option.map diff ~f:(fun _ _ -> tag))))
    |> ok
  in
  let accept update = Reconciler.accept r update |> ok in
  let has_epoch update epoch =
    match Reconciler.message update with
    | Some (Apply transaction) ->
      List.exists transaction.operations ~f:(function
        | Set_document_diff (id, value, _) ->
          Gpuio_protocol.Node_id.equal id node && Int64.equal value epoch
        | _ -> false)
    | Some _ | None -> false
  in
  let initial = prepare ~diff:Document.Diff.Config.default () in
  assert (has_epoch initial 1L);
  accept initial;
  assert (Option.equal String.equal (Reconciler.dispatch r (event ())) (Some "diff"));
  let nav =
    Wire.Event.Document_navigation
      (window, node, handler, 1L, source_id, 1L, Link "https://example.test")
  in
  assert (Option.equal String.equal (Reconciler.dispatch r nav) (Some "navigation"));
  let closure = prepare ~diff:Document.Diff.Config.default ~tag:"latest" () in
  assert (Option.is_none (Reconciler.message closure));
  accept closure;
  assert (Option.equal String.equal (Reconciler.dispatch r (event ())) (Some "latest"));
  let cosmetic = prepare ~diff:Document.Diff.Config.default ~label:"Renamed" () in
  assert (not (has_epoch cosmetic 2L));
  accept cosmetic;
  assert (Option.is_some (Reconciler.dispatch r (event ())));
  let changed = Document.Diff.Config.create ~word_diff:false () |> ok in
  let update = prepare ~diff:changed () in
  assert (has_epoch update 2L);
  accept update;
  assert (Option.is_none (Reconciler.dispatch r (event ())));
  assert (Option.is_some (Reconciler.dispatch r (event ~epoch:2L ())));
  assert (Option.is_some (Reconciler.dispatch r nav));
  let bad_handler = Gpuio_protocol.Handler_id.create ~slot:0L ~generation:2L |> ok in
  let bad_resource = Gpuio_protocol.Resource_id.create ~slot:0L ~generation:2L |> ok in
  assert (
    Option.is_none (Reconciler.dispatch r (event ~epoch:2L ~event_handler:bad_handler ())));
  assert (
    Option.is_none (Reconciler.dispatch r (event ~epoch:2L ~resource:bad_resource ())));
  let remove = prepare () in
  assert (has_epoch remove 3L);
  accept remove;
  assert (Option.is_none (Reconciler.dispatch r (event ~epoch:2L ())));
  assert (Option.is_some (Reconciler.dispatch r nav));
  let restore = prepare ~diff:changed () in
  assert (has_epoch restore 4L);
  accept restore;
  assert (Option.is_none (Reconciler.dispatch r (event ~epoch:2L ())));
  assert (Option.is_some (Reconciler.dispatch r (event ~epoch:4L ())));
  let discard = prepare ~diff:Document.Diff.Config.default () in
  assert (has_epoch discard 5L);
  assert (Option.is_some (Reconciler.dispatch r (event ~epoch:4L ())));
  assert (Result.is_error (Document.Config.create ~source ~mode:Markdown ~diff:changed ()));
  assert (
    Result.is_error
      (Reconciler.prepare
         r
         ~theme:Theme.default
         (Some (View.document (cfg ()) ~on_diff:(fun _ -> "invalid")))));
  accept (Reconciler.prepare r ~theme:Theme.default None |> ok);
  assert (Option.is_none (Reconciler.dispatch r (event ~epoch:4L ())));
  print_endline
    "shared handler, closure refresh, stable cosmetic epoch, clear/readd, source/handler \
     fences and teardown";
  [%expect
    {| shared handler, closure refresh, stable cosmetic epoch, clear/readd, source/handler fences and teardown |}]
;;
