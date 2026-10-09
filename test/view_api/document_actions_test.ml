open Core
open Gpuio
module A = Document.Actions
module P = Gpuio_protocol.Document_actions_wire
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn

let hex writer value =
  Bin_prot.Utils.bin_dump writer value
  |> Bigstring.to_string
  |> String.to_list
  |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
  |> String.concat
;;

let action ?(enabled = true) () =
  A.Action.create ~id:(A.Id.of_string "run" |> ok) ~label:"Run" ~enabled () |> ok
;;

let config ?(enabled = true) () =
  A.Config.create ~code:[ action ~enabled () ] ~copy_code:false () |> ok
;;

let event : P.Event.t =
  { config_epoch = 1L
  ; action = "run"
  ; source_revision = 7L
  ; source_generation = 2L
  ; source_range = None
  ; block = Code (None, "ok")
  ; activation =
      { source = Keyboard
      ; modifiers =
          { shift = false
          ; control = false
          ; alt = false
          ; command = false
          ; function_ = false
          }
      }
  }
;;

let%expect_test "document action bounds and paired bytes" =
  List.iter
    [ ""; "1run"; "bad id"; "世界"; String.make 65 'a' ]
    ~f:(fun id -> assert (Result.is_error (A.Id.of_string id)));
  List.iter
    [ ""; "bad\000label"; String.make 257 'a'; "\255" ]
    ~f:(fun label ->
      assert (Result.is_error (A.Action.create ~id:(A.Id.of_string "run" |> ok) ~label ())));
  assert (Result.is_error (A.Config.create ~code:[ action (); action () ] ()));
  assert (
    Result.is_error
      (A.Config.create
         ~code:
           (List.init 17 ~f:(fun i ->
              A.Action.create
                ~id:(A.Id.of_string (sprintf "a%d" i) |> ok)
                ~label:"Action"
                ()
              |> ok))
         ()));
  let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok in
  assert (
    String.equal
      (hex
         W.Op.bin_writer_t
         (W.Op.Set_document_actions
            (node, A.Expert.to_wire (config ()) ~epoch:1L ~observe:true)))
      "78000101010001010372756e0352756e0100");
  assert (
    String.equal (hex P.Event.bin_writer_t event) "010372756e0702000000026f6b010000000000");
  let actual = A.Expert.event_of_wire ~config:(config ()) event |> ok in
  assert (String.equal (A.Id.to_string actual.action) "run");
  assert (
    Result.is_error (A.Expert.event_of_wire ~config:(config ~enabled:false ()) event));
  List.iter
    [ { event with source_revision = 0L }
    ; { event with source_generation = 0L }
    ; { event with source_range = Some { start_byte = 4L; end_byte = 3L } }
    ; { event with block = Code (None, String.make 65537 'x') }
    ; { event with action = "missing" }
    ]
    ~f:(fun e -> assert (Result.is_error (A.Expert.event_of_wire ~config:(config ()) e)));
  print_endline
    "paired Op120/event bytes; bounded IDs, labels, lists and payloads; disabled and \
     unknown actions rejected";
  [%expect
    {| paired Op120/event bytes; bounded IDs, labels, lists and payloads; disabled and unknown actions rejected |}]
;;

let%expect_test "actions reconcile independently and fence old configurations" =
  let owner = Text_source.Expert.Owner.create () in
  let source_id = Gpuio_protocol.Resource_id.create ~slot:0L ~generation:1L |> ok in
  let source = Text_source.Expert.handle ~owner source_id in
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let reconciler = Reconciler.create ~document_owner:owner window in
  List.iter [ Document.Mode.Diff; Code Document.Language.ocaml ] ~f:(fun mode ->
    assert (Result.is_error (Document.Config.create ~source ~mode ~actions:(config ()) ())));
  let document actions =
    Document.Config.create ~source ~mode:Markdown ~actions () |> ok
  in
  assert (
    Result.is_error
      (Reconciler.prepare
         reconciler
         ~theme:Theme.default
         (Some (View.document (document (config ()))))));
  let publish actions tag =
    let update =
      Reconciler.prepare
        reconciler
        ~theme:Theme.default
        (Some (View.document ~on_action:(fun _ -> tag) (document actions)))
      |> ok
    in
    let ops =
      match Reconciler.message update with
      | None -> []
      | Some (W.Message.Apply t) -> t.operations
      | Some _ -> assert false
    in
    Reconciler.accept reconciler update |> ok;
    ops
  in
  let first = publish (config ()) "first" in
  let node, handler =
    List.find_map_exn first ~f:(function
      | W.Op.Create (n, _, _, Some h) -> Some (n, h)
      | _ -> None)
  in
  let dispatch e =
    Reconciler.dispatch
      reconciler
      (W.Event.Document_action (window, node, handler, 1L, source_id, e))
  in
  assert (Option.equal String.equal (dispatch event) (Some "first"));
  assert (List.is_empty (publish (config ()) "latest"));
  assert (Option.equal String.equal (dispatch event) (Some "latest"));
  (match publish (config ~enabled:false ()) "disabled" with
   | [ W.Op.Set_document_actions (same, c) ] ->
     assert (Gpuio_protocol.Node_id.equal same node && Int64.equal c.epoch 2L)
   | _ -> assert false);
  assert (Option.is_none (dispatch event));
  assert (Option.is_none (dispatch { event with config_epoch = 2L }));
  ignore (publish A.Config.default "cleared");
  ignore (publish (config ()) "reinstalled");
  assert (Option.is_none (dispatch event));
  assert (
    Option.equal
      String.equal
      (dispatch { event with config_epoch = 4L })
      (Some "reinstalled"));
  let removal = Reconciler.prepare reconciler ~theme:Theme.default None |> ok in
  Reconciler.accept reconciler removal |> ok;
  assert (Option.is_none (dispatch { event with config_epoch = 4L }));
  print_endline
    "mode and callback admission; unchanged silence/latest handler; epoch A/B/A and \
     disabled fences; unmount rejects";
  [%expect
    {| mode and callback admission; unchanged silence/latest handler; epoch A/B/A and disabled fences; unmount rejects |}]
;;
