open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module C = Gpuio.Calendar
module P = Gpuio.Date_picker
module View = Gpuio_bonsai.View

type command =
  C.Snapshot.t -> C.Command.t -> (C.Snapshot.t, C.Command_error.t) Result.t E.t

type input =
  { config : C.Config.t
  ; value : C.Selection.t
  ; on_change : C.Selection.t -> unit E.t
  }

type result = (C.Selection.t, P.Error.t) Result.t

type action =
  | Sync
  | Select_preset of
      P.Session.Id.t * P.Preset.t * ((C.Snapshot.t, P.Error.t) Result.t -> unit)
  | Preset_finished of
      P.Session.Id.t
      * C.Snapshot.t
      * (C.Snapshot.t, C.Command_error.t) Result.t
      * ((C.Snapshot.t, P.Error.t) Result.t -> unit)
  | Confirm of P.Session.Id.t * (result -> unit)
  | Open
  | Cancel of P.Session.Id.t
  | Observe of P.Session.Id.t * C.Snapshot.t
  | Native of P.Session.Id.t * C.Snapshot.t
  | Finish of
      P.Session.Id.t
      * C.Snapshot.t
      * (C.Snapshot.t, C.Command_error.t) Result.t
      * (result -> unit)

type t =
  { native_command : command
  ; key : string
  ; config : C.Config.t
  ; model : P.t
  ; session : P.Session.t option
  ; pending : P.Session.Id.t option
  ; inject : action -> unit E.t
  }

let same_lease left right =
  Gpuio_protocol.Window_id.equal (C.Expert.window left) (C.Expert.window right)
  && Gpuio_protocol.Node_id.equal (C.Expert.node left) (C.Expert.node right)
;;

let create native_command ~config ~value ~initial_month ~on_change graph =
  let open B.Let_syntax in
  let input =
    let%arr config = config
    and value = value
    and on_change = on_change in
    { config; value; on_change }
  in
  let state, inject =
    B.state_machine1
      ~default_model:(P.empty, None)
      ~equal:[%equal: P.t * P.Session.Id.t option]
      ~apply_action:(fun context input (model, pending) action ->
        let schedule = B.Apply_action_context.schedule_event context in
        let inject = B.Apply_action_context.inject context in
        let reply callback result = schedule (E.of_thunk (fun () -> callback result)) in
        match input with
        | Bonsai.Computation_status.Inactive ->
          (match action with
           | Confirm (_, callback) | Finish (_, _, _, callback) ->
             reply callback (Error P.Error.Not_open)
           | Select_preset (_, _, callback) | Preset_finished (_, _, _, callback) ->
             reply callback (Error P.Error.Not_open)
           | Sync | Open | Cancel _ | Observe _ | Native _ -> ());
          (match action with
           | Cancel session -> P.cancel model ~session, None
           | _ -> model, pending)
        | Active { config; value; on_change } ->
          let model = P.sync model ~config ~value in
          let current_session = P.session model ~config ~value in
          let matches id =
            Option.exists current_session ~f:(fun current ->
              P.Session.Id.equal (P.Session.id current) id)
          in
          let pending = Option.filter pending ~f:matches in
          let ready session =
            if not (matches session)
            then Error P.Error.Stale_session
            else if Option.is_some pending
            then Error (P.Error.Native Busy)
            else Result.of_option (P.draft model) ~error:P.Error.Not_ready
          in
          (match action with
           | Sync -> model, pending
           | Open -> P.open_popup model ~config ~value ~initial_month, pending
           | Cancel session ->
             P.cancel model ~session, if matches session then None else pending
           | Observe (session, snapshot) -> P.observe model ~session snapshot, pending
           | Native (session, snapshot) ->
             P.observe_native model ~session snapshot, pending
           | Select_preset (session, preset, callback) ->
             let request =
               let open Result.Let_syntax in
               let%bind snapshot = ready session in
               let%map () = P.Preset.validate preset ~config in
               snapshot
             in
             (match request with
              | Error error ->
                reply callback (Error error);
                model, pending
              | Ok snapshot ->
                schedule
                  (let open E.Let_syntax in
                   let%bind result =
                     native_command
                       snapshot
                       (C.Command.Replace
                          { selection = P.Preset.selection preset
                          ; if_revision = Some (C.Snapshot.revision snapshot)
                          })
                   in
                   inject (Preset_finished (session, snapshot, result, callback)));
                model, Some session)
           | Preset_finished (session, expected, result, callback) ->
             if not (matches session)
             then (
               reply callback (Error P.Error.Stale_session);
               model, pending)
             else (
               let model, result =
                 if not (Option.exists (P.draft model) ~f:(same_lease expected))
                 then model, Error P.Error.Stale_draft
                 else (
                   match result with
                   | Error error ->
                     let error = P.Error.Native error in
                     P.failed model ~session error, Error error
                   | Ok snapshot ->
                     let observed = P.observe model ~session snapshot in
                     if Option.exists (P.draft observed) ~f:(C.Snapshot.equal snapshot)
                     then observed, Ok snapshot
                     else model, Error P.Error.Stale_draft)
               in
               reply callback result;
               model, None)
           | Confirm (session, callback) ->
             (match ready session with
              | Error error -> reply callback (Error error)
              | Ok snapshot ->
                schedule
                  (let open E.Let_syntax in
                   let%bind result = native_command snapshot Read_snapshot in
                   inject (Finish (session, snapshot, result, callback))));
             model, pending
           | Finish (session, expected, result, callback) ->
             let model, result =
               if not (matches session)
               then model, Error P.Error.Stale_session
               else if Option.is_some pending
               then model, Error (P.Error.Native Busy)
               else (
                 match result with
                 | Ok snapshot -> P.confirm model ~config ~value ~session snapshot
                 | Error error ->
                   let error =
                     if C.Command_error.equal error Closed || matches session
                     then P.Error.Native error
                     else P.Error.Stale_session
                   in
                   let model =
                     if Option.exists (P.draft model) ~f:(same_lease expected)
                     then P.failed model ~session error
                     else model
                   in
                   model, Error error)
             in
             schedule
               (E.Many
                  [ (match result with
                     | Ok selection -> on_change selection
                     | Error _ -> E.Ignore)
                  ; E.of_thunk (fun () -> callback result)
                  ]);
             model, pending))
      input
      graph
  in
  let model = B.map state ~f:fst in
  let changes = B.map input ~f:(fun input -> input.config, input.value) in
  B.Edge.lifecycle
    ~on_deactivate:
      (let%arr model = model
       and input = input
       and inject = inject in
       Option.value_map
         (P.session model ~config:input.config ~value:input.value)
         ~default:E.Ignore
         ~f:(fun session -> inject (Cancel (P.Session.id session))))
    graph;
  B.Edge.on_change
    changes
    ~equal:[%equal: C.Config.t * C.Selection.t]
    ~callback:(B.map inject ~f:(fun inject _ -> inject Sync))
    graph;
  let key = B.path_id graph in
  let%arr model, pending = state
  and inject = inject
  and input = input
  and key = key in
  let model = P.sync model ~config:input.config ~value:input.value in
  { native_command
  ; key
  ; config = input.config
  ; model
  ; session = P.session model ~config:input.config ~value:input.value
  ; pending =
      Option.filter pending ~f:(fun id ->
        Option.exists
          (P.session model ~config:input.config ~value:input.value)
          ~f:(fun s -> P.Session.Id.equal (P.Session.id s) id))
  ; inject
  }
;;

let is_open t = Option.is_some t.session
let is_selecting_preset t = Option.is_some t.pending
let draft t = P.draft t.model
let error t = P.error t.model

let can_confirm t =
  is_open t
  && (not (is_selecting_preset t))
  && (not (C.Config.is_disabled t.config || C.Config.is_read_only t.config))
  && Option.exists (draft t) ~f:(fun snapshot ->
    let selection = C.Snapshot.selection snapshot in
    (match selection with
     | C.Selection.Range_start _ -> false
     | Empty | Single _ | Range _ -> true)
    && C.Constraints.allows_selection
         (C.Config.constraints t.config)
         selection
         ~mode:(C.Config.mode t.config))
;;

let open_popup t = t.inject Open

let cancel t =
  Option.value_map t.session ~default:E.Ignore ~f:(fun session ->
    t.inject (Cancel (P.Session.id session)))
;;

let command t command =
  match t.session, draft t with
  | Some session, Some snapshot ->
    let open E.Let_syntax in
    let%bind result = t.native_command snapshot command in
    let%map () =
      match result with
      | Ok snapshot -> t.inject (Observe (P.Session.id session, snapshot))
      | Error _ -> E.Ignore
    in
    result
  | None, _ | _, None -> E.return (Error C.Command_error.Not_mounted)
;;

let request t action =
  E.Expert.of_fun ~f:(fun ~callback ->
    E.Expert.eval (t.inject (action callback)) ~f:(fun () -> ()))
;;

let confirm t =
  match t.session with
  | None -> E.return (Error P.Error.Not_open)
  | Some session -> request t (fun callback -> Confirm (P.Session.id session, callback))
;;

let select_preset t preset =
  match t.session with
  | None -> E.return (Error P.Error.Not_open)
  | Some session ->
    request t (fun callback -> Select_preset (P.Session.id session, preset, callback))
;;

let popup
      ?style
      ?appearance
      ?calendar_content
      ?on_calendar_viewport_change
      ?(presets = P.Preset.Collection.empty)
      ?(apply_label = "Apply")
      ?(cancel_label = "Cancel")
      ~overlay
      ~anchor
      t
  =
  let on_event session event =
    let snapshot =
      match event with
      | C.Event.Observed s | Changed s | Selected s | Rejected (_, s) -> s
    in
    t.inject (Native (P.Session.id session, snapshot))
  in
  let content =
    Option.map t.session ~f:(fun session ->
      let calendar =
        Gpuio.View.calendar
          ?appearance
          ?content:calendar_content
          ?on_viewport_change:on_calendar_viewport_change
          ~controller:
            (Gpuio.Key.of_string_exn
               (t.key
                ^ ":calendar:"
                ^ Int64.to_string (P.Session.Id.to_int64 (P.Session.id session))))
          ~config:t.config
          ~initial:(P.Session.initial session)
          ~initial_month:(P.Session.initial_month session)
          ~on_event:(on_event session)
          ()
      in
      let preset_rows =
        match P.Preset.Collection.to_list presets with
        | [] -> []
        | items ->
          [ View.row
              ~style:(Gpuio.Style.create_exn [ Gap (Gpuio.Length.px_exn 8.); Wrap Wrap ])
              (List.map items ~f:(fun preset ->
                 View.button
                   ~key:
                     (Gpuio.Key.of_string_exn
                        (Gpuio.Choice.Id.to_string (P.Preset.id preset)))
                   ~disabled:
                     (is_selecting_preset t
                      || Option.is_none (draft t)
                      || Result.is_error (P.Preset.validate preset ~config:t.config))
                   ~on_click:(E.map (select_preset t preset) ~f:(fun _ -> ()))
                   (P.Preset.label preset)))
          ]
      in
      View.column
        ~style:(Gpuio.Style.create_exn [ Gap (Gpuio.Length.px_exn 12.) ])
        (preset_rows
         @ [ calendar
           ; View.row
               ~style:(Gpuio.Style.create_exn [ Gap (Gpuio.Length.px_exn 8.) ])
               [ View.button ~on_click:(cancel t) cancel_label
               ; View.button
                   ~disabled:(not (can_confirm t))
                   ~on_click:(E.map (confirm t) ~f:(fun _ -> ()))
                   apply_label
               ]
           ]))
  in
  View.popover
    ?style
    ~key:(Gpuio.Key.of_string_exn (t.key ^ ":popup"))
    ~config:overlay
    ~anchor
    ~on_dismiss:(fun _ -> cancel t)
    content
;;

let trigger_action t = if is_open t then cancel t else open_popup t

let view
      ?style
      ?trigger_style
      ?appearance
      ?calendar_content
      ?on_calendar_viewport_change
      ?presets
      ?apply_label
      ?cancel_label
      ~overlay
      ~label
      t
  =
  let anchor =
    View.button
      ?style:trigger_style
      ~disabled:(C.Config.is_disabled t.config)
      ~on_click:(trigger_action t)
      label
  in
  popup
    ?style
    ?appearance
    ?calendar_content
    ?on_calendar_viewport_change
    ?presets
    ?apply_label
    ?cancel_label
    ~overlay
    ~anchor
    t
;;

let view_with_trigger
      ?style
      ?trigger_style
      ?appearance
      ?calendar_content
      ?on_calendar_viewport_change
      ?presets
      ?apply_label
      ?cancel_label
      ~overlay
      ~accessible_name
      ~trigger
      t
  =
  let%map.Or_error anchor =
    View.button_with_content
      ?style:trigger_style
      ~disabled:(C.Config.is_disabled t.config)
      ~accessible_name
      ~on_click:(trigger_action t)
      trigger
  in
  popup
    ?style
    ?appearance
    ?calendar_content
    ?on_calendar_viewport_change
    ?presets
    ?apply_label
    ?cancel_label
    ~overlay
    ~anchor
    t
;;
