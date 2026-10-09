open Core
open Gpuio_protocol
module B = Bonsai.Cont
module E = Bonsai.Effect
module P = Gpuio.Choice_picker
module Input = Gpuio.Text_input
module Component = Choice_picker_component
module Driver = Gpuio_runtime_core.Window_driver

let ok = Or_error.ok_exn
let window = Window_id.create ~slot:0L ~generation:1L |> ok

let config search =
  let choice =
    Gpuio.Choice.create ~id:(Gpuio.Choice.Id.of_string "a" |> ok) ~label:"A" () |> ok
  in
  let options = Gpuio.Choice.Collection.create [ choice ] |> ok |> P.Collection.flat in
  P.Config.create
    ~label:"Pick"
    ~options
    ~selected:(P.Selection.single None)
    ~search
    ~clearable:true
    ()
  |> ok
;;

module Harness = struct
  type request =
    { expected : Input.Snapshot.t
    ; command : Input.Command.t
    ; complete : (Input.Snapshot.t, Input.Command_error.t) Result.t -> unit
    }

  type t =
    { driver : Driver.t
    ; search : P.Search.t B.Expert.Var.t
    ; latest : Component.t option ref
    ; peek : Component.t B.Computation_status.t E.t option ref
    ; on_event : (P.Event.t -> unit E.t) ref
    ; requests : request Queue.t
    ; root : Node_id.t
    ; handler : Handler_id.t
    ; mutable query : Node_id.t
    }

  let accept driver =
    match Driver.next_message driver with
    | Some (Apply tx) ->
      Driver.submitted driver;
      Driver.acknowledge driver ~revision:tx.revision |> ok;
      Some tx
    | None -> None
    | Some _ -> assert false
  ;;

  let created tx kind =
    List.find_map_exn tx.Wire.Transaction.operations ~f:(function
      | Create (node, actual, _, Some handler) when Wire.Kind.equal actual kind ->
        Some (node, handler)
      | _ -> None)
  ;;

  let create () =
    let search = B.Expert.Var.create P.Search.Substring in
    let latest = ref None in
    let peek = ref None in
    let on_event = ref (fun _ -> E.Ignore) in
    let requests = Queue.create () in
    let command expected command =
      E.Expert.of_fun ~f:(fun ~callback ->
        Queue.enqueue requests { expected; command; complete = callback })
    in
    let component graph =
      let config = B.map (B.Expert.Var.value search) ~f:config in
      let picker =
        Component.create
          command
          ~config
          ~on_event:(B.return (fun event -> !on_event event))
          graph
      in
      let read = B.peek picker graph in
      let open B.Let_syntax in
      let%arr picker = picker
      and read = read in
      latest := Some picker;
      peek := Some read;
      Component.view picker |> ok
    in
    let driver =
      Driver.create window ~start:Time_ns.epoch ~theme:Gpuio.Theme.default component
    in
    Driver.cycle driver ~now:Time_ns.epoch |> ok;
    let tx = accept driver |> Option.value_exn in
    let root, handler = created tx Choice_picker in
    let query, _ = created tx Input in
    { driver; search; latest; peek; on_event; requests; root; handler; query }
  ;;

  let current t = Option.value_exn !(t.latest)
  let cycle t = Driver.cycle t.driver ~now:Time_ns.epoch |> ok

  let run t action =
    Driver.schedule t.driver action;
    cycle t
  ;;

  let snapshot t ?(revision = 1L) text : Choice_picker_wire.Query.t =
    { node = t.query
    ; snapshot =
        { revision
        ; text
        ; selection =
            { anchor = Int64.of_int (String.length text)
            ; head = Int64.of_int (String.length text)
            }
        ; composition = None
        ; focused = true
        }
    }
  ;;

  let dispatch t event =
    Driver.dispatch
      t.driver
      (Choice_picker_event (window, t.root, t.handler, Driver.revision t.driver, event));
    cycle t
  ;;

  let snapshot_text picker = Option.map (Component.snapshot picker) ~f:Input.Snapshot.text

  let inspect_callback t event =
    let open E.Let_syntax in
    let%bind current = Option.value_exn !(t.peek) in
    match current with
    | Inactive -> failwith "active picker expected"
    | Active picker ->
      let kind =
        match event with
        | P.Event.Query_changed _ -> "query"
        | Selection_requested _ -> "selection"
        | Open_requested _ -> "open"
        | Visibility _ -> "visibility"
      in
      E.of_thunk (fun () ->
        print_s [%sexp (kind : string), (snapshot_text picker : string option)])
  ;;
end

let%expect_test "picker observations settle before callbacks sample the Bonsai controller"
  =
  let t = Harness.create () in
  t.on_event := Harness.inspect_callback t;
  assert (Option.is_none (Component.snapshot (Harness.current t)));
  Harness.dispatch t (Query_changed (Harness.snapshot t "draft"));
  Harness.dispatch
    t
    (Selection_requested (Select "a", Some (Harness.snapshot t ~revision:2L "selected")));
  (* A lower native revision cannot roll back the reactive snapshot. *)
  Harness.dispatch t (Query_changed (Harness.snapshot t "older"));
  assert (Queue.is_empty t.requests);
  Driver.close t.driver;
  [%expect
    {|
    (query (draft))
    (selection (selected))
    (query (selected))
    |}]
;;

let%expect_test "late command replies cannot replace newer typing or a remounted query" =
  let t = Harness.create () in
  Harness.dispatch t (Query_changed (Harness.snapshot t "first"));
  let results = ref [] in
  let capture action = E.map action ~f:(fun result -> results := result :: !results) in
  Harness.run t (capture (Component.focus (Harness.current t)));
  let first = Queue.dequeue_exn t.requests in
  Harness.dispatch t (Query_changed (Harness.snapshot t ~revision:3L "typed"));
  first.complete (Ok first.expected);
  Harness.cycle t;
  print_s [%sexp (Harness.snapshot_text (Harness.current t) : string option)];
  Harness.run t (capture (Component.focus (Harness.current t)));
  let retiring = Queue.dequeue_exn t.requests in
  let delayed = capture (Component.focus (Harness.current t)) in
  let old_query = t.query in
  B.Expert.Var.set t.search None;
  Harness.cycle t;
  ignore (Harness.accept t.driver : Wire.Transaction.t option);
  assert (Option.is_none (Component.snapshot (Harness.current t)));
  Harness.run t (capture (Component.focus (Harness.current t)));
  assert (Queue.is_empty t.requests);
  B.Expert.Var.set t.search Substring;
  Harness.cycle t;
  let tx = Harness.accept t.driver |> Option.value_exn in
  t.query <- fst (Harness.created tx Input);
  assert (not (Node_id.equal old_query t.query));
  Harness.dispatch t (Query_changed (Harness.snapshot t "replacement"));
  retiring.complete (Ok retiring.expected);
  Harness.cycle t;
  Harness.run t delayed;
  let delayed = Queue.dequeue_exn t.requests in
  assert (Node_id.equal (Input.Expert.node delayed.expected) old_query);
  delayed.complete (Error Stale_editor);
  Harness.cycle t;
  Harness.dispatch
    t
    (Query_changed { (Harness.snapshot t ~revision:99L "retired") with node = old_query });
  print_s [%sexp (Harness.snapshot_text (Harness.current t) : string option)];
  print_s
    [%sexp
      (List.rev_map !results ~f:(Result.map ~f:Input.Snapshot.text)
       : (string, Input.Command_error.t) Result.t list)];
  Driver.close t.driver;
  [%expect
    {|
    (typed)
    (replacement)
    ((Ok first) (Error Not_mounted) (Ok typed) (Error Stale_editor))
    |}]
;;

let%expect_test
    "conditional replacement targets the selection request's exact lease and revision"
  =
  let t = Harness.create () in
  let selection = ref None in
  (t.on_event
   := function
      | P.Event.Selection_requested request ->
        E.of_thunk (fun () -> selection := Some request)
      | _ -> E.Ignore);
  Harness.dispatch
    t
    (Selection_requested (Select "a", Some (Harness.snapshot t ~revision:7L "chosen")));
  let selected = Option.value_exn !selection in
  Harness.dispatch t (Query_changed (Harness.snapshot t ~revision:8L "new typing"));
  let result = ref None in
  Harness.run
    t
    (E.map
       (Component.replace_if_unchanged (Harness.current t) selected "")
       ~f:(fun value -> result := Some value));
  let request = Queue.dequeue_exn t.requests in
  print_s
    [%sexp
      (Input.Snapshot.text request.expected : string), (request.command : Input.Command.t)];
  request.complete (Error Stale_revision);
  Harness.cycle t;
  print_s [%sexp (!result : (Input.Snapshot.t, Input.Command_error.t) Result.t option)];
  print_s [%sexp (Harness.snapshot_text (Harness.current t) : string option)];
  Driver.close t.driver;
  [%expect
    {|
    (chosen (Replace (text "") (selection End) (undo Record) (if_revision (7))))
    ((Error Stale_revision))
    ("new typing")
    |}]
;;

let%expect_test "retired query selections cannot issue commands when search is disabled" =
  let t = Harness.create () in
  let selected = ref None in
  (t.on_event
   := function
      | P.Event.Selection_requested request ->
        E.of_thunk (fun () -> selected := Some request)
      | _ -> E.Ignore);
  Harness.dispatch
    t
    (Selection_requested (Select "a", Some (Harness.snapshot t "selected")));
  let selected = Option.value_exn !selected in
  B.Expert.Var.set t.search None;
  Harness.cycle t;
  ignore (Harness.accept t.driver : Wire.Transaction.t option);
  let result = ref None in
  Harness.run
    t
    (E.map
       (Component.replace_if_unchanged (Harness.current t) selected "")
       ~f:(fun value -> result := Some value));
  assert (Queue.is_empty t.requests);
  print_s [%sexp (!result : (Input.Snapshot.t, Input.Command_error.t) Result.t option)];
  B.Expert.Var.set t.search Substring;
  Harness.cycle t;
  let tx = Harness.accept t.driver |> Option.value_exn in
  t.query <- fst (Harness.created tx Input);
  Harness.dispatch t (Query_changed (Harness.snapshot t "replacement"));
  result := None;
  Harness.run
    t
    (E.map
       (Component.replace_if_unchanged (Harness.current t) selected "")
       ~f:(fun value -> result := Some value));
  assert (Queue.is_empty t.requests);
  print_s [%sexp (!result : (Input.Snapshot.t, Input.Command_error.t) Result.t option)];
  Driver.close t.driver;
  [%expect
    {|
    ((Error Not_mounted))
    ((Error Stale_editor))
    |}]
;;
