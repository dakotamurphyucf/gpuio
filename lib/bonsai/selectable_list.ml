open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module C = Gpuio.List_collection
module S = Gpuio.List_selection
module Rows = Gpuio.List_rows
module I = Gpuio.List_input
module V = Virtual_list

module Interaction = struct
  type t =
    { epoch : Gpuio.Key.t
    ; mode : S.Mode.t
    ; boundary : S.Boundary.t
    ; selection_on_navigation : bool
    ; disabled : bool
    ; busy : bool
    }

  let create
        ~epoch
        ?(mode = S.Mode.Single)
        ?(boundary = S.Boundary.Stop)
        ?(selection_on_navigation = false)
        ?(disabled = false)
        ?(busy = false)
        ()
    =
    { epoch; mode; boundary; selection_on_navigation; disabled; busy }
  ;;
end

module Policy = struct
  (* No views or payload-bearing snapshots in delayed native actions. *)
  type t =
    { epoch : Gpuio.Key.t
    ; mode : S.Mode.t
    ; boundary : S.Boundary.t
    ; selection_on_navigation : bool
    ; disabled : bool
    ; query : (Gpuio.Key.t * Gpuio.Key.t) option
    }
  [@@deriving equal]

  let create (t : Interaction.t) query before after =
    let query =
      Option.bind query ~f:(fun key ->
        List.find_map (before @ after) ~f:(fun view ->
          let d = Gpuio.View.Expert.describe view in
          if Option.equal Gpuio.Key.equal d.key (Some key)
          then Option.map d.editor ~f:(fun editor -> key, editor.controller)
          else None))
    in
    { epoch = t.epoch
    ; mode = t.mode
    ; boundary = t.boundary
    ; selection_on_navigation = t.selection_on_navigation
    ; disabled = t.disabled
    ; query
    }
  ;;
end

module Epoch = struct
  type t =
    { policy : Policy.t
    ; serial : int64
    }

  let update previous policy =
    match previous with
    | Some previous when Policy.equal previous.policy policy -> previous
    | previous ->
      let serial = Option.value_map previous ~default:0L ~f:(fun p -> p.serial) in
      if Int64.equal serial Int64.max_value
      then failwith "selectable list epoch exhausted";
      { policy; serial = Int64.succ serial }
  ;;

  let key t = Gpuio.Key.of_string_exn (Int64.to_string t.serial)
end

module Action = struct
  type 'key t =
    | Confirm of 'key C.Item_ref.t * S.Confirmation.t
    | Context of 'key C.Item_ref.t
    | Cancel
  [@@deriving sexp_of]
end

module Request = struct
  type 'key t =
    | Input of 'key C.Item_ref.t I.t
    | Clear_selection
end

module Controller = struct
  type 'key t = { dispatch : 'key Request.t -> unit E.t }

  let focus t target = t.dispatch (Input (I.Focus target))
  let select t target gesture = t.dispatch (Input (I.Select (target, gesture)))
  let set_selected t target value = t.dispatch (Input (I.Set_selected (target, value)))

  let navigate t ?selection direction =
    t.dispatch (Input (I.Navigate (direction, selection)))
  ;;

  let confirm t ?target kind =
    t.dispatch
      (Input
         (match target with
          | Some target -> I.Confirm (target, kind)
          | None -> Confirm_active kind))
  ;;

  let context t ?target () =
    t.dispatch
      (Input
         (match target with
          | Some target -> I.Context target
          | None -> Context_active))
  ;;

  let cancel t = t.dispatch (Input I.Cancel)
  let clear_selection t = t.dispatch Clear_selection
end

module Row = struct
  type ('key, 'data) t =
    { item : ('key, 'data) Rows.Item.t
    ; selected : bool
    ; cursor : bool
    }

  let item t = t.item
  let is_selected t = t.selected
  let is_cursor t = t.cursor
end

module Output = struct
  type ('key, 'data, 'cmp) t =
    { view : unit E.t Gpuio.View.t
    ; list : Rows.Key.t V.Output.t
    ; projection : ('key, 'data, 'cmp) Rows.t
    ; state : ('key, 'cmp) S.t
    ; controller : 'key Controller.t
    }

  let view t = t.view
  let state t = t.state
  let controller t = t.controller
  let target t key = C.item_ref (Rows.source t.projection) key
  let viewport t = V.Output.viewport t.list
  let active_rows t = V.Output.active_rows t.list
  let budget_exhausted t = V.Output.budget_exhausted t.list
end

module Update = struct
  type 'key t =
    | Sync
    | Programmatic of 'key Request.t
    | Native of Policy.t * 'key C.Item_ref.t I.t
end

type ('key, 'cmp) input =
  { catalog : ('key, 'cmp) S.Catalog.t
  ; policy : Policy.t
  ; initial_selected : 'key list
  ; on_action : 'key Action.t -> unit E.t
  ; lifetime : Managed_rows.Lifetime.t
  }

let state model input =
  match model with
  | None ->
    S.create input.catalog ~mode:input.policy.mode ~selected:input.initial_selected ()
  | Some state -> Ok (S.with_mode state input.catalog input.policy.mode)
;;

let eligible catalog target =
  C.Identity.contains_ref (S.Catalog.identity catalog) target
  && S.Catalog.is_enabled catalog (C.Item_ref.key target)
;;

let reduce state input request ~emit =
  let catalog = input.catalog in
  let confirm target kind =
    if eligible catalog target then emit (Action.Confirm (target, kind));
    state
  in
  let context target =
    if eligible catalog target
    then (
      emit (Action.Context target);
      S.with_context state catalog (Some target))
    else state
  in
  match request with
  | Request.Clear_selection -> S.with_selected state catalog [] |> Or_error.ok_exn
  | Input request ->
    (match request with
     | I.Navigate (direction, selection) ->
       S.navigate state catalog ~boundary:input.policy.boundary ?selection direction
     | Select (target, gesture) -> S.select state catalog target gesture
     | Focus target -> S.focus state catalog target
     | Set_selected (target, selected) -> S.set_selected state catalog target selected
     | Select_active gesture ->
       Option.value_map (S.cursor state) ~default:state ~f:(fun target ->
         S.select state catalog target gesture)
     | Confirm (target, kind) -> confirm target kind
     | Confirm_active kind ->
       Option.value_map
         (S.confirm state catalog kind)
         ~default:state
         ~f:(fun (target, kind) -> confirm target kind)
     | Context target -> context target
     | Context_active -> Option.value_map (S.cursor state) ~default:state ~f:context
     | Cancel ->
       emit Action.Cancel;
       S.cancel state catalog)
;;

let apply_action context input model action =
  match input with
  | B.Computation_status.Inactive | Active (Error _) -> model
  | Active (Ok input) ->
    (match state model input with
     | Error _ -> model
     | Ok state ->
       let emit action =
         B.Apply_action_context.schedule_event
           context
           (Managed_rows.Lifetime.guard input.lifetime (input.on_action action))
       in
       let state =
         match action with
         | Update.Sync -> state
         | Programmatic _ when input.policy.disabled -> state
         | Programmatic request -> reduce state input request ~emit
         | Native (policy, request) ->
           if input.policy.disabled || not (Policy.equal policy input.policy)
           then state
           else reduce state input (Request.Input request) ~emit
       in
       Some state)
;;

let fill =
  Gpuio.Style.create_exn
    [ Width (Gpuio.Length.percent_exn 100.)
    ; Height (Gpuio.Length.percent_exn 100.)
    ; Min_width (Gpuio.Length.px_exn 0.)
    ; Min_height (Gpuio.Length.px_exn 0.)
    ]
;;

let presentation (row : (_, _) Row.t) =
  let open Gpuio.Style.Property in
  let token = Gpuio.Color.token_exn in
  Gpuio.Style.create_exn
    ([ Padding (Gpuio.Length.px_exn 8.)
     ; Grow 1.
     ; Min_width (Gpuio.Length.px_exn 0.)
     ; Border_width 1.
     ; Border_color
         (Gpuio.Color.with_opacity (token "foreground") (if row.cursor then 1. else 0.)
          |> Or_error.ok_exn)
     ; Foreground
         (token (if Rows.Item.is_disabled row.Row.item then "muted" else "foreground"))
     ]
     @
     if row.selected then [ Background (Gpuio.Background.solid (token "accent")) ] else []
    )
;;

let inner
      source
      ~layout
      ~config
      ~interaction
      ~label
      ~item_label
      ~initial_selected
      ~query
      ~before
      ~after
      ~on_action
      ~row_style
      ~render_row
      ~lifetime
      graph
  =
  let open B.Let_syntax in
  let accepted, set_accepted = B.state_opt ~equal:phys_equal graph in
  let projection =
    let%arr source = source
    and layout = layout
    and accepted = accepted in
    match accepted with
    | None -> Rows.create source ~layout
    | Some previous -> Rows.update previous source ~layout
  in
  let policy =
    let%arr interaction = interaction
    and query = query
    and before = before
    and after = after in
    Policy.create interaction query before after
  in
  let accepted_epoch, set_accepted_epoch = B.state_opt ~equal:phys_equal graph in
  let epoch =
    let%arr accepted = accepted_epoch
    and policy = policy in
    Epoch.update accepted policy
  in
  let input =
    let%arr projection = projection
    and policy = policy
    and initial_selected = initial_selected
    and on_action = on_action
    and lifetime = lifetime in
    Or_error.map projection ~f:(fun projection ->
      { catalog = Rows.Layout.catalog (Rows.layout projection)
      ; policy
      ; initial_selected
      ; on_action
      ; lifetime
      })
  in
  let model, inject =
    B.state_machine1
      ~default_model:None
      ~equal:(Option.equal phys_equal)
      ~apply_action
      input
      graph
  in
  let state =
    let%arr input = input
    and model = model in
    Or_error.bind input ~f:(state model)
  in
  let controller =
    let%arr inject = inject
    and lifetime = lifetime in
    { Controller.dispatch =
        (fun request ->
          Managed_rows.Lifetime.guard lifetime (inject (Programmatic request)))
    }
  in
  let peek = B.peek projection graph in
  let on_input =
    let%arr peek = peek
    and policy = policy
    and inject = inject
    and lifetime = lifetime in
    fun request ->
      Managed_rows.Lifetime.guard
        lifetime
        (E.bind peek ~f:(function
           | B.Computation_status.Inactive | Active (Error _) -> E.Ignore
           | Active (Ok projection) ->
             (match
                I.filter_map request ~f:(fun key ->
                  Option.map (Rows.find projection key) ~f:Rows.Item.target)
              with
              | None -> E.Ignore
              | Some request -> inject (Native (policy, request)))))
  in
  let prepared =
    let%arr projection = projection
    and state = state
    and interaction = interaction
    and query = query
    and before = before
    and after = after
    and on_input = on_input
    and epoch = epoch in
    let open Or_error.Let_syntax in
    let%bind projection = projection in
    let%bind state = state in
    let%bind accessibility =
      Gpuio.Accessibility.create
        ~role:(List_box (S.Mode.equal (S.mode state) Multiple))
        ~label
        ()
    in
    let%map input =
      V.Input.create
        ~epoch:(Epoch.key epoch)
        ?cursor:(Option.bind (S.cursor state) ~f:(Rows.item_key projection))
        ?query
        ~before
        ~after
        ~selection_on_navigation:interaction.selection_on_navigation
        ~disabled:interaction.disabled
        ~busy:interaction.busy
        ~on_input
        ()
    in
    projection, state, accessibility, input
  in
  let output =
    match%sub prepared with
    | Error error ->
      let%arr error = error in
      Error error
    | Ok prepared ->
      let collection = B.map prepared ~f:(fun (p, _, _, _) -> Rows.collection p) in
      let state = B.map prepared ~f:(fun (_, s, _, _) -> s) in
      let accessibility = B.map prepared ~f:(fun (_, _, a, _) -> a) in
      let input = B.map prepared ~f:(fun (_, _, _, i) -> i) in
      let render ~key:_ ~data:item ~lifetime graph =
        let row =
          let%arr item = item
          and state = state in
          { Row.item
          ; selected = S.is_selected state (Rows.Item.key item)
          ; cursor =
              Option.exists (S.cursor state) ~f:(fun target ->
                Gpuio.Key.equal
                  (C.Expert.item_key target)
                  (C.Expert.item_key (Rows.Item.target item)))
          }
        in
        let content =
          match render_row with
          | Some render -> render ~row ~controller ~lifetime graph
          | None ->
            B.map row ~f:(fun row ->
              Gpuio.View.text
                (item_label ~key:(Rows.Item.key row.item) (Rows.Item.data row.item)))
        in
        let%arr row = row
        and content = content in
        let style =
          match row_style with
          | None -> presentation row
          | Some f -> Gpuio.Style.merge [ presentation row; f row ]
        in
        let view = Gpuio.View.column ~style [ content ] in
        match
          Rows.Item.accessibility
            row.item
            ~label:(item_label ~key:(Rows.Item.key row.item) (Rows.Item.data row.item))
            ~selected:row.selected
          |> Or_error.ok_exn
        with
        | None -> view
        | Some accessibility ->
          Gpuio.View.with_accessibility view accessibility |> Or_error.ok_exn
      in
      let list =
        V.component_with_config
          (module Rows.Key)
          collection
          ~row_key:Rows.Key.to_view_key
          ~config
          ~accessibility
          ~input
          ~render_row:render
          graph
      in
      let%arr list = list
      and prepared = prepared
      and controller = controller in
      let projection, state, _, _ = prepared in
      Or_error.map list ~f:(fun list ->
        { Output.view = V.Output.view list; list; projection; state; controller })
  in
  let after_display =
    let%arr output = output
    and model = model
    and inject = inject
    and accepted = accepted
    and set_accepted = set_accepted
    and epoch = epoch
    and accepted_epoch = accepted_epoch
    and set_accepted_epoch = set_accepted_epoch
    and lifetime = lifetime in
    match output with
    | Error _ -> E.Ignore
    | Ok output ->
      Managed_rows.Lifetime.guard
        lifetime
        (E.Many
           [ (if Option.exists model ~f:(phys_equal output.state)
              then E.Ignore
              else inject Sync)
           ; (if Option.exists accepted ~f:(phys_equal output.projection)
              then E.Ignore
              else set_accepted (Some output.projection))
           ; (if Option.exists accepted_epoch ~f:(phys_equal epoch)
              then E.Ignore
              else set_accepted_epoch (Some epoch))
           ])
  in
  B.Edge.after_display after_display graph;
  output
;;

let component
      source
      ~layout
      ~config
      ~interaction
      ~label
      ~item_label
      ?key
      ?(style = B.return fill)
      ?(initial_selected = B.return [])
      ?query
      ?(before = B.return [])
      ?(after = B.return [])
      ?(on_action = B.return (fun _ -> E.Ignore))
      ?row_style
      ?render_row
      graph
  =
  let open B.Let_syntax in
  let query = B.transpose_opt query in
  let sources =
    B.map source ~f:(fun source ->
      Map.singleton (module C.Source_id) (C.Identity.source_id (C.identity source)) source)
  in
  let results =
    Managed_rows.assoc
      (module C.Source_id)
      sources
      ~f:(fun source_id source lifetime graph ->
        let output =
          inner
            source
            ~layout
            ~config
            ~interaction
            ~label
            ~item_label
            ~initial_selected
            ~query
            ~before
            ~after
            ~on_action
            ~row_style
            ~render_row
            ~lifetime
            graph
        in
        let%arr output = output
        and source_id = source_id in
        Or_error.map output ~f:(fun output ->
          { output with
            Output.view = Gpuio.View.with_key output.view (C.Source_id.to_key source_id)
          }))
      graph
  in
  let%arr results = results
  and style = style in
  Map.data results
  |> List.hd_exn
  |> Or_error.map ~f:(fun output ->
    { output with Output.view = Gpuio.View.column ?key ~style [ output.view ] })
;;
