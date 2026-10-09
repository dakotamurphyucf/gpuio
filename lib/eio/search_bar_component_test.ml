open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module Input = Gpuio.Text_input
module S = Input.Search
module C = Search_bar_component
module W = Gpuio_protocol.Wire
module Driver = Gpuio_runtime_core.Window_driver

let ok = Or_error.ok_exn
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok
let source = Gpuio_protocol.Node_id.create ~slot:99L ~generation:1L |> ok

let search ?(mode = Gpuio_protocol.Editor_search_wire.Mode.Find) activation =
  S.Expert.snapshot_of_wire
    ~window
    ~node:source
    { stamp = { editor_revision = 0L; search_revision = Int64.(activation + 2L) }
    ; activation_revision = activation
    ; mode
    ; query = "one"
    ; case = Sensitive
    ; text_bytes = 11L
    ; match_count = 2L
    ; current = Some { index = 0L; byte_start = 0L; byte_end = 3L }
    ; can_replace = not (Gpuio_protocol.Editor_search_wire.Mode.equal mode Closed)
    }
  |> ok
;;

type edit_request =
  { expected : Input.Snapshot.t
  ; command : Input.Command.t
  ; complete : (Input.Snapshot.t, Input.Command_error.t) Result.t -> unit
  }

type harness =
  { driver : Driver.t
  ; target : S.Snapshot.t option B.Expert.Var.t
  ; document : Gpuio_protocol.Node_id.t option ref
  ; latest : C.t option ref
  ; edits : edit_request Queue.t
  ; searches :
      (S.Command.t * ((S.Response.t, Input.Command_error.t) Result.t -> unit)) Queue.t
  }

let cycle t = Driver.cycle t.driver ~now:Time_ns.epoch |> ok

let accept t =
  match Driver.next_message t.driver with
  | Some (Apply tx) ->
    Option.iter !(t.document) ~f:(fun document ->
      List.iter tx.operations ~f:(function
        | Remove node | Set_text (node, _) ->
          assert (not (Gpuio_protocol.Node_id.equal node document))
        | _ -> ()));
    Driver.submitted t.driver;
    Driver.acknowledge t.driver ~revision:tx.revision |> ok;
    Some tx
  | None -> None
  | Some _ -> assert false
;;

let create () =
  let target = B.Expert.Var.create None in
  let edits = Queue.create ()
  and searches = Queue.create ()
  and latest = ref None in
  let edit expected command =
    E.Expert.of_fun ~f:(fun ~callback ->
      Queue.enqueue edits { expected; command; complete = callback })
  in
  let component graph =
    let target =
      B.map (B.Expert.Var.value target) ~f:(fun search ->
        { C.search
        ; command =
            (fun command ->
              E.Expert.of_fun ~f:(fun ~callback ->
                Queue.enqueue searches (command, callback)))
        })
    in
    let bar = C.create edit ~target graph in
    let open B.Let_syntax in
    let%arr bar = bar in
    latest := Some bar;
    let content =
      Gpuio.View.text_input
        ~controller:(Gpuio.Key.of_string_exn "document")
        ~config:
          (Input.Config.create ~mode:Multiline ~searchable:true ~label:"Document" () |> ok)
        ~initial_text:"one two one"
        ~on_event:(fun _ -> E.Ignore)
        ()
      |> ok
    in
    C.wrap bar content
  in
  let driver =
    Driver.create window ~start:Time_ns.epoch ~theme:Gpuio.Theme.default component
  in
  let t = { driver; target; latest; edits; searches; document = ref None } in
  cycle t;
  let tx = accept t |> Option.value_exn in
  t.document
  := List.find_map tx.operations ~f:(function
       | Set_editor (node, config) when String.equal config.label "Document" -> Some node
       | _ -> None);
  assert (Option.is_some !(t.document));
  t
;;

let show t metadata =
  B.Expert.Var.set t.target (Some metadata);
  cycle t;
  accept t |> Option.value_exn
;;

let field tx label =
  let node =
    List.find_map_exn tx.W.Transaction.operations ~f:(function
      | Set_editor (id, config) when String.equal config.label label -> Some id
      | _ -> None)
  in
  let handler =
    List.find_map_exn tx.operations ~f:(function
      | Create (id, Textarea, _, Some handler) when Gpuio_protocol.Node_id.equal id node
        -> Some handler
      | _ -> None)
  in
  node, handler
;;

let wire ?(composition = None) revision text : W.Editor.Snapshot.t =
  { revision; text; selection = { anchor = 0L; head = 0L }; composition; focused = true }
;;

let observe t (node, handler) ?composition revision text =
  Driver.dispatch
    t.driver
    (Editor_event
       ( window
       , node
       , handler
       , Driver.revision t.driver
       , Changed
       , wire ?composition revision text ));
  cycle t;
  ignore (accept t : W.Transaction.t option)
;;

let resolve_edit t request ?composition revision text =
  let value =
    Input.Expert.snapshot_of_wire
      ~window
      ~node:(Input.Expert.node request.expected)
      (wire ?composition revision text)
    |> ok
  in
  request.complete (Ok value);
  cycle t;
  ignore (accept t : W.Transaction.t option)
;;

let invoke t name =
  let view =
    C.wrap (Option.value_exn !(t.latest)) (Gpuio_bonsai.View.text "Document content")
  in
  let commands = (Gpuio.View.Expert.describe view).commands |> Option.value_exn in
  let action =
    Gpuio.Command.Registry.find
      commands
      (Gpuio.Command.Id.of_string ("gpuio.search." ^ name) |> ok)
    |> Option.value_exn
    |> Gpuio.Command.Expert.invoke
    |> Option.value_exn
  in
  Driver.schedule t.driver action;
  cycle t;
  ignore (accept t : W.Transaction.t option)
;;

let mount ?mode t activation =
  let tx = show t (search ?mode activation) in
  let query = field tx "Find in editor"
  and replacement = field tx "Replace with" in
  observe t query 0L "one";
  observe t replacement 0L "";
  let initial = Queue.dequeue_exn t.edits in
  assert (
    match initial.command with
    | Replace
        { text; selection = Select selection; undo = Reset; if_revision = Some revision }
      ->
      String.equal text "one"
      && Int.equal (Input.Selection.head selection) 3
      && Int64.equal (Input.Revision.to_int64 revision) 0L
    | _ -> false);
  resolve_edit t initial 1L "one";
  let echoed, complete = Queue.dequeue_exn t.searches in
  assert (
    match echoed with
    | Set_query_text query -> String.equal (S.Query.to_string query) "one"
    | _ -> false);
  complete (Ok (Observed (search ?mode activation)));
  cycle t;
  ignore (accept t : W.Transaction.t option);
  query, replacement
;;

let%expect_test
    "one initial selection, composition-safe query updates and retired completion"
  =
  let t = create () in
  let query, _ = mount t 1L in
  assert (Queue.is_empty t.edits && Queue.is_empty t.searches);
  observe t query ~composition:(Some { anchor = 0L; head = 1L }) 2L "o";
  assert (Queue.is_empty t.searches);
  observe t query 3L "o";
  let command, complete = Queue.dequeue_exn t.searches in
  assert (
    match command with
    | Set_query_text query -> String.equal (S.Query.to_string query) "o"
    | _ -> false);
  assert (Queue.is_empty t.edits);
  ignore (show t (search ~mode:Closed 1L) : W.Transaction.t);
  let new_query, _ = mount t 2L in
  assert (not (Gpuio_protocol.Node_id.equal (fst query) (fst new_query)));
  complete (Error Native_failure);
  cycle t;
  assert (Option.is_none (accept t));
  assert (Queue.is_empty t.searches && Queue.is_empty t.edits);
  Driver.close t.driver;
  print_endline "select once; composition waits; reopening retires query and stale errors";
  [%expect {| select once; composition waits; reopening retires query and stale errors |}]
;;

let%expect_test
    "close rejects composition and stale async reads cannot close a new opening"
  =
  let t = create () in
  let _, _ = mount t 1L in
  invoke t "close";
  let query_read = Queue.dequeue_exn t.edits in
  assert (Input.Command.equal query_read.command Read_snapshot);
  resolve_edit t query_read ~composition:(Some { anchor = 0L; head = 1L }) 2L "one";
  assert (Queue.is_empty t.searches && Queue.is_empty t.edits);
  invoke t "find-replace";
  let query_read = Queue.dequeue_exn t.edits in
  resolve_edit t query_read ~composition:(Some { anchor = 0L; head = 1L }) 3L "one";
  assert (Queue.is_empty t.searches && Queue.is_empty t.edits);
  invoke t "close";
  let pending = Queue.dequeue_exn t.edits in
  ignore (show t (search ~mode:Closed 1L) : W.Transaction.t);
  ignore
    (mount t 2L
     : (Gpuio_protocol.Node_id.t * Gpuio_protocol.Handler_id.t)
       * (Gpuio_protocol.Node_id.t * Gpuio_protocol.Handler_id.t));
  resolve_edit t pending 3L "one";
  assert (Queue.is_empty t.searches && Queue.is_empty t.edits);
  invoke t "close";
  let query = Queue.dequeue_exn t.edits in
  resolve_edit t query 2L "one";
  let replacement = Queue.dequeue_exn t.edits in
  resolve_edit t replacement 1L "";
  let command, complete = Queue.dequeue_exn t.searches in
  assert (
    match command with
    | Close_and_focus value -> Int64.equal (S.Snapshot.activation_revision value) 2L
    | _ -> false);
  complete (Ok (Observed (search ~mode:Closed 2L)));
  cycle t;
  Driver.close t.driver;
  print_endline
    "composition preserved; retired read discarded; close binds current activation";
  [%expect
    {| composition preserved; retired read discarded; close binds current activation |}]
;;

let click t label =
  let rec find view =
    let description = Gpuio.View.Expert.describe view in
    match description.on_click with
    | Some action when String.equal description.text label -> Some (action ())
    | Some _ | None -> List.find_map description.children ~f:find
  in
  let view =
    C.wrap (Option.value_exn !(t.latest)) (Gpuio_bonsai.View.text "Document content")
  in
  Driver.schedule t.driver (find view |> Option.value_exn);
  cycle t;
  ignore (accept t : W.Transaction.t option)
;;

let%expect_test "replacement uses the displayed stamp and exact committed field values" =
  let t = create () in
  let query, _ = mount ~mode:Replace t 1L in
  click t "Replace";
  let query_read = Queue.dequeue_exn t.edits in
  resolve_edit t query_read 2L "two";
  let command, complete = Queue.dequeue_exn t.searches in
  assert (
    match command with
    | Set_query_text query -> String.equal (S.Query.to_string query) "two"
    | _ -> false);
  complete (Error Busy);
  cycle t;
  ignore (accept t : W.Transaction.t option);
  assert (Queue.is_empty t.searches && Queue.is_empty t.edits);
  click t "Replace";
  let query_read = Queue.dequeue_exn t.edits in
  resolve_edit t query_read 3L "one";
  let echoed, complete = Queue.dequeue_exn t.searches in
  assert (
    match echoed with
    | Set_query_text query -> String.equal (S.Query.to_string query) "one"
    | _ -> false);
  complete (Ok (Observed (search ~mode:Replace 1L)));
  cycle t;
  ignore (accept t : W.Transaction.t option);
  let replacement_read = Queue.dequeue_exn t.edits in
  resolve_edit t replacement_read ~composition:(Some { anchor = 0L; head = 2L }) 1L "é";
  assert (Queue.is_empty t.searches);
  click t "Replace all";
  let query_read = Queue.dequeue_exn t.edits in
  resolve_edit t query_read 4L "one";
  let replacement_read = Queue.dequeue_exn t.edits in
  resolve_edit t replacement_read 2L "é\n";
  let command, complete = Queue.dequeue_exn t.searches in
  assert (
    match command with
    | Replace_all { if_stamp; replacement } ->
      S.Stamp.equal if_stamp (S.Snapshot.stamp (search ~mode:Replace 1L))
      && String.equal replacement "é\n"
    | _ -> false);
  complete (Error Stale_search);
  cycle t;
  ignore (accept t : W.Transaction.t option);
  observe t query 5L (String.make 2049 'x');
  assert (Queue.is_empty t.searches);
  ignore (show t (search ~mode:Closed 1L) : W.Transaction.t);
  let reopened = show t (search ~mode:Replace 2L) in
  let replacement, _ = field reopened "Replace with" in
  assert (
    List.exists reopened.operations ~f:(function
      | Create (node, Textarea, text, _)
        when Gpuio_protocol.Node_id.equal node replacement -> String.equal text "é\n"
      | _ -> false));
  Driver.close t.driver;
  print_endline
    "pending query and composition reject; exact stamp/text sent; oversized query stays \
     local";
  [%expect
    {| pending query and composition reject; exact stamp/text sent; oversized query stays local |}]
;;

let%expect_test "typing back to the observed query still supersedes an in-flight edit" =
  let t = create () in
  let query, _ = mount t 1L in
  observe t query 2L "o";
  observe t query 3L "one";
  let first, fail_first = Queue.dequeue_exn t.searches in
  let second, accept_second = Queue.dequeue_exn t.searches in
  assert (
    match first, second with
    | Set_query_text a, Set_query_text b ->
      String.equal (S.Query.to_string a) "o" && String.equal (S.Query.to_string b) "one"
    | _ -> false);
  accept_second (Ok (Observed (search 1L)));
  fail_first (Error Busy);
  cycle t;
  ignore (accept t : W.Transaction.t option);
  assert (Queue.is_empty t.searches);
  let rec has_error view =
    let description = Gpuio.View.Expert.describe view in
    String.is_substring description.text ~substring:"could not complete"
    || List.exists description.children ~f:has_error
  in
  assert (
    not
      (has_error
         (C.wrap
            (Option.value_exn !(t.latest))
            (Gpuio_bonsai.View.text "Document content"))));
  Driver.close t.driver;
  print_endline "both committed edits delivered in order, including a return to baseline";
  [%expect {| both committed edits delivered in order, including a return to baseline |}]
;;

let%expect_test "search shortcut scopes preserve document newlines and composition" =
  let t = create () in
  let root () =
    C.wrap (Option.value_exn !(t.latest)) (Gpuio_bonsai.View.text "Document content")
  in
  let registry view =
    (Gpuio.View.Expert.describe view).commands
    |> Option.value_exn
    |> Gpuio.Command.Registry.to_list
  in
  assert (List.length (registry (root ())) = 2);
  ignore
    (mount t 1L
     : (Gpuio_protocol.Node_id.t * Gpuio_protocol.Handler_id.t)
       * (Gpuio_protocol.Node_id.t * Gpuio_protocol.Handler_id.t));
  let commands = registry (root ()) in
  assert (List.length commands = 5);
  let shortcuts commands =
    List.concat_map commands ~f:(fun command ->
      (Gpuio.Command.Expert.to_wire command ~generation:1L).shortcuts)
  in
  assert (
    not
      (List.exists (shortcuts commands) ~f:(fun (s : W.Shortcut.t) ->
         String.equal s.key "enter")));
  let rec scopes view =
    let description = Gpuio.View.Expert.describe view in
    Option.to_list description.commands @ List.concat_map description.children ~f:scopes
  in
  let all = scopes (root ()) |> List.map ~f:Gpuio.Command.Registry.to_list in
  let query =
    List.find_exn all ~f:(fun commands ->
      List.exists (shortcuts commands) ~f:(fun (s : W.Shortcut.t) ->
        String.equal s.key "enter"))
  in
  assert (List.length query = 2);
  List.iter (List.concat_map all ~f:shortcuts) ~f:(fun s ->
    assert (W.Shortcut_priority.equal s.priority Override);
    assert (W.Shortcut_text_input.equal s.text_input Always);
    assert (not s.during_composition));
  ignore (show t (search ~mode:Closed 1L) : W.Transaction.t);
  assert (List.length (registry (root ())) = 2);
  Driver.close t.driver;
  print_endline
    "open-only navigation; query-only Enter; all shortcuts defer during composition";
  [%expect
    {| open-only navigation; query-only Enter; all shortcuts defer during composition |}]
;;

let%expect_test "a duplicate query echo does not swallow an in-flight Next gesture" =
  let t = create () in
  ignore
    (mount t 1L
     : (Gpuio_protocol.Node_id.t * Gpuio_protocol.Handler_id.t)
       * (Gpuio_protocol.Node_id.t * Gpuio_protocol.Handler_id.t));
  invoke t "next";
  let read = Queue.dequeue_exn t.edits in
  resolve_edit t read 2L "two";
  let first, continue_next = Queue.dequeue_exn t.searches in
  let second, accept_echo = Queue.dequeue_exn t.searches in
  List.iter [ first; second ] ~f:(function
    | S.Command.Set_query_text query ->
      assert (String.equal (S.Query.to_string query) "two")
    | _ -> assert false);
  let value =
    S.Expert.snapshot_of_wire
      ~window
      ~node:source
      { stamp = { editor_revision = 0L; search_revision = 10L }
      ; activation_revision = 1L
      ; mode = Find
      ; query = "two"
      ; case = Sensitive
      ; text_bytes = 11L
      ; match_count = 1L
      ; current = Some { index = 0L; byte_start = 4L; byte_end = 7L }
      ; can_replace = true
      }
    |> ok
  in
  accept_echo (Ok (Observed value));
  B.Expert.Var.set t.target (Some value);
  cycle t;
  ignore (accept t : W.Transaction.t option);
  continue_next (Ok (Observed value));
  cycle t;
  ignore (accept t : W.Transaction.t option);
  let next, complete = Queue.dequeue_exn t.searches in
  assert (S.Command.equal next Next);
  complete (Ok (Observed value));
  cycle t;
  ignore (accept t : W.Transaction.t option);
  assert (Queue.is_empty t.searches && Queue.is_empty t.edits);
  Driver.close t.driver;
  print_endline "one Next reaches native search even when its query echo completes first";
  [%expect {| one Next reaches native search even when its query echo completes first |}]
;;

let%expect_test "coalesced typing before the first mount observation is never selected" =
  let t = create () in
  let tx = show t (search 1L) in
  let query = field tx "Find in editor"
  and replacement = field tx "Replace with" in
  observe t query 1L "already typed";
  observe t replacement 0L "";
  assert (Queue.is_empty t.edits);
  let command, complete = Queue.dequeue_exn t.searches in
  assert (
    match command with
    | Set_query_text value -> String.equal (S.Query.to_string value) "already typed"
    | _ -> false);
  complete (Error Busy);
  cycle t;
  ignore (accept t : W.Transaction.t option);
  Driver.close t.driver;
  print_endline "first observed typing supersedes initial select-all";
  [%expect {| first observed typing supersedes initial select-all |}]
;;

let%expect_test "late open failures do not replace newer successful feedback" =
  let t = create () in
  invoke t "find";
  invoke t "find-replace";
  let _, complete_first = Queue.dequeue_exn t.searches in
  let _, complete_second = Queue.dequeue_exn t.searches in
  complete_second (Ok (Observed (search ~mode:Replace 2L)));
  complete_first (Error Native_failure);
  cycle t;
  let rec has_error view =
    let description = Gpuio.View.Expert.describe view in
    String.is_substring description.text ~substring:"could not complete"
    || List.exists description.children ~f:has_error
  in
  assert (
    not
      (has_error
         (C.wrap
            (Option.value_exn !(t.latest))
            (Gpuio_bonsai.View.text "Document content"))));
  Driver.close t.driver;
  print_endline "only the latest open request can update parent feedback";
  [%expect {| only the latest open request can update parent feedback |}]
;;
