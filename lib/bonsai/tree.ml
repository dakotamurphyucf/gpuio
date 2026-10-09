open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module T = Gpuio.Tree
module S = Gpuio.Tree_state
module I = Gpuio.Tree_interaction
module L = Gpuio.Tree_loading
module Rows = Gpuio.Tree_rows
module V = Tree_rows
module Target = I.Target

module Load = struct
  type t =
    | Request
    | Retry
    | Cancel
  [@@deriving sexp_of]
end

module Controller = struct
  type t =
    { dispatch : I.Request.t -> unit E.t
    ; load : Target.t -> Load.t -> unit E.t
    }

  let dispatch t = t.dispatch

  let select t ?(gesture = S.Selection.Replace) target =
    t.dispatch (I.Request.select target gesture)
  ;;

  let set_selected t target selected = t.dispatch (I.Request.set_selected target selected)
  let set_expanded t target expanded = t.dispatch (I.Request.set_expanded target expanded)
  let toggle_expanded t target = t.dispatch (I.Request.toggle_expanded target)
  let reveal t ?(focus = false) target = t.dispatch (I.Request.reveal target ~focus)
  let activate t target = t.dispatch (I.Request.activate target)

  let propose_move t ~source ~destination placement =
    t.dispatch (I.Request.move ~source ~destination placement)
  ;;

  let request t target = t.load target Load.Request
  let retry t target = t.load target Load.Retry
  let cancel t target = t.load target Load.Cancel
end

module Output = struct
  type 'data t =
    { rows : 'data V.Output.t
    ; controller : Controller.t
    ; view : unit E.t Gpuio.View.t
    }

  let view t = t.view
  let projection t = V.Output.projection t.rows
  let state t = Rows.state (projection t)
  let controller t = t.controller
  let target t id = Target.capture (Rows.source (projection t)) id
  let viewport t = V.Output.viewport t.rows
  let active_rows t = V.Output.active_rows t.rows
  let budget_exhausted t = V.Output.budget_exhausted t.rows
end

module Pending = struct
  type t =
    { serial : int64
    ; target : Target.t
    ; focus : bool
    }
end

module Model = struct
  type t =
    { state : S.t option
    ; serial : int64
    ; pending : Pending.t option
    }

  let empty = { state = None; serial = 0L; pending = None }
end

module Action = struct
  type t =
    | Sync
    | Request of (I.Request.t[@sexp.opaque])
    | Load of (Target.t[@sexp.opaque]) * Load.t
    | Finish of int64
  [@@deriving sexp_of]
end

type 'data input =
  { source : 'data L.Snapshot.t
  ; mode : S.Mode.t
  ; initial_selected : T.Id.t list
  ; initial_expanded : T.Id.t list
  ; auto_load : bool
  ; loading : V.Loading.t option
  ; on_action : I.Action.t -> unit E.t
  ; lifetime : Managed_rows.Lifetime.t
  }

let state model input =
  let tree = L.Snapshot.tree input.source in
  match model.Model.state with
  | None ->
    S.create
      tree
      ~mode:input.mode
      ~selected:input.initial_selected
      ~expanded:input.initial_expanded
      ()
  | Some state -> Ok (S.with_mode state tree input.mode)
;;

let eligible source state target =
  Target.is_current target source
  && Option.is_some (S.visible_index state (Target.id target))
  && Option.exists
       (T.find (L.Snapshot.tree source) (Target.id target))
       ~f:(fun node -> not (T.Node.is_disabled node))
;;

let pending model input state =
  Option.filter model.Model.pending ~f:(fun pending ->
    eligible input.source state pending.Pending.target)
;;

let expansion_requests input ~previous ~state ~max_active =
  let available =
    Int.min max_active (L.max_queued - L.Snapshot.queued_count input.source)
  in
  S.fold_changed_items
    state
    ~previous
    ~init:(available, [])
    ~f:(fun (available, targets) id ->
      if
        available <= 0
        || S.is_expanded previous id
        || (not (S.is_expanded state id))
        || Option.is_none (S.visible_index state id)
      then available, targets
      else (
        match
          L.Snapshot.status input.source id, T.find (L.Snapshot.tree input.source) id
        with
        | Some Ready, Some node when not (T.Node.is_disabled node) ->
          (match L.Snapshot.target input.source id with
           | Error _ -> available, targets
           | Ok target -> available - 1, target :: targets)
        | (None | Some (Ready | Queued | Loading | End | Failed _)), _ ->
          available, targets))
  |> snd
  |> List.rev
;;

let apply_action ~max_active context input model action =
  match input with
  | B.Computation_status.Inactive -> model
  | Active input ->
    (match state model input with
     | Error _ -> model
     | Ok state ->
       let pending = pending model input state in
       let model = { model with Model.state = Some state; pending } in
       let schedule event =
         B.Apply_action_context.schedule_event
           context
           (Managed_rows.Lifetime.guard input.lifetime event)
       in
       (match action with
        | Action.Sync -> model
        | Finish serial ->
          if
            Option.exists pending ~f:(fun pending ->
              Int64.equal serial pending.Pending.serial)
          then { model with pending = None }
          else model
        | Load (target, operation) ->
          (match input.loading with
           | None -> ()
           | Some loading ->
             let visible =
               match operation with
               | Load.Cancel -> true
               | Request | Retry ->
                 eligible input.source state target
                 && S.is_expanded state (Target.id target)
             in
             if visible && Target.is_current target input.source
             then (
               match L.Snapshot.target input.source (Target.id target) with
               | Error _ -> ()
               | Ok target ->
                 schedule
                   (match operation with
                    | Request -> V.Loading.request loading target
                    | Retry -> V.Loading.retry loading target
                    | Cancel -> V.Loading.cancel loading target)));
          model
        | Request request ->
          (match I.apply state input.source request with
           | None -> model
           | Some outcome ->
             if Int64.equal model.serial Int64.max_value
             then failwith "tree command serial exhausted";
             let serial = Int64.succ model.serial in
             let previous = state in
             let state = I.Outcome.state outcome in
             (match input.loading with
              | Some loading when input.auto_load ->
                expansion_requests input ~previous ~state ~max_active
                |> List.iter ~f:(fun target ->
                  schedule (V.Loading.request loading target))
              | None | Some _ -> ());
             let pending =
               match I.Outcome.reveal outcome with
               | Some target ->
                 Some { Pending.serial; target; focus = I.Outcome.focus outcome }
               | None ->
                 Option.filter pending ~f:(fun pending ->
                   eligible input.source state pending.target)
             in
             (match I.Outcome.action outcome with
              | None -> ()
              | (Activate _ | Move _) as action -> schedule (input.on_action action));
             { Model.state = Some state; serial; pending })))
;;

let render_row
      ~source
      ~config
      ~controller
      ~loading
      ~render_item
      ~key:_
      ~data
      ~lifetime
      graph
  =
  let open B.Let_syntax in
  match%sub data with
  | Rows.Row.Item item ->
    let target =
      let%arr source = source
      and item = item in
      Target.capture source item.Rows.Item.id |> Or_error.ok_exn
    in
    let content = render_item ~target ~item ~controller ~lifetime graph in
    let%arr item = item
    and target = target
    and content = content
    and controller = controller
    and lifetime = lifetime in
    let on_toggle =
      Managed_rows.Lifetime.guard
        lifetime
        (E.Many
           [ Controller.dispatch controller (I.Request.focus target)
           ; Controller.toggle_expanded controller target
           ])
    in
    Tree_presentation.item item ~config ~content ~on_toggle
  | Boundary boundary ->
    let%arr source = source
    and boundary = boundary
    and controller = controller
    and loading = loading
    and lifetime = lifetime in
    let target = Target.capture source boundary.Rows.Boundary.parent |> Or_error.ok_exn in
    Tree_presentation.boundary
      boundary
      ~config
      ~can_load:(Option.is_some loading)
      ~request:
        (Managed_rows.Lifetime.guard lifetime (Controller.request controller target))
      ~retry:(Managed_rows.Lifetime.guard lifetime (Controller.retry controller target))
;;

let inner
      source
      ~mode
      ~initial_selected
      ~initial_expanded
      ~loading
      ~on_action
      ~allow_moves
      ~lifetime
      ~config
      ~scrollbar
      ~label
      ~auto_load
      ~cancel_hidden
      ~render_item
      graph
  =
  let open B.Let_syntax in
  let optional_loading = loading in
  let loading = B.transpose_opt loading in
  let input =
    let%arr source = source
    and mode = mode
    and loading = loading
    and on_action = on_action
    and lifetime = lifetime
    and initial_selected = initial_selected
    and initial_expanded = initial_expanded
    and auto_load = auto_load in
    { source
    ; mode
    ; loading
    ; on_action
    ; lifetime
    ; initial_selected
    ; initial_expanded
    ; auto_load
    }
  in
  let model, inject =
    B.state_machine1
      ~default_model:Model.empty
      ~equal:phys_equal
      ~sexp_of_action:Action.sexp_of_t
      ~apply_action:
        (apply_action ~max_active:(Gpuio.Virtual_list.Config.max_active config))
      input
      graph
  in
  let controller =
    let%arr inject = inject
    and lifetime = lifetime in
    { Controller.dispatch =
        (fun request -> Managed_rows.Lifetime.guard lifetime (inject (Request request)))
    ; load =
        (fun target operation ->
          Managed_rows.Lifetime.guard lifetime (inject (Load (target, operation))))
    }
  in
  let state =
    let%arr model = model
    and input = input in
    state model input
  in
  let output =
    match%sub state with
    | Error error ->
      let%arr error = error in
      Error error
    | Ok state ->
      let accessibility =
        let%arr state = state in
        Gpuio.Accessibility.create
          ~role:(Tree (S.Mode.equal (S.mode state) Multiple))
          ~label
          ()
      in
      (match%sub accessibility with
       | Error error ->
         let%arr error = error in
         Error error
       | Ok accessibility ->
         let on_request =
           B.map controller ~f:(fun controller -> Controller.dispatch controller)
         in
         let rows =
           V.component
             source
             ~state
             ~config
             ~scrollbar
             ~accessibility
             ~on_request
             ~allow_moves
             ?loading:optional_loading
             ~auto_load
             ~cancel_hidden
             ~render_row:(render_row ~source ~config ~controller ~loading ~render_item)
             graph
         in
         let%arr rows = rows
         and controller = controller in
         Or_error.map rows ~f:(fun rows ->
           { Output.rows; controller; view = V.Output.view rows }))
  in
  let observed = B.both output model in
  let peek = B.peek observed graph in
  let after_display =
    let%arr output = output
    and model = model
    and inject = inject
    and peek = peek
    and lifetime = lifetime in
    match output with
    | Error _ -> E.Ignore
    | Ok output ->
      let source = Rows.source (Output.projection output) in
      let state = Output.state output in
      let current =
        Option.filter model.pending ~f:(fun p -> eligible source state p.target)
      in
      let sync =
        if
          Option.exists model.state ~f:(phys_equal state)
          && Bool.equal (Option.is_some current) (Option.is_some model.pending)
        then E.Ignore
        else inject Sync
      in
      let reveal =
        match current with
        | None -> E.Ignore
        | Some pending ->
          E.bind peek ~f:(function
            | B.Computation_status.Inactive | Active (Error _, _) -> E.Ignore
            | Active (Ok output, model) ->
              let rows = Output.projection output in
              if
                (not
                   (Option.exists model.pending ~f:(fun p ->
                      Int64.equal p.serial pending.serial)))
                || not (eligible (Rows.source rows) (Rows.state rows) pending.target)
              then E.Ignore
              else (
                match Rows.item_key rows (Target.id pending.target) with
                | None -> inject (Finish pending.serial)
                | Some key ->
                  E.Many
                    [ inject (Finish pending.serial)
                    ; V.Controller.reveal
                        (V.Output.controller output.rows)
                        ~focus:pending.focus
                        key
                    ]))
      in
      Managed_rows.Lifetime.guard lifetime (E.Many [ sync; reveal ])
  in
  B.Edge.after_display after_display graph;
  output
;;

let fill =
  Gpuio.Style.create_exn
    [ Width (Gpuio.Length.percent_exn 100.)
    ; Height (Gpuio.Length.percent_exn 100.)
    ; Min_width (Gpuio.Length.px_exn 0.)
    ; Min_height (Gpuio.Length.px_exn 0.)
    ]
;;

let component
      source
      ~config
      ~label
      ?key
      ?(style = B.return fill)
      ?(scrollbar = B.return None)
      ?(mode = B.return S.Mode.Single)
      ?(initial_selected = B.return [])
      ?(initial_expanded = B.return [])
      ?loading
      ?(auto_load = B.return true)
      ?(cancel_hidden = B.return true)
      ?(allow_moves = B.return false)
      ?(on_action = B.return (fun _ -> E.Ignore))
      ?(render_item =
        fun ~target:_ ~item ~controller:_ ~lifetime:_ _ ->
          B.map item ~f:Tree_presentation.label)
      graph
  =
  let open B.Let_syntax in
  let generations =
    B.map source ~f:(fun source ->
      Map.singleton (module L.Lease) (L.Snapshot.lease source) source)
  in
  let outputs =
    Managed_rows.assoc
      (module L.Lease)
      generations
      ~f:(fun _ source lifetime graph ->
        inner
          source
          ~mode
          ~initial_selected
          ~initial_expanded
          ~loading
          ~on_action
          ~allow_moves
          ~lifetime
          ~config
          ~scrollbar
          ~label
          ~auto_load
          ~cancel_hidden
          ~render_item
          graph)
      graph
  in
  let%arr outputs = outputs
  and style = style in
  Map.data outputs
  |> List.hd_exn
  |> Or_error.map ~f:(fun output ->
    let style =
      Gpuio.Style.merge
        [ Gpuio.Style.create_exn [ Foreground (Gpuio.Color.token_exn "foreground") ]
        ; style
        ]
    in
    let view = Gpuio.View.column ?key ~style [ Output.view output ] in
    { output with view })
;;
