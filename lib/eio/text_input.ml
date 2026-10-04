open Core
module Bonsai = Bonsai.Cont
module Input = Gpuio.Text_input

type t =
  { editor : Editor_controller.t
  ; query_hint :
      Input.Snapshot.t
      -> (Input.Content_hint.Status.t, Input.Command_error.t) Result.t Bonsai.Effect.t
  ; query_range :
      Input.Snapshot.t
      -> Input.Selection.t
      -> (Gpuio.Editor_geometry.t option, Input.Command_error.t) Result.t Bonsai.Effect.t
  ; query_viewport :
      Input.Snapshot.t
      -> (Gpuio.Editor_viewport.t option, Input.Command_error.t) Result.t Bonsai.Effect.t
  ; scroll_viewport :
      Input.Snapshot.t
      -> Gpuio.Editor_viewport.Offset.t
      -> (unit, Input.Command_error.t) Result.t Bonsai.Effect.t
  ; config : Input.Config.t
  ; query_search :
      Input.Snapshot.t
      -> Input.Search.Command.t
      -> ( Input.Search.Response.t * Input.Snapshot.t option
           , Input.Command_error.t )
           Result.t
           Bonsai.Effect.t
  ; initial_text : string
  ; on_submit : Input.Submission.t -> unit Bonsai.Effect.t
  ; on_event : Input.Event.t -> unit Bonsai.Effect.t
  }

let create window ~config ?(initial_text = "") ?on_submit graph =
  let editor = Editor_controller.create window graph in
  let on_submit =
    Option.value on_submit ~default:(Bonsai.return (fun _ -> Bonsai.Effect.Ignore))
  in
  let open Bonsai.Let_syntax in
  let%arr editor = editor
  and config = config
  and on_submit = on_submit in
  Input.validate_text ~mode:(Input.Config.mode config) initial_text |> Or_error.ok_exn;
  let on_event = function
    | Input.Event.Search_changed snapshot ->
      Editor_controller.observe_search editor snapshot
    | Input.Event.Changed snapshot -> Editor_controller.observe editor snapshot
    | Submitted submission ->
      Bonsai.Effect.Many
        [ Editor_controller.observe editor (Input.Expert.submission_snapshot submission)
        ; on_submit submission
        ]
  in
  { editor
  ; query_hint = App.Window.Expert.editor_content_hint_status window
  ; query_range = App.Window.Expert.editor_range_bounds window
  ; query_viewport = App.Window.Expert.editor_viewport window
  ; scroll_viewport = App.Window.Expert.editor_scroll_to window
  ; config
  ; query_search = App.Window.Expert.editor_search window
  ; initial_text
  ; on_event
  ; on_submit
  }
;;

let view ?style ?initial_text ?(on_event = fun _ -> Bonsai.Effect.Ignore) t =
  Gpuio.View.text_input
    ?style
    ~initial_text:(Option.value initial_text ~default:t.initial_text)
    ~controller:(Editor_controller.key t.editor)
    ~config:t.config
    ~on_event:(fun event -> Bonsai.Effect.Many [ t.on_event event; on_event event ])
    ()
  |> Or_error.ok_exn
;;

let snapshot t = Editor_controller.snapshot t.editor
let search_snapshot t = Editor_controller.search_snapshot t.editor
let command t command = Editor_controller.command t.editor command
let focus t = Editor_controller.focus t.editor
let select t selection = Editor_controller.select t.editor selection

let replace t ?if_revision ~selection ~undo text =
  Editor_controller.replace t.editor ?if_revision ~selection ~undo text
;;

let clear_if_unchanged t submission =
  Editor_controller.replace_if_unchanged
    t.editor
    (Input.Expert.submission_snapshot submission)
    ~selection:Start
    ~undo:Record
    ""
;;

let replace_if_unchanged t expected ~selection ~undo text =
  Editor_controller.replace_if_unchanged t.editor expected ~selection ~undo text
;;

let submit t =
  let open Bonsai.Effect.Let_syntax in
  let%bind result = command t Submit in
  match result with
  | Error error -> Bonsai.Effect.return (Error error)
  | Ok snapshot ->
    (match Input.Expert.submission snapshot with
     | Error _ -> Bonsai.Effect.return (Error Input.Command_error.Composing)
     | Ok submission ->
       let%map () = t.on_submit submission in
       Ok ())
;;

let read_snapshot t = command t Read_snapshot

let content_hint_status t =
  match snapshot t with
  | None -> Bonsai.Effect.return (Error Input.Command_error.Not_mounted)
  | Some snapshot -> t.query_hint snapshot
;;

let read_viewport t =
  match snapshot t with
  | None -> Bonsai.Effect.return (Error Input.Command_error.Not_mounted)
  | Some snapshot -> t.query_viewport snapshot
;;

let range_bounds t ~snapshot:expected ~range =
  match snapshot t with
  | None -> Bonsai.Effect.return (Error Input.Command_error.Not_mounted)
  | Some current
    when (not
            (Gpuio_protocol.Window_id.equal
               (Input.Expert.window current)
               (Input.Expert.window expected)))
         || not
              (Gpuio_protocol.Node_id.equal
                 (Input.Expert.node current)
                 (Input.Expert.node expected)) ->
    Bonsai.Effect.return (Error Input.Command_error.Stale_editor)
  | Some _ -> t.query_range expected range
;;

let scroll_to t offset =
  match snapshot t with
  | None -> Bonsai.Effect.return (Error Input.Command_error.Not_mounted)
  | Some snapshot -> t.scroll_viewport snapshot offset
;;

let search_command t command =
  match snapshot t with
  | None -> Bonsai.Effect.return (Error Input.Command_error.Not_mounted)
  | Some snapshot ->
    let open Bonsai.Effect.Let_syntax in
    let%bind result = t.query_search snapshot command in
    (match result with
     | Error error -> Bonsai.Effect.return (Error error)
     | Ok (response, editor) ->
       let%bind () =
         match editor with
         | None -> Bonsai.Effect.Ignore
         | Some editor ->
           Editor_controller.observe_reply t.editor ~expected:snapshot editor
       in
       let search =
         match response with
         | Input.Search.Response.Observed search -> search
         | Replaced { snapshot; count = _ } -> snapshot
       in
       let%map () =
         Editor_controller.observe_search_reply t.editor ~expected:snapshot search
       in
       Ok response)
;;

let%test_module "hint status observations" =
  (module struct
    module Driver = Gpuio_runtime_core.Window_driver
    module W = Gpuio_protocol.Wire
    module E = Bonsai.Effect

    let%expect_test
        "late hint replies preserve newer editor observations and identify their hint"
      =
      let ok = Or_error.ok_exn in
      let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
      let latest = ref None in
      let hint = Bonsai.Expert.Var.create Input.Content_hint.Email_address in
      let requests = Queue.create () in
      let results = ref [] in
      let component graph =
        let editor =
          Editor_controller.create_with_command
            (fun _ _ -> failwith "metadata query sent an editing command")
            graph
        in
        let open Bonsai.Let_syntax in
        let%arr editor = editor
        and content_hint = Bonsai.Expert.Var.value hint in
        let t =
          { editor
          ; query_hint =
              (fun expected ->
                E.Expert.of_fun ~f:(fun ~callback ->
                  Queue.enqueue requests (expected, callback)))
          ; query_range = (fun _ _ -> failwith "unexpected range query")
          ; query_viewport = (fun _ -> failwith "unexpected viewport query")
          ; scroll_viewport = (fun _ _ -> failwith "unexpected viewport scroll")
          ; query_search = (fun _ _ -> failwith "unexpected search command")
          ; config =
              Input.Config.create ~mode:Single_line ~label:"Hint test" ~content_hint ()
              |> ok
          ; initial_text = ""
          ; on_submit = (fun _ -> E.Ignore)
          ; on_event =
              (function
                | Input.Event.Changed snapshot ->
                  Editor_controller.observe editor snapshot
                | Search_changed search -> Editor_controller.observe_search editor search
                | Submitted _ -> E.Ignore)
          }
        in
        latest := Some t;
        view t
      in
      let driver =
        Driver.create window ~start:Time_ns.epoch ~theme:Gpuio.Theme.default component
      in
      let cycle () = Driver.cycle driver ~now:Time_ns.epoch |> ok in
      let current () = Option.value_exn !latest in
      let query () =
        Driver.schedule
          driver
          (E.map
             (content_hint_status (current ()))
             ~f:(fun result -> results := result :: !results));
        cycle ()
      in
      cycle ();
      let tx =
        match Driver.next_message driver with
        | Some (Apply tx) -> tx
        | _ -> assert false
      in
      let node, handler =
        List.find_map_exn tx.operations ~f:(function
          | W.Op.Create (node, Input, _, Some handler) -> Some (node, handler)
          | _ -> None)
      in
      Driver.submitted driver;
      Driver.acknowledge driver ~revision:tx.revision |> ok;
      query ();
      assert (Queue.is_empty requests);
      let observe revision text =
        let value : W.Editor.Snapshot.t =
          { revision
          ; text
          ; selection = { anchor = 0L; head = 0L }
          ; composition = None
          ; focused = true
          }
        in
        Driver.dispatch
          driver
          (Editor_event (window, node, handler, Driver.revision driver, Changed, value));
        cycle ()
      in
      observe 1L "first";
      query ();
      let expected, complete = Queue.dequeue_exn requests in
      assert (String.equal (Input.Snapshot.text expected) "first");
      Bonsai.Expert.Var.set hint Url;
      observe 2L "new typing";
      let before = snapshot (current ()) |> Option.value_exn in
      complete (Ok (Input.Content_hint.Status.Exposed Email_address));
      cycle ();
      assert (Input.Snapshot.equal before (snapshot (current ()) |> Option.value_exn));
      assert (Queue.is_empty requests);
      List.rev !results
      |> List.iter ~f:(fun result ->
        print_s
          [%sexp (result : (Input.Content_hint.Status.t, Input.Command_error.t) Result.t)]);
      print_endline (Input.Snapshot.text before);
      Driver.close driver;
      [%expect
        {|
      (Error Not_mounted)
      (Ok (Exposed Email_address))
      new typing
    |}]
    ;;

    let%expect_test
        "late viewport replies preserve newer editor observations and the original lease"
      =
      let ok = Or_error.ok_exn in
      let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
      let latest = ref None in
      let hint = Bonsai.Expert.Var.create Input.Content_hint.Email_address in
      let requests = Queue.create () in
      let results = ref [] in
      let component graph =
        let editor =
          Editor_controller.create_with_command
            (fun _ _ -> failwith "metadata query sent an editing command")
            graph
        in
        let open Bonsai.Let_syntax in
        let%arr editor = editor
        and content_hint = Bonsai.Expert.Var.value hint in
        let t =
          { editor
          ; query_hint = (fun _ -> failwith "unexpected hint query")
          ; query_range = (fun _ _ -> failwith "unexpected range query")
          ; query_viewport =
              (fun expected ->
                E.Expert.of_fun ~f:(fun ~callback ->
                  Queue.enqueue requests (expected, callback)))
          ; scroll_viewport = (fun _ _ -> failwith "unexpected viewport scroll")
          ; query_search = (fun _ _ -> failwith "unexpected search command")
          ; config =
              Input.Config.create
                ~mode:Single_line
                ~label:"Viewport test"
                ~content_hint
                ()
              |> ok
          ; initial_text = ""
          ; on_submit = (fun _ -> E.Ignore)
          ; on_event =
              (function
                | Input.Event.Changed snapshot ->
                  Editor_controller.observe editor snapshot
                | Search_changed search -> Editor_controller.observe_search editor search
                | Submitted _ -> E.Ignore)
          }
        in
        latest := Some t;
        view t
      in
      let driver =
        Driver.create window ~start:Time_ns.epoch ~theme:Gpuio.Theme.default component
      in
      let cycle () = Driver.cycle driver ~now:Time_ns.epoch |> ok in
      let current () = Option.value_exn !latest in
      let query () =
        Driver.schedule
          driver
          (E.map
             (read_viewport (current ()))
             ~f:(fun result -> results := result :: !results));
        cycle ()
      in
      cycle ();
      let tx =
        match Driver.next_message driver with
        | Some (Apply tx) -> tx
        | _ -> assert false
      in
      let node, handler =
        List.find_map_exn tx.operations ~f:(function
          | W.Op.Create (node, Input, _, Some handler) -> Some (node, handler)
          | _ -> None)
      in
      Driver.submitted driver;
      Driver.acknowledge driver ~revision:tx.revision |> ok;
      query ();
      assert (Queue.is_empty requests);
      let observe revision text =
        let value : W.Editor.Snapshot.t =
          { revision
          ; text
          ; selection = { anchor = 0L; head = 0L }
          ; composition = None
          ; focused = true
          }
        in
        Driver.dispatch
          driver
          (Editor_event (window, node, handler, Driver.revision driver, Changed, value));
        cycle ()
      in
      observe 1L "first";
      query ();
      let expected, complete = Queue.dequeue_exn requests in
      assert (String.equal (Input.Snapshot.text expected) "first");
      Bonsai.Expert.Var.set hint Url;
      observe 2L "new typing";
      let before = snapshot (current ()) |> Option.value_exn in
      complete (Ok None);
      cycle ();
      assert (Input.Snapshot.equal before (snapshot (current ()) |> Option.value_exn));
      assert (Queue.is_empty requests);
      List.rev !results
      |> List.iter ~f:(fun result ->
        print_s
          [%sexp
            (result : (Gpuio.Editor_viewport.t option, Input.Command_error.t) Result.t)]);
      print_endline (Input.Snapshot.text before);
      Driver.close driver;
      [%expect
        {|
      (Error Not_mounted)
      (Ok ())
      new typing
    |}]
    ;;

    let%expect_test "late search replacement replies preserve newer native typing" =
      let ok = Or_error.ok_exn in
      let module S = Input.Search in
      let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
      let latest = ref None in
      let requests = Queue.create () in
      let component graph =
        let editor =
          Editor_controller.create_with_command
            (fun _ _ -> failwith "wrong command route")
            graph
        in
        let open Bonsai.Let_syntax in
        let%arr editor = editor in
        let t =
          { editor
          ; query_hint = (fun _ -> failwith "hint")
          ; query_range = (fun _ _ -> failwith "unexpected range query")
          ; query_viewport = (fun _ -> failwith "viewport")
          ; scroll_viewport = (fun _ _ -> failwith "scroll")
          ; query_search =
              (fun expected command ->
                E.Expert.of_fun ~f:(fun ~callback ->
                  Queue.enqueue requests (expected, command, callback)))
          ; config =
              Input.Config.create ~mode:Multiline ~searchable:true ~label:"Search" ()
              |> ok
          ; initial_text = "one"
          ; on_submit = (fun _ -> E.Ignore)
          ; on_event =
              (function
                | Changed snapshot -> Editor_controller.observe editor snapshot
                | Search_changed search -> Editor_controller.observe_search editor search
                | Submitted _ -> E.Ignore)
          }
        in
        latest := Some t;
        view t
      in
      let driver =
        Driver.create window ~start:Time_ns.epoch ~theme:Gpuio.Theme.default component
      in
      let cycle () = Driver.cycle driver ~now:Time_ns.epoch |> ok in
      let current () = Option.value_exn !latest in
      cycle ();
      let tx =
        match Driver.next_message driver with
        | Some (Apply tx) -> tx
        | _ -> assert false
      in
      let node, handler =
        List.find_map_exn tx.operations ~f:(function
          | W.Op.Create (node, Textarea, _, Some handler) -> Some (node, handler)
          | _ -> None)
      in
      Driver.submitted driver;
      Driver.acknowledge driver ~revision:tx.revision |> ok;
      let editor_wire revision text : W.Editor.Snapshot.t =
        { revision
        ; text
        ; selection = { anchor = 0L; head = 0L }
        ; composition = None
        ; focused = true
        }
      in
      let observe revision text =
        Driver.dispatch
          driver
          (Editor_event
             ( window
             , node
             , handler
             , Driver.revision driver
             , Changed
             , editor_wire revision text ));
        cycle ()
      in
      observe 2L "one";
      let metadata revision count current : Gpuio_protocol.Editor_search_wire.Snapshot.t =
        { stamp = { editor_revision = revision; search_revision = revision }
        ; activation_revision = 1L
        ; mode = Replace
        ; query = "o"
        ; case = Sensitive
        ; text_bytes = 3L
        ; match_count = count
        ; current
        ; can_replace = true
        }
      in
      let before =
        S.Expert.snapshot_of_wire
          ~window
          ~node
          (metadata 2L 1L (Some { index = 0L; byte_start = 0L; byte_end = 1L }))
        |> ok
      in
      assert (Option.is_none (search_snapshot (current ())));
      Driver.dispatch
        driver
        (Editor_search_observed
           ( window
           , node
           , handler
           , Driver.revision driver
           , metadata 2L 1L (Some { index = 0L; byte_start = 0L; byte_end = 1L }) ));
      cycle ();
      assert (Option.equal S.Snapshot.equal (search_snapshot (current ())) (Some before));
      let result = ref None in
      Driver.schedule
        driver
        (E.map
           (search_command
              (current ())
              (Replace_current { if_stamp = S.Snapshot.stamp before; replacement = "X" }))
           ~f:(fun value -> result := Some value));
      cycle ();
      let expected, _, complete = Queue.dequeue_exn requests in
      assert (String.equal (Input.Snapshot.text expected) "one");
      observe 4L "new typing";
      let after = S.Expert.snapshot_of_wire ~window ~node (metadata 3L 0L None) |> ok in
      let editor =
        Input.Expert.snapshot_of_wire ~window ~node (editor_wire 3L "Xne") |> ok
      in
      complete (Ok (S.Response.Replaced { snapshot = after; count = 1 }, Some editor));
      cycle ();
      assert (
        match !result with
        | Some (Ok (Replaced { count = 1; _ })) -> true
        | _ -> false);
      print_endline (Input.Snapshot.text (snapshot (current ()) |> Option.value_exn));
      assert (Option.equal S.Snapshot.equal (search_snapshot (current ())) (Some before));
      observe 5L "one";
      let before =
        S.Expert.snapshot_of_wire
          ~window
          ~node
          (metadata 5L 1L (Some { index = 0L; byte_start = 0L; byte_end = 1L }))
        |> ok
      in
      Driver.schedule
        driver
        (E.map
           (search_command
              (current ())
              (Replace_current { if_stamp = S.Snapshot.stamp before; replacement = "X" }))
           ~f:(fun _ -> ()));
      cycle ();
      let expected, _, complete = Queue.dequeue_exn requests in
      assert (Gpuio_protocol.Node_id.equal (Input.Expert.node expected) node);
      let fresh_node = Gpuio_protocol.Node_id.create ~slot:99L ~generation:1L |> ok in
      let fresh =
        Input.Expert.snapshot_of_wire
          ~window
          ~node:fresh_node
          (editor_wire 0L "fresh mount")
        |> ok
      in
      (* Exercise the controller transition produced by a new native mount. *)
      Driver.schedule driver (Editor_controller.observe (current ()).editor fresh);
      cycle ();
      let old_reply =
        Input.Expert.snapshot_of_wire ~window ~node (editor_wire 6L "Xne") |> ok
      in
      let after = S.Expert.snapshot_of_wire ~window ~node (metadata 6L 0L None) |> ok in
      complete (Ok (S.Response.Replaced { snapshot = after; count = 1 }, Some old_reply));
      cycle ();
      let observed = snapshot (current ()) |> Option.value_exn in
      assert (Gpuio_protocol.Node_id.equal (Input.Expert.node observed) fresh_node);
      assert (Option.is_none (search_snapshot (current ())));
      print_endline (Input.Snapshot.text observed);
      Driver.close driver;
      [%expect
        {|
        new typing
        fresh mount
      |}]
    ;;
  end)
;;

let%test_module "range geometry observations" =
  (module struct
    module Driver = Gpuio_runtime_core.Window_driver
    module W = Gpuio_protocol.Wire
    module E = Bonsai.Effect

    let ok = Or_error.ok_exn

    let%expect_test
        "late geometry cannot replace newer text and foreign leases are refused"
      =
      let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
      let latest = ref None in
      let requests = Queue.create () in
      let results = Queue.create () in
      let component graph =
        let editor =
          Editor_controller.create_with_command
            (fun _ _ -> failwith "range query sent an editing command")
            graph
        in
        let open Bonsai.Let_syntax in
        let%arr editor = editor in
        let t =
          { editor
          ; query_hint = (fun _ -> failwith "hint")
          ; query_viewport = (fun _ -> failwith "viewport")
          ; query_range =
              (fun expected range ->
                E.Expert.of_fun ~f:(fun ~callback ->
                  Queue.enqueue requests (expected, range, callback)))
          ; scroll_viewport = (fun _ _ -> failwith "scroll")
          ; query_search = (fun _ _ -> failwith "search")
          ; config = Input.Config.create ~mode:Single_line ~label:"Range test" () |> ok
          ; initial_text = ""
          ; on_submit = (fun _ -> E.Ignore)
          ; on_event =
              (function
                | Input.Event.Changed snapshot ->
                  Editor_controller.observe editor snapshot
                | Search_changed search -> Editor_controller.observe_search editor search
                | Submitted _ -> E.Ignore)
          }
        in
        latest := Some t;
        view t
      in
      let driver =
        Driver.create window ~start:Time_ns.epoch ~theme:Gpuio.Theme.default component
      in
      let cycle () = Driver.cycle driver ~now:Time_ns.epoch |> ok in
      let current () = Option.value_exn !latest in
      cycle ();
      let tx =
        match Driver.next_message driver with
        | Some (Apply tx) -> tx
        | _ -> assert false
      in
      let node, handler =
        List.find_map_exn tx.operations ~f:(function
          | W.Op.Create (node, Input, _, Some handler) -> Some (node, handler)
          | _ -> None)
      in
      Driver.submitted driver;
      Driver.acknowledge driver ~revision:tx.revision |> ok;
      let wire revision text : W.Editor.Snapshot.t =
        { revision
        ; text
        ; selection = { anchor = 0L; head = 0L }
        ; composition = None
        ; focused = true
        }
      in
      let expected =
        Input.Expert.snapshot_of_wire ~window ~node (wire 1L "first") |> ok
      in
      let range = Input.Selection.create ~anchor:1 ~head:0 |> ok in
      let query expected =
        Driver.schedule
          driver
          (E.map
             (range_bounds (current ()) ~snapshot:expected ~range)
             ~f:(Queue.enqueue results));
        cycle ()
      in
      query expected;
      assert (Queue.is_empty requests);
      assert (
        match Queue.dequeue_exn results with
        | Error Not_mounted -> true
        | _ -> false);
      let observe revision text =
        Driver.dispatch
          driver
          (Editor_event
             (window, node, handler, Driver.revision driver, Changed, wire revision text));
        cycle ()
      in
      observe 1L "first";
      query expected;
      let sent, sent_range, complete = Queue.dequeue_exn requests in
      assert (Input.Snapshot.equal sent expected && Input.Selection.equal sent_range range);
      observe 2L "new typing";
      let before = snapshot (current ()) |> Option.value_exn in
      let geometry =
        Gpuio.Editor_geometry.Expert.of_wire
          { revision = 1L; x = 0.; y = 0.; width = 10.; height = 20. }
        |> ok
      in
      complete (Ok (Some geometry));
      cycle ();
      assert (Input.Snapshot.equal before (snapshot (current ()) |> Option.value_exn));
      assert (
        match Queue.dequeue_exn results with
        | Ok (Some result) -> Gpuio.Editor_geometry.equal result geometry
        | _ -> false);
      let replacement = Gpuio_protocol.Node_id.create ~slot:1L ~generation:99L |> ok in
      let foreign =
        Input.Expert.snapshot_of_wire ~window ~node:replacement (wire 1L "first") |> ok
      in
      query foreign;
      assert (Queue.is_empty requests);
      assert (
        match Queue.dequeue_exn results with
        | Error Stale_editor -> true
        | _ -> false);
      assert (String.equal (Input.Snapshot.text before) "new typing");
      Driver.close driver;
      [%expect {| |}]
    ;;
  end)
;;
