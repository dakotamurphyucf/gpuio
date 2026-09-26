open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module Rows = Gpuio.Tree_rows
module Loading_model = Gpuio.Tree_loading
module Snapshot = Loading_model.Snapshot
module Lease = Loading_model.Lease
module Target = Loading_model.Target
module V = Virtual_list

let fill =
  Gpuio.Style.create_exn
    [ Width (Gpuio.Length.percent_exn 100.)
    ; Height (Gpuio.Length.percent_exn 100.)
    ; Min_height (Gpuio.Length.px_exn 0.)
    ; Min_width (Gpuio.Length.px_exn 0.)
    ]
;;

module Loading = struct
  type t =
    { request : Target.t -> unit E.t
    ; retry : Target.t -> unit E.t
    ; cancel : Target.t -> unit E.t
    ; cancel_hidden : Lease.t -> Gpuio.Tree_state.t -> unit E.t
    }

  let create ~request ~retry ~cancel ~cancel_hidden =
    { request; retry; cancel; cancel_hidden }
  ;;

  let request t = t.request
  let retry t = t.retry
  let cancel t = t.cancel
  let cancel_hidden t = t.cancel_hidden
end

module Controller = struct
  type t =
    { reveal : focus:bool -> Rows.Key.t -> unit E.t
    ; request : Target.t -> unit E.t
    ; retry : Target.t -> unit E.t
    ; cancel : Target.t -> unit E.t
    }

  let reveal t ?(focus = false) key = t.reveal ~focus key
  let request t = t.request
  let retry t = t.retry
  let cancel t = t.cancel
end

module Output = struct
  type 'data t =
    { view : unit E.t Gpuio.View.t
    ; list : Rows.Key.t V.Output.t
    ; projection : 'data Rows.t
    ; controller : Controller.t
    }

  let view t = t.view
  let projection t = t.projection
  let controller t = t.controller
  let viewport t = V.Output.viewport t.list
  let active_rows t = V.Output.active_rows t.list
  let budget_exhausted t = V.Output.budget_exhausted t.list
end

let with_current lifetime peek ~f =
  Managed_rows.Lifetime.guard
    lifetime
    (E.bind peek ~f:(function
       | B.Computation_status.Inactive | Active (Error _) -> E.Ignore
       | Active (Ok rows) -> f rows))
;;

module Demand = struct
  type t =
    { lease : Lease.t
    ; state : Gpuio.Tree_state.t
    ; targets : Target.t list
    ; cancel_hidden : bool
    }

  let equal a b =
    Lease.equal a.lease b.lease
    && phys_equal a.state b.state
    && List.equal Target.equal a.targets b.targets
    && Bool.equal a.cancel_hidden b.cancel_hidden
  ;;
end

let targets rows viewport ~config ~auto_load =
  match viewport with
  | None -> []
  | Some _ when not auto_load -> []
  | Some viewport ->
    let source = Rows.source rows in
    let available = Loading_model.max_queued - Snapshot.queued_count source in
    let collection = Rows.collection rows in
    let count = Gpuio.List_collection.length collection in
    let first =
      Int.min count (Int.max 0 viewport.Gpuio.Virtual_list.Viewport.visible_first)
    in
    let last =
      Int.min count (Int.min viewport.visible_last (first + V.Config.max_active config))
      |> Int.max first
    in
    let visible =
      Gpuio.List_collection.range collection ~first ~last |> Or_error.ok_exn
    in
    List.fold visible ~init:(available, []) ~f:(fun (available, reversed) (_, row) ->
      if available <= 0
      then available, reversed
      else (
        match row with
        | Rows.Row.Item _ -> available, reversed
        | Boundary { parent; status = Ready; _ } ->
          let target = Snapshot.target source parent |> Or_error.ok_exn in
          available - 1, target :: reversed
        | Boundary { status = Queued | Loading | End | Failed _; _ } ->
          available, reversed))
    |> snd
    |> List.rev
;;

let controller ~lifetime ~peek ~list ~loading =
  let list_controller = V.Output.controller list in
  let run target ~visible operation =
    with_current lifetime peek ~f:(fun rows ->
      if
        Snapshot.is_current (Rows.source rows) target
        && ((not visible)
            || Option.is_some (Rows.boundary_key rows (Target.parent target)))
      then
        Option.value_map loading ~default:E.Ignore ~f:(fun loading ->
          operation loading target)
      else E.Ignore)
  in
  { Controller.reveal =
      (fun ~focus key ->
        with_current lifetime peek ~f:(fun rows ->
          match Rows.find rows key with
          | None | Some (Boundary _) -> E.Ignore
          | Some (Item item) ->
            if Gpuio.Tree.Node.is_disabled item.node
            then E.Ignore
            else if focus
            then V.Controller.focus_tree_row list_controller key
            else V.Controller.reveal list_controller key))
  ; request = (fun target -> run target ~visible:true Loading.request)
  ; retry = (fun target -> run target ~visible:true Loading.retry)
  ; cancel = (fun target -> run target ~visible:false Loading.cancel)
  }
;;

let inner
      source
      ~state
      ~config
      ?accessibility
      ?on_request
      ~pinned
      ~loading
      ~auto_load
      ~cancel_hidden
      ~lifetime
      ~render_row
      graph
  =
  let open B.Let_syntax in
  let accepted, set_accepted = B.state_opt ~equal:phys_equal graph in
  let projection =
    let%arr source = source
    and state = state
    and accepted = accepted in
    match accepted with
    | None -> Rows.create source ~state
    | Some previous -> Rows.update previous source ~state
  in
  let collection =
    B.map projection ~f:(function
      | Ok rows -> Rows.collection rows
      | Error _ -> Gpuio.List_collection.empty (module Rows.Key))
  in
  let pins =
    let%arr projection = projection
    and pinned = pinned in
    match projection with
    | Error _ -> []
    | Ok rows -> List.filter_map pinned ~f:(Rows.item_key rows)
  in
  let peek = B.peek projection graph in
  let on_tree_input =
    Option.map on_request ~f:(fun callback ->
      let%arr callback = callback
      and peek = peek
      and lifetime = lifetime in
      fun input ->
        with_current lifetime peek ~f:(fun rows ->
          let source = Rows.source rows in
          let target key =
            match Rows.find rows key with
            | None | Some (Boundary _) -> None
            | Some (Item item) ->
              Gpuio.Tree_interaction.Target.capture source item.id |> Result.ok
          in
          match Gpuio.Tree_input.filter_map input ~f:target with
          | None -> E.Ignore
          | Some input ->
            let module R = Gpuio.Tree_interaction.Request in
            let request =
              match input with
              | Navigate (direction, selection) -> R.navigate source ~selection direction
              | Select (target, selection) -> R.select target selection
              | Focus target -> R.focus target
              | Set_expanded (target, expanded) -> R.set_expanded target expanded
              | Activate target -> R.activate target
              | Select_active selection -> R.select_active source selection
              | Activate_active -> R.activate_active source
              | Typeahead input -> R.typeahead source input
              | Set_selected (target, selected) -> R.set_selected target selected
            in
            callback request))
  in
  let list =
    V.component
      (module Rows.Key)
      collection
      ~row_key:Rows.Key.to_view_key
      ~config
      ?accessibility
      ?on_tree_input
      ~pinned:pins
      ~render_row
      graph
  in
  let output =
    let%arr projection = projection
    and list = list
    and peek = peek
    and loading = loading
    and lifetime = lifetime in
    let%bind.Or_error projection = projection in
    let%map.Or_error list = list in
    { Output.view = V.Output.view list
    ; list
    ; projection
    ; controller = controller ~lifetime ~peek ~list ~loading
    }
  in
  let accept =
    let%arr output = output
    and accepted = accepted
    and set_accepted = set_accepted in
    match output with
    | Error _ -> E.Ignore
    | Ok output ->
      if Option.exists accepted ~f:(phys_equal output.projection)
      then E.Ignore
      else set_accepted (Some output.projection)
  in
  B.Edge.after_display accept graph;
  let demand =
    let%arr output = output
    and auto_load = auto_load
    and cancel_hidden = cancel_hidden in
    Or_error.ok output
    |> Option.map ~f:(fun output ->
      let rows = output.Output.projection in
      { Demand.lease = Snapshot.lease (Rows.source rows)
      ; state = Rows.state rows
      ; targets = targets rows (V.Output.viewport output.list) ~config ~auto_load
      ; cancel_hidden
      })
  in
  let observed = B.both demand loading in
  let peek_demand = B.peek observed graph in
  let callback =
    let%arr peek_demand = peek_demand
    and peek = peek
    and lifetime = lifetime in
    fun _ ->
      E.bind peek_demand ~f:(function
        | B.Computation_status.Inactive | Active (_, None) | Active (None, Some _) ->
          E.Ignore
        | Active (Some demand, Some loading) ->
          with_current lifetime peek ~f:(fun rows ->
            if not (Lease.equal demand.Demand.lease (Snapshot.lease (Rows.source rows)))
            then E.Ignore
            else
              E.Many
                ((if demand.cancel_hidden
                  then [ Loading.cancel_hidden loading demand.lease (Rows.state rows) ]
                  else [])
                 @ List.filter_map demand.targets ~f:(fun target ->
                   if
                     Snapshot.is_current (Rows.source rows) target
                     && Option.is_some (Rows.boundary_key rows (Target.parent target))
                   then Some (Loading.request loading target)
                   else None))))
  in
  B.Edge.on_change
    ~equal:(fun (a, al) (b, bl) ->
      Option.equal Demand.equal a b && Option.equal phys_equal al bl)
    observed
    ~callback
    graph;
  output
;;

let component
      source
      ~state
      ~config
      ?key
      ?(style = B.return fill)
      ?accessibility
      ?on_request
      ?(pinned = B.return [])
      ?loading
      ?(auto_load = B.return true)
      ?(cancel_hidden = B.return true)
      ~render_row
      graph
  =
  let open B.Let_syntax in
  let loading =
    match loading with
    | None -> B.return None
    | Some loading -> B.map loading ~f:Option.some
  in
  let generations =
    B.map source ~f:(fun source ->
      Map.singleton (module Lease) (Snapshot.lease source) source)
  in
  let results =
    Managed_rows.assoc
      (module Lease)
      generations
      ~f:(fun lease source lifetime graph ->
        let output =
          inner
            source
            ~state
            ~config
            ?accessibility
            ?on_request
            ~pinned
            ~loading
            ~auto_load
            ~cancel_hidden
            ~lifetime
            ~render_row
            graph
        in
        let%arr output = output
        and lease = lease in
        Or_error.map output ~f:(fun output ->
          let key = Gpuio.Key.of_string_exn (Sexp.to_string (Lease.sexp_of_t lease)) in
          output, Gpuio.View.column ~key ~style:fill [ Output.view output ]))
      graph
  in
  let%arr results = results
  and style = style in
  Map.data results
  |> List.hd_exn
  |> Or_error.map ~f:(fun (output, view) ->
    (* Keep the public wrapper stable, but replace the source-keyed child on reset. *)
    { output with Output.view = Gpuio.View.column ?key ~style [ view ] })
;;
