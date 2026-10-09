open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module P = Gpuio.Date_picker
module C = Gpuio.Calendar
module Picker = Date_picker_component
module W = Gpuio_protocol.Wire
module CW = Gpuio_protocol.Calendar_wire
module Driver = Gpuio_runtime_core.Window_driver

let ok = Or_error.ok_exn
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok
let overlay = Gpuio.Overlay.Config.create ~label:"Date popup" () |> ok
let date = Date.of_string "2024-02-29"
let month = C.Month.of_date date |> ok
let selected = C.Selection.single date |> ok
let config = C.Config.create ~label:"Date" () |> ok

let preset =
  P.Preset.create
    ~id:(Gpuio.Choice.Id.of_string "leap" |> ok)
    ~label:"Leap day"
    ~selection:selected
  |> ok
;;

let presets = P.Preset.Collection.create [ preset ] |> ok

type request =
  { expected : C.Snapshot.t
  ; command : C.Command.t
  ; complete : (C.Snapshot.t, C.Command_error.t) Result.t -> unit
  }

type harness =
  { driver : Driver.t
  ; initial : W.Op.t list
  ; content : unit E.t Gpuio.View.Calendar_content.t option B.Expert.Var.t
  ; trigger : Gpuio_bonsai.View.t option B.Expert.Var.t
  ; config : C.Config.t B.Expert.Var.t
  ; value : C.Selection.t B.Expert.Var.t
  ; active : bool B.Expert.Var.t
  ; latest : Picker.t option ref
  ; requests : request Queue.t
  ; commits : C.Selection.t Queue.t
  }

let cycle t =
  Driver.cycle t.driver ~now:Time_ns.epoch |> ok;
  match Driver.next_message t.driver with
  | Some (W.Message.Apply tx) ->
    Driver.submitted t.driver;
    Driver.acknowledge t.driver ~revision:tx.revision |> ok;
    tx.operations
  | None -> []
  | Some _ -> assert false
;;

let current t = Option.value_exn !(t.latest)

let run t action =
  Driver.schedule t.driver action;
  cycle t
;;

let ignore_run t action = ignore (run t action : W.Op.t list)

let create ?on_calendar_viewport_change ?(presets = presets) () =
  let config = B.Expert.Var.create config
  and value = B.Expert.Var.create C.Selection.empty
  and active = B.Expert.Var.create true in
  let trigger = B.Expert.Var.create None in
  let content = B.Expert.Var.create None in
  let latest = ref None
  and requests = Queue.create ()
  and commits = Queue.create () in
  let native_command expected command =
    E.Expert.of_fun ~f:(fun ~callback ->
      Queue.enqueue requests { expected; command; complete = callback })
  in
  let component graph =
    let open B.Let_syntax in
    match%sub B.Expert.Var.value active with
    | false -> B.return (Gpuio_bonsai.View.column [])
    | true ->
      let picker =
        Picker.create
          native_command
          ~config:(B.Expert.Var.value config)
          ~value:(B.Expert.Var.value value)
          ~initial_month:month
          ~on_change:
            (B.return (fun value -> E.of_thunk (fun () -> Queue.enqueue commits value)))
          graph
      in
      let%arr picker = picker
      and trigger = B.Expert.Var.value trigger
      and content = B.Expert.Var.value content in
      latest := Some picker;
      (match trigger with
       | None ->
         Picker.view
           ?calendar_content:content
           ?on_calendar_viewport_change
           ~presets
           ~overlay
           ~label:"Choose date"
           picker
       | Some trigger ->
         Picker.view_with_trigger
           ?calendar_content:content
           ?on_calendar_viewport_change
           ~presets
           ~overlay
           ~accessible_name:"Choose date"
           ~trigger
           picker
         |> ok)
  in
  let driver =
    Driver.create window ~start:Time_ns.epoch ~theme:Gpuio.Theme.default component
  in
  let t =
    { driver
    ; initial = []
    ; content
    ; trigger
    ; config
    ; value
    ; active
    ; latest
    ; requests
    ; commits
    }
  in
  { t with initial = cycle t }
;;

let wire revision selection : CW.Snapshot.t =
  { revision
  ; mode = Single
  ; selection = C.Expert.selection_to_wire selection
  ; selection_allowed = true
  ; month = C.Expert.month_to_wire month
  ; focused_date = C.Expert.date_to_ordinal date |> ok
  ; presentation = Days
  ; focused = false
  }
;;

let open_popup_details t =
  let operations = run t (Picker.open_popup (current t)) in
  let node =
    List.find_map_exn operations ~f:(function
      | W.Op.Set_calendar (node, _, _, _) -> Some node
      | _ -> None)
  in
  let handler =
    List.find_map_exn operations ~f:(function
      | W.Op.Create (id, Calendar, _, Some handler)
        when Gpuio_protocol.Node_id.equal id node -> Some handler
      | _ -> None)
  in
  Driver.dispatch
    t.driver
    (W.Event.Calendar_event
       ( window
       , node
       , handler
       , Driver.revision t.driver
       , CW.Event.Observed (wire 0L C.Selection.empty) ));
  let observed = cycle t in
  node, handler, operations @ observed
;;

let open_popup t =
  let node, handler, _ = open_popup_details t in
  node, handler
;;

let finish t request node revision selection =
  request.complete
    (Ok (C.Expert.snapshot_of_wire ~window ~node (wire revision selection) |> ok));
  ignore (cycle t : W.Op.t list)
;;

let collect queue action = E.map action ~f:(Queue.enqueue queue)

let%expect_test "preset changes only the draft and gates Apply until its guarded reply" =
  let t = create () in
  assert (
    List.exists t.initial ~f:(function
      | W.Op.Set_popover (_, true) -> true
      | _ -> false));
  let node, _ = open_popup t in
  let captured = current t in
  let results = Queue.create ()
  and confirmations = Queue.create () in
  ignore_run t (collect results (Picker.select_preset captured preset));
  assert (Picker.is_selecting_preset (current t) && not (Picker.can_confirm (current t)));
  ignore_run
    t
    (E.Many
       [ collect results (Picker.select_preset captured preset)
       ; collect confirmations (Picker.confirm captured)
       ]);
  assert (Queue.length t.requests = 1);
  let request = Queue.dequeue_exn t.requests in
  assert (
    C.Command.equal
      request.command
      (Replace
         { selection = selected
         ; if_revision = Some (C.Snapshot.revision request.expected)
         }));
  finish t request node 1L selected;
  assert (Picker.is_open (current t) && Picker.can_confirm (current t));
  assert (Queue.is_empty t.commits);
  print_s
    [%sexp
      (Queue.to_list results |> List.map ~f:(Result.map ~f:(fun _ -> ()))
       : (unit, P.Error.t) Result.t list)];
  print_s [%sexp (Queue.to_list confirmations : (C.Selection.t, P.Error.t) Result.t list)];
  ignore_run t (collect confirmations (Picker.confirm (current t)));
  let request = Queue.dequeue_exn t.requests in
  assert (C.Command.equal request.command Read_snapshot);
  finish t request node 1L selected;
  assert (not (Picker.is_open (current t)));
  print_s [%sexp (Queue.to_list t.commits : C.Selection.t list)];
  [%expect
    {|
    ((Error (Native Busy)) (Ok ()))
    ((Error (Native Busy)))
    ((Single 2024-02-29))
    |}]
;;

let%expect_test "old preset reply cannot change a reopened picker or clear its busy gate" =
  let t = create () in
  let node, _ = open_popup t in
  let old = current t in
  let results = Queue.create () in
  ignore_run t (collect results (Picker.select_preset old preset));
  let first = Queue.dequeue_exn t.requests in
  ignore_run t (Picker.cancel old);
  let next, _ = open_popup t in
  ignore_run t (collect results (Picker.select_preset (current t) preset));
  let second = Queue.dequeue_exn t.requests in
  finish t first node 1L selected;
  assert (Picker.is_selecting_preset (current t));
  ignore_run t (collect results (Picker.select_preset old preset));
  assert (Queue.is_empty t.requests);
  finish t second next 1L selected;
  assert ((not (Picker.is_selecting_preset (current t))) && Picker.is_open (current t));
  assert (Queue.is_empty t.commits);
  print_s
    [%sexp
      (Queue.to_list results |> List.map ~f:(Result.map ~f:(fun _ -> ()))
       : (unit, P.Error.t) Result.t list)];
  [%expect {| ((Error Stale_session) (Error Stale_session) (Ok ())) |}]
;;

let%expect_test "captured preset actions revalidate current policy before native dispatch"
  =
  List.iter
    [ "readonly"; "disabled"; "constraints"; "value"; "inactive" ]
    ~f:(fun change ->
      let t = create () in
      ignore (open_popup t);
      let captured = current t in
      (match change with
       | "readonly" ->
         B.Expert.Var.set t.config (C.Config.create ~read_only:true ~label:"Date" () |> ok)
       | "disabled" ->
         B.Expert.Var.set t.config (C.Config.create ~disabled:true ~label:"Date" () |> ok)
       | "constraints" ->
         let constraints = C.Constraints.create ~disabled_dates:[ date ] () |> ok in
         B.Expert.Var.set t.config (C.Config.create ~constraints ~label:"Date" () |> ok)
       | "value" -> B.Expert.Var.set t.value selected
       | "inactive" -> B.Expert.Var.set t.active false
       | _ -> assert false);
      ignore (cycle t : W.Op.t list);
      let result = Queue.create () in
      ignore_run t (collect result (Picker.select_preset captured preset));
      assert (Queue.is_empty t.requests && Queue.is_empty t.commits);
      print_s
        [%sexp
          (change : string)
        , (Queue.to_list result |> List.map ~f:(Result.map ~f:(fun _ -> ()))
           : (unit, P.Error.t) Result.t list)]);
  [%expect
    {|
    (readonly ((Error Read_only)))
    (disabled ((Error Stale_session)))
    (constraints ((Error Disallowed_selection)))
    (value ((Error Stale_session)))
    (inactive ((Error Not_open)))
    |}]
;;

let%expect_test
    "confirmation started first cannot commit while preset replacement is pending"
  =
  let t = create () in
  let node, _ = open_popup t in
  let confirmations = Queue.create ()
  and replacements = Queue.create () in
  ignore_run t (collect confirmations (Picker.confirm (current t)));
  let read = Queue.dequeue_exn t.requests in
  ignore_run t (collect replacements (Picker.select_preset (current t) preset));
  let replace = Queue.dequeue_exn t.requests in
  finish t read node 0L C.Selection.empty;
  assert (Picker.is_selecting_preset (current t) && Queue.is_empty t.commits);
  finish t replace node 1L selected;
  assert (Picker.is_open (current t) && Picker.can_confirm (current t));
  print_s [%sexp (Queue.to_list confirmations : (C.Selection.t, P.Error.t) Result.t list)];
  [%expect {| ((Error (Native Busy))) |}]
;;

let%expect_test
    "later native revisions fence a delayed preset result and native failure releases \
     the gate"
  =
  let t = create () in
  let node, handler = open_popup t in
  let results = Queue.create () in
  ignore_run t (collect results (Picker.select_preset (current t) preset));
  let replace = Queue.dequeue_exn t.requests in
  Driver.dispatch
    t.driver
    (W.Event.Calendar_event
       ( window
       , node
       , handler
       , Driver.revision t.driver
       , CW.Event.Changed (wire 2L C.Selection.empty) ));
  ignore (cycle t : W.Op.t list);
  finish t replace node 1L selected;
  assert (not (Picker.is_selecting_preset (current t)));
  assert (
    C.Selection.equal
      (C.Snapshot.selection (Picker.draft (current t) |> Option.value_exn))
      C.Selection.empty);
  ignore_run t (collect results (Picker.select_preset (current t) preset));
  let replace = Queue.dequeue_exn t.requests in
  replace.complete (Error C.Command_error.Stale_revision);
  ignore (cycle t : W.Op.t list);
  assert ((not (Picker.is_selecting_preset (current t))) && Picker.is_open (current t));
  assert (Queue.is_empty t.commits);
  print_s
    [%sexp
      (Queue.to_list results |> List.map ~f:(Result.map ~f:(fun _ -> ()))
       : (unit, P.Error.t) Result.t list)];
  [%expect {| ((Error Stale_draft) (Error (Native Stale_revision))) |}]
;;

let%expect_test "read-only after dispatch retains the draft but rejects Apply" =
  let t = create () in
  let node, _ = open_popup t in
  let replacements = Queue.create ()
  and confirmations = Queue.create () in
  ignore_run t (collect replacements (Picker.select_preset (current t) preset));
  let replace = Queue.dequeue_exn t.requests in
  B.Expert.Var.set t.config (C.Config.create ~read_only:true ~label:"Date" () |> ok);
  ignore (cycle t : W.Op.t list);
  finish t replace node 1L selected;
  assert (Picker.is_open (current t) && not (Picker.can_confirm (current t)));
  ignore_run t (collect confirmations (Picker.confirm (current t)));
  let read = Queue.dequeue_exn t.requests in
  finish t read node 1L selected;
  assert (Queue.is_empty t.commits);
  print_s [%sexp (Queue.to_list confirmations : (C.Selection.t, P.Error.t) Result.t list)];
  [%expect {| ((Error Read_only)) |}]
;;

let%expect_test
    "public preset button accepts maximum ID and routes into the guarded draft flow"
  =
  let preset =
    P.Preset.create
      ~id:(Gpuio.Choice.Id.of_string (String.make 256 'x') |> ok)
      ~label:"Named date shortcut"
      ~selection:selected
    |> ok
  in
  let t = create ~presets:(P.Preset.Collection.create [ preset ] |> ok) () in
  let node, _, operations = open_popup_details t in
  let button, initial_handler =
    List.find_map_exn operations ~f:(function
      | W.Op.Create (node, Button, label, handler)
        when String.equal label "Named date shortcut" -> Some (node, handler)
      | _ -> None)
  in
  assert (Option.is_none initial_handler);
  let handler =
    List.fold operations ~init:initial_handler ~f:(fun handler -> function
      | W.Op.Bind (node, next) when Gpuio_protocol.Node_id.equal node button -> next
      | _ -> handler)
    |> Option.value_exn
  in
  Driver.dispatch
    t.driver
    (W.Event.Press (window, button, handler, Driver.revision t.driver));
  ignore (cycle t : W.Op.t list);
  assert (Picker.is_selecting_preset (current t));
  let replace = Queue.dequeue_exn t.requests in
  finish t replace node 1L selected;
  assert (Picker.is_open (current t) && Queue.is_empty t.commits);
  assert (
    C.Selection.equal
      (C.Snapshot.selection (Picker.draft (current t) |> Option.value_exn))
      selected);
  print_endline
    "named button, full-length stable ID, native replacement, explicit Apply retained";
  [%expect
    {| named button, full-length stable ID, native replacement, explicit Apply retained |}]
;;

let%expect_test "rich date trigger updates preserve the native button and open draft" =
  let t = create () in
  let button =
    List.find_map_exn t.initial ~f:(function
      | W.Op.Create (id, Button, "Choose date", _) -> Some id
      | _ -> None)
  in
  B.Expert.Var.set
    t.trigger
    (Some (Gpuio_bonsai.View.row [ Gpuio_bonsai.View.text "29 Feb" ]));
  let rich = cycle t in
  assert (
    not
      (List.exists rich ~f:(function
         | W.Op.Create (_, Button, _, _) -> true
         | _ -> false)));
  assert (
    List.exists rich ~f:(function
      | W.Op.Set_button_presentation (id, Some _) ->
        Gpuio_protocol.Node_id.equal id button
      | _ -> false));
  let node, _ = open_popup t in
  B.Expert.Var.set
    t.trigger
    (Some (Gpuio_bonsai.View.row [ Gpuio_bonsai.View.text "Thursday, February 29" ]));
  let changed = cycle t in
  assert (
    not
      (List.exists changed ~f:(function
         | W.Op.Create _ | Remove _ -> true
         | _ -> false)));
  assert (Picker.is_open (current t));
  assert (
    Gpuio_protocol.Node_id.equal
      node
      (C.Expert.node (Picker.draft (current t) |> Option.value_exn)));
  assert (Queue.is_empty t.requests && Queue.is_empty t.commits);
  let invalid trigger name =
    Result.is_error
      (Picker.view_with_trigger ~overlay ~accessible_name:name ~trigger (current t))
  in
  assert (invalid (Gpuio_bonsai.View.button ~on_click:E.Ignore "Nested action") "Date");
  assert (invalid (Gpuio_bonsai.View.text "Date") " ");
  assert (invalid (Gpuio_bonsai.View.text "Date") (String.make 1025 'x'));
  print_endline
    "plain/rich button identity retained; text refresh keeps draft; nested actions and \
     invalid names rejected";
  [%expect
    {| plain/rich button identity retained; text refresh keeps draft; nested actions and invalid names rejected |}]
;;

let%expect_test "calendar content updates preserve pending picker confirmation" =
  let t = create () in
  let content text =
    Gpuio.View.Calendar_content.create
      [ Gpuio.View.Calendar_content.Item.create
          ~slot:C.Slot.next
          (Gpuio_bonsai.View.text text)
        |> ok
      ]
    |> ok
  in
  B.Expert.Var.set t.content (Some (content "Next artwork"));
  ignore (cycle t : W.Op.t list);
  let node, _ = open_popup t in
  ignore_run t (E.map (Picker.confirm (current t)) ~f:ignore);
  let pending = Queue.dequeue_exn t.requests in
  let draft = Picker.draft (current t) |> Option.value_exn in
  B.Expert.Var.set t.content (Some (content "Updated artwork"));
  let changed = cycle t in
  assert (
    List.exists changed ~f:(function
      | W.Op.Set_text (_, "Updated artwork") -> true
      | _ -> false));
  assert (
    not
      (List.exists changed ~f:(function
         | W.Op.Create _ | Remove _ | Set_calendar _ | Set_calendar_content _ -> true
         | _ -> false)));
  assert (C.Snapshot.equal (Picker.draft (current t) |> Option.value_exn) draft);
  assert (Queue.is_empty t.requests && Queue.is_empty t.commits);
  finish t pending node 1L selected;
  assert (not (Picker.is_open (current t)));
  assert (C.Selection.equal (Queue.dequeue_exn t.commits) selected);
  print_endline "content edit retains draft, native owner and pending confirmation";
  [%expect {| content edit retains draft, native owner and pending confirmation |}]
;;

let%expect_test
    "viewport content loading does not invalidate pending confirmation or cross popup \
     sessions"
  =
  let observations = Queue.create () in
  let t =
    create
      ~on_calendar_viewport_change:(fun viewport ->
        E.of_thunk (fun () -> Queue.enqueue observations viewport))
      ()
  in
  let node, _, ops = open_popup_details t in
  let observer =
    List.find_map_exn ops ~f:(function
      | W.Op.Set_calendar_viewport_observer (_, Some handler) -> Some handler
      | _ -> None)
  in
  let event sequence =
    W.Event.Calendar_viewport_changed
      ( window
      , node
      , observer
      , Driver.revision t.driver
      , Gpuio_protocol.Calendar_viewport_wire.
          { sequence
          ; display =
              Days
                { first_month = C.Expert.month_to_wire month
                ; months = 2L
                ; first_weekday = 1L
                }
          } )
  in
  Driver.dispatch t.driver (event 0L);
  ignore (cycle t : W.Op.t list);
  assert (Queue.length observations = 1);
  ignore_run t (E.map (Picker.confirm (current t)) ~f:ignore);
  let pending = Queue.dequeue_exn t.requests in
  let draft = Picker.draft (current t) |> Option.value_exn in
  let late = event 2L in
  Driver.dispatch t.driver (event 1L);
  ignore (cycle t : W.Op.t list);
  assert (Queue.length observations = 2);
  assert (C.Snapshot.equal draft (Picker.draft (current t) |> Option.value_exn));
  assert (Queue.is_empty t.requests);
  finish t pending node 1L selected;
  assert (not (Picker.is_open (current t)));
  assert (C.Selection.equal (Queue.dequeue_exn t.commits) selected);
  ignore (open_popup t : Gpuio_protocol.Node_id.t * Gpuio_protocol.Handler_id.t);
  Driver.dispatch t.driver late;
  ignore (cycle t : W.Op.t list);
  assert (Queue.length observations = 2);
  print_endline
    "viewport callback leaves confirmation intact; closed popup observations retired";
  [%expect
    {| viewport callback leaves confirmation intact; closed popup observations retired |}]
;;
