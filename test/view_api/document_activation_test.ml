open Core
open Gpuio
module W = Gpuio_protocol.Wire
module D = Gpuio_protocol.Document_wire

let ok = Or_error.ok_exn

let modifiers : Gpuio_protocol.Pointer_wire.Modifiers.t =
  { shift = false; control = false; alt = false; command = true; function_ = false }
;;

let activation : D.Activation.t = { source = Mouse Middle; modifiers }

let%expect_test "document input metadata and legacy provenance" =
  let event = Document.Expert.navigation (Link_activated ("x", activation)) |> ok in
  (match event with
   | Link { url; activation = Some value } ->
     assert (String.equal url "x");
     assert (Document.Activation.Source.equal value.source (Mouse Middle));
     assert value.modifiers.command
   | Link _ | Line _ -> assert false);
  (match Document.Expert.navigation (Link "x") |> ok with
   | Link { activation = None; url = _ } -> ()
   | Link _ | Line _ -> assert false);
  assert (
    Result.is_error
      (Document.Expert.navigation
         (Link_activated ("x", { activation with source = Keyboard }))));
  print_endline
    "Mouse button/modifiers retained; legacy metadata absent; malformed keyboard rejected";
  [%expect
    {| Mouse button/modifiers retained; legacy metadata absent; malformed keyboard rejected |}]
;;

let%expect_test "queued activation uses live callback and fences stale ownership" =
  let source_id = Gpuio_protocol.Resource_id.create ~slot:0L ~generation:1L |> ok in
  let owner = Text_source.Expert.Owner.create () in
  let source = Text_source.Expert.handle ~owner source_id in
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok in
  let handler = Gpuio_protocol.Handler_id.create ~slot:0L ~generation:1L |> ok in
  let reconciler = Reconciler.create ~document_owner:owner window in
  let config = Document.Config.create ~source ~mode:Markdown () |> ok in
  let publish callback =
    let update =
      Reconciler.prepare
        reconciler
        ~theme:Theme.default
        (Some (View.document config ~on_navigate:callback))
      |> ok
    in
    Reconciler.accept reconciler update |> ok
  in
  let callback label = function
    | Document.Navigation.Link { activation = Some value; url } ->
      assert value.modifiers.command;
      label ^ url
    | Link _ | Line _ -> assert false
  in
  publish (callback "first:");
  let event handler source =
    W.Event.Document_navigation
      (window, node, handler, 1L, source, 3L, Link_activated ("x", activation))
  in
  assert (
    Option.equal
      String.equal
      (Reconciler.dispatch reconciler (event handler source_id))
      (Some "first:x"));
  publish (callback "latest:");
  assert (
    Option.equal
      String.equal
      (Reconciler.dispatch reconciler (event handler source_id))
      (Some "latest:x"));
  let stale_handler = Gpuio_protocol.Handler_id.create ~slot:0L ~generation:2L |> ok in
  let stale_source = Gpuio_protocol.Resource_id.create ~slot:0L ~generation:2L |> ok in
  assert (Option.is_none (Reconciler.dispatch reconciler (event stale_handler source_id)));
  assert (Option.is_none (Reconciler.dispatch reconciler (event handler stale_source)));
  let update = Reconciler.prepare reconciler ~theme:Theme.default None |> ok in
  Reconciler.accept reconciler update |> ok;
  assert (Option.is_none (Reconciler.dispatch reconciler (event handler source_id)));
  print_endline "Latest callback; stale handler/source and unmounted node rejected";
  [%expect {| Latest callback; stale handler/source and unmounted node rejected |}]
;;
