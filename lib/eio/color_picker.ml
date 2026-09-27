open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module C = Gpuio.Color_input
module P = Gpuio.Color_picker
module View = Gpuio_bonsai.View

type input =
  { config : C.Config.t
  ; value : Gpuio.Color_value.Value.t
  ; on_change : Gpuio.Color_value.Value.t -> unit E.t
  }

type result = (Gpuio.Color_value.Value.t, P.Error.t) Result.t

type action =
  | Sync
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
  { window : App.Window.t
  ; key : string
  ; config : C.Config.t
  ; model : P.t
  ; session : P.Session.t option
  ; inject : action -> unit E.t
  }

let create window ~config ~value ~on_change graph =
  let open B.Let_syntax in
  let input =
    let%arr config = config
    and value = value
    and on_change = on_change in
    { config; value; on_change }
  in
  let model, inject =
    B.state_machine1
      ~default_model:P.empty
      ~equal:P.equal
      ~apply_action:(fun context input model action ->
        let schedule event = B.Apply_action_context.schedule_event context event in
        match input with
        | Bonsai.Computation_status.Inactive ->
          (match action with
           | Finish (_, _, reply, callback) ->
             let error =
               match reply with
               | Error error -> P.Error.Native error
               | Ok _ -> P.Error.Not_open
             in
             schedule (E.of_thunk (fun () -> callback (Error error)))
           | Sync | Open | Cancel _ | Observe _ | Native _ -> ());
          (match action with
           | Cancel session -> P.cancel model ~session
           | _ -> model)
        | Active { config; value; on_change } ->
          let model = P.sync model ~config ~value in
          (match action with
           | Sync -> model
           | Open -> P.open_popup model ~config ~value
           | Cancel session -> P.cancel model ~session
           | Observe (session, snapshot) -> P.observe model ~session snapshot
           | Native (session, snapshot) -> P.observe_native model ~session snapshot
           | Finish (session, expected, reply, callback) ->
             let model, result =
               match reply with
               | Ok snapshot -> P.confirm model ~config ~value ~session snapshot
               | Error error ->
                 let error =
                   if C.Command_error.equal error Closed
                   then P.Error.Native error
                   else if
                     Option.exists (P.session model ~config ~value) ~f:(fun current ->
                       P.Session.Id.equal (P.Session.id current) session)
                   then P.Error.Native error
                   else P.Error.Stale_session
                 in
                 let model =
                   if
                     Option.exists (P.draft model) ~f:(fun current ->
                       Gpuio_protocol.Window_id.equal
                         (C.Expert.window current)
                         (C.Expert.window expected)
                       && Gpuio_protocol.Node_id.equal
                            (C.Expert.node current)
                            (C.Expert.node expected))
                   then P.failed model ~session error
                   else model
                 in
                 model, Error error
             in
             schedule
               (E.Many
                  [ (match result with
                     | Ok selection -> on_change selection
                     | Error _ -> E.Ignore)
                  ; E.of_thunk (fun () -> callback result)
                  ]);
             model))
      input
      graph
  in
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
    ~equal:[%equal: C.Config.t * Gpuio.Color_value.Value.t]
    ~callback:(B.map inject ~f:(fun inject _ -> inject Sync))
    graph;
  let key = B.path_id graph in
  let%arr model = model
  and inject = inject
  and input = input
  and key = key in
  let model = P.sync model ~config:input.config ~value:input.value in
  { window
  ; key
  ; config = input.config
  ; model
  ; session = P.session model ~config:input.config ~value:input.value
  ; inject
  }
;;

let is_open t = Option.is_some t.session
let draft t = P.draft t.model
let error t = P.error t.model

let can_confirm t =
  is_open t
  && Option.exists (draft t) ~f:(fun snapshot ->
    Result.is_ok (P.candidate snapshot ~config:t.config))
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
    let%bind result = App.Window.Expert.color_input_command t.window snapshot command in
    let%map () =
      match result with
      | Ok snapshot -> t.inject (Observe (P.Session.id session, snapshot))
      | Error _ -> E.Ignore
    in
    result
  | None, _ | _, None -> E.return (Error C.Command_error.Not_mounted)
;;

let confirm t =
  match t.session, draft t with
  | None, _ -> E.return (Error P.Error.Not_open)
  | Some _, None -> E.return (Error P.Error.Not_ready)
  | Some session, Some snapshot ->
    let open E.Let_syntax in
    let%bind result =
      App.Window.Expert.color_input_command t.window snapshot Read_snapshot
    in
    E.Expert.of_fun ~f:(fun ~callback ->
      E.Expert.eval
        (t.inject (Finish (P.Session.id session, snapshot, result, callback)))
        ~f:(fun () -> ()))
;;

let view ?style ?(apply_label = "Apply") ?(cancel_label = "Cancel") ~overlay ~label t =
  let on_event session event =
    let snapshot = C.Event.snapshot event in
    t.inject (Native (P.Session.id session, snapshot))
  in
  let content =
    Option.map t.session ~f:(fun session ->
      let color_input =
        Gpuio.View.color_input
          ~controller:
            (Gpuio.Key.of_string_exn
               (t.key
                ^ ":color:"
                ^ Int64.to_string (P.Session.Id.to_int64 (P.Session.id session))))
          ~config:t.config
          ~initial:(P.Session.initial session)
          ~on_event:(on_event session)
          ()
      in
      View.column
        ~style:(Gpuio.Style.create_exn [ Gap (Gpuio.Length.px_exn 12.) ])
        [ color_input
        ; View.row
            ~style:(Gpuio.Style.create_exn [ Gap (Gpuio.Length.px_exn 8.) ])
            [ View.button ~on_click:(cancel t) cancel_label
            ; View.button
                ~disabled:(not (can_confirm t))
                ~on_click:(E.map (confirm t) ~f:(fun _ -> ()))
                apply_label
            ]
        ])
  in
  View.popover
    ?style
    ~key:(Gpuio.Key.of_string_exn (t.key ^ ":popup"))
    ~config:overlay
    ~anchor:
      (View.button
         ~disabled:(C.Config.is_disabled t.config)
         ~on_click:(if is_open t then cancel t else open_popup t)
         label)
    ~on_dismiss:(fun _ -> cancel t)
    content
;;
