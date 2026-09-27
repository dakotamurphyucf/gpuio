open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio.Virtual_list
module C = Gpuio.List_collection
module Config = V.Config
module Viewport = V.Viewport
module Key = Gpuio.Key

module Command = struct
  type t =
    | Offset of Key.t * float
    | Reveal of Key.t
    | Focus_tree_row of Key.t
    | End
  [@@deriving equal, sexp_of]

  let target = function
    | Offset (key, _) | Reveal key | Focus_tree_row key -> Some key
    | End -> None
  ;;

  let to_request t ~serial =
    match t with
    | Offset (key, offset) -> V.Scroll_request.to_row ~serial ~offset key
    | Reveal key -> V.Scroll_request.reveal ~serial key
    | Focus_tree_row key -> V.Scroll_request.focus_tree_row ~serial key
    | End -> V.Scroll_request.to_end ~serial ()
  ;;
end

module Controller = struct
  type 'key t =
    { key : 'key -> Key.t
    ; submit : Command.t -> unit E.t
    }

  let scroll_to t ?(offset = 0.) key =
    let key = t.key key in
    let open Or_error.Let_syntax in
    let%map (_ : V.Scroll_request.t) = V.Scroll_request.to_row ~serial:1L ~offset key in
    t.submit (Offset (key, offset))
  ;;

  let reveal t key = t.submit (Reveal (t.key key))
  let focus_tree_row t key = t.submit (Focus_tree_row (t.key key))
  let jump_to_latest t = t.submit End
end

module Output = struct
  type 'key t =
    { view : unit E.t Gpuio.View.t
    ; controller : 'key Controller.t
    ; viewport : Viewport.t option
    ; active_rows : int
    ; budget_exhausted : bool
    }

  let view t = t.view
  let controller t = t.controller
  let viewport t = t.viewport
  let active_rows t = t.active_rows
  let budget_exhausted t = t.budget_exhausted
end

module Metadata = struct
  type 'key t =
    { order : V.Order.t
    ; by_wire : 'key String.Map.t
    ; reversed : Key.t list
    }

  let create keys ~row_key =
    let pairs = List.map keys ~f:(fun key -> row_key key, key) in
    let open Or_error.Let_syntax in
    let%bind order = V.Order.create (List.map pairs ~f:fst) in
    let%map by_wire =
      List.map pairs ~f:(fun (wire, key) -> Key.to_string wire, key)
      |> String.Map.of_alist_or_error
    in
    { order; by_wire; reversed = List.rev (V.Order.keys order) }
  ;;
end

module Model = struct
  type t =
    { requested : Key.t list
    ; pins : Key.t list
    ; viewport : Viewport.t option
    ; viewport_revision : int64 option
    ; observed_tail : Key.t option
    ; serial : int64
    ; scroll : V.Scroll_request.t option
    }
  [@@deriving equal, sexp_of]

  let empty =
    { requested = []
    ; pins = []
    ; viewport = None
    ; viewport_revision = None
    ; observed_tail = None
    ; serial = 0L
    ; scroll = None
    }
  ;;
end

module Action = struct
  type t =
    | Observe of int64 * Key.t option * Viewport.t
    | Retain of Key.t list
    | Scroll of Command.t
  [@@deriving sexp_of]
end

module Accepted = struct
  type ('key, 'data, 'cmp) t =
    { source : ('key, 'data, 'cmp) C.t
    ; revision : int64
    }
end

let fill =
  Gpuio.Style.create_exn
    [ Width (Gpuio.Length.percent_exn 100.)
    ; Height (Gpuio.Length.percent_exn 100.)
    ; Min_height (Gpuio.Length.px_exn 0.)
    ; Min_width (Gpuio.Length.px_exn 0.)
    ]
;;

let apply_action _ input model action =
  match input with
  | Bonsai.Computation_status.Inactive | Active (Error _) -> model
  | Active (Ok metadata) ->
    (match action with
     | Action.Observe (revision, tail, viewport) ->
       { model with
         Model.requested = viewport.requested
       ; pins = viewport.pinned
       ; viewport = Some viewport
       ; viewport_revision = Some revision
       ; observed_tail = tail
       }
     | Retain pins ->
       { model with pins = List.dedup_and_sort (pins @ model.pins) ~compare:Key.compare }
     | Scroll command ->
       if
         Option.exists (Command.target command) ~f:(fun key ->
           not (V.Order.mem metadata.Metadata.order key))
       then model
       else (
         if Int64.equal model.serial Int64.max_value
         then failwith "list scroll serial exhausted";
         let serial = Int64.succ model.serial in
         let scroll = Command.to_request command ~serial |> Or_error.ok_exn in
         { model with serial; scroll = Some scroll }))
;;

module Selection = struct
  type t =
    { keys : Key.t list
    ; exhausted : bool
    }
end

let active_keys (metadata : _ Metadata.t) model extra_pins ~max_active =
  let live keys = List.filter keys ~f:(V.Order.mem metadata.order) in
  let pins =
    live (extra_pins @ model.Model.pins) |> List.dedup_and_sort ~compare:Key.compare
  in
  if List.length pins > max_active
  then Or_error.error_string "virtual list pinned rows exceed the active budget"
  else (
    (* Native scrolling still owns tail-following policy. While it reports that
       policy active, supply the bounded newest rows in the same transaction as
       an append, rather than first painting estimated-height placeholders. *)
    let tail =
      if Option.exists model.viewport ~f:(fun viewport -> viewport.following_tail)
      then
        List.take metadata.reversed (max_active - List.length pins)
        |> List.take_while ~f:(fun key ->
          not (Option.exists model.observed_tail ~f:(Key.equal key)))
      else []
    in
    let requested = tail @ live model.requested in
    let _, reversed =
      List.fold
        (pins @ requested)
        ~init:(String.Set.empty, [])
        ~f:(fun (seen, keys) key ->
          let wire = Key.to_string key in
          if Set.mem seen wire || Set.length seen >= max_active
          then seen, keys
          else Set.add seen wire, key :: keys)
    in
    let requested_count =
      List.map (pins @ requested) ~f:Key.to_string |> String.Set.of_list |> Set.length
    in
    Ok { Selection.keys = List.rev reversed; exhausted = requested_count > max_active })
;;

let inner
      (type key cmp)
      (comparator : (key, cmp) B.comparator)
      source
      ~row_key
      ~config
      ~style
      ~accessibility
      ~on_tree_input
      ~tree_moves
      ~generation
      ~pinned
      ~on_viewport
      ~render_row
      graph
  =
  let module K = (val comparator) in
  let open B.Let_syntax in
  let keys = B.map source ~f:C.keys |> B.cutoff ~equal:phys_equal in
  let metadata = B.map keys ~f:(Metadata.create ~row_key) in
  let model, inject =
    B.state_machine1
      ~default_model:Model.empty
      ~equal:Model.equal
      ~sexp_of_model:Model.sexp_of_t
      ~sexp_of_action:Action.sexp_of_t
      ~apply_action
      metadata
      graph
  in
  let accepted, set_accepted = B.state_opt ~equal:phys_equal graph in
  let checkpoint =
    let%arr source = source
    and accepted = accepted in
    match accepted with
    | Some previous when phys_equal source previous.Accepted.source -> previous
    | previous ->
      let revision =
        Option.value_map previous ~default:0L ~f:(fun previous ->
          let same_order =
            phys_equal (C.keys source) (C.keys previous.source)
            || List.equal
                 (fun a b -> K.comparator.compare a b = 0)
                 (C.keys source)
                 (C.keys previous.source)
          in
          let changed =
            (not same_order)
            || C.fold_changed_values
                 source
                 ~previous:previous.source
                 ~init:false
                 ~f:(fun _ _ -> true)
          in
          if not changed
          then previous.revision
          else (
            if Int64.equal previous.revision Int64.max_value
            then failwith "list invalidation revision exhausted";
            Int64.succ previous.revision))
      in
      { Accepted.source; revision }
  in
  let invalidated =
    let%arr checkpoint = checkpoint
    and accepted = accepted in
    Option.value_map accepted ~default:[] ~f:(fun previous ->
      C.fold_changed_values
        checkpoint.source
        ~previous:previous.source
        ~init:[]
        ~f:(fun keys key ->
          if Option.is_some (C.find checkpoint.source key)
          then row_key key :: keys
          else keys))
  in
  let active =
    let%arr metadata = metadata
    and model = model
    and pinned = pinned in
    Or_error.bind metadata ~f:(fun metadata ->
      active_keys
        metadata
        model
        (List.map pinned ~f:row_key)
        ~max_active:(Config.max_active config))
  in
  let members =
    let%arr active = active
    and metadata = metadata
    and source = source in
    match active, metadata with
    | Ok active, Ok metadata ->
      List.fold
        active.Selection.keys
        ~init:(Map.empty (module K))
        ~f:(fun rows wire ->
          let key = Map.find_exn metadata.by_wire (Key.to_string wire) in
          match C.find source key with
          | Some data -> Map.set rows ~key ~data
          | None -> rows)
    | Error _, _ | _, Error _ -> Map.empty (module K)
  in
  let rows =
    Managed_rows.assoc
      comparator
      members
      ~f:(fun key data lifetime graph -> render_row ~key ~data ~lifetime graph)
      graph
  in
  let result =
    let%arr metadata = metadata
    and active = active
    and rows = rows
    and model = model
    and inject = inject
    and invalidated = invalidated
    and checkpoint = checkpoint
    and style = style
    and accessibility = accessibility
    and generation = generation
    and observe = on_viewport
    and on_tree_input = on_tree_input
    and tree_moves = tree_moves in
    let open Or_error.Let_syntax in
    let%bind metadata = metadata in
    let%bind active = active in
    let%bind view =
      Gpuio.View.Expert.managed_virtual_list
        ~key:(Key.of_string_exn (Int64.to_string generation))
        ~style
        ~config
        ~order:metadata.order
        ?scroll:model.scroll
        ~invalidated
        ~invalidation_revision:checkpoint.revision
        ~on_viewport:(fun viewport ->
          E.Many
            [ inject (Observe (checkpoint.revision, List.hd metadata.reversed, viewport))
            ; observe viewport
            ])
        ~on_retain:(fun keys -> inject (Retain keys))
        ~tree_moves
        ?on_tree_input:
          (Option.map on_tree_input ~f:(fun callback input ->
             match
               Gpuio.Tree_input.filter_map input ~f:(fun key ->
                 Map.find metadata.by_wire (Key.to_string key))
             with
             | None -> E.Ignore
             | Some input -> callback input))
        (Map.to_alist rows |> List.map ~f:(fun (key, view) -> row_key key, view))
    in
    let%map view =
      match accessibility with
      | None -> Ok view
      | Some accessibility -> Gpuio.View.with_accessibility view accessibility
    in
    { Output.view
    ; controller =
        { Controller.key = row_key; submit = (fun command -> inject (Scroll command)) }
    ; viewport =
        (if Option.exists model.viewport_revision ~f:(Int64.equal checkpoint.revision)
         then model.viewport
         else None)
    ; active_rows = List.length active.Selection.keys
    ; budget_exhausted =
        active.exhausted
        || Option.exists model.viewport ~f:(fun viewport -> viewport.budget_exhausted)
    }
  in
  let after_display =
    let%arr result = result
    and checkpoint = checkpoint
    and accepted = accepted
    and set_accepted = set_accepted in
    match result with
    | Error _ -> E.Ignore
    | Ok _ ->
      if Option.exists accepted ~f:(phys_equal checkpoint)
      then E.Ignore
      else set_accepted (Some checkpoint)
  in
  B.Edge.after_display after_display graph;
  result
;;

let component
      comparator
      source
      ~row_key
      ~config
      ?key
      ?(style = B.return fill)
      ?accessibility
      ?on_tree_input
      ?(tree_moves = B.return false)
      ?(generation = B.return 0L)
      ?(pinned = B.return [])
      ?(on_viewport = B.return (fun _ -> E.Ignore))
      ~render_row
      graph
  =
  let open B.Let_syntax in
  let accessibility = B.transpose_opt accessibility in
  let on_tree_input = B.transpose_opt on_tree_input in
  let generations =
    let%arr generation = generation
    and source = source in
    Int64.Map.singleton generation source
  in
  let results =
    Managed_rows.assoc
      (module Int64)
      generations
      ~f:(fun generation source _ graph ->
        inner
          comparator
          source
          ~row_key
          ~config
          ~style:(B.return fill)
          ~accessibility
          ~on_tree_input
          ~tree_moves
          ~generation
          ~pinned
          ~on_viewport
          ~render_row
          graph)
      graph
  in
  let%arr results = results
  and style = style in
  let result = Map.data results |> List.hd_exn in
  Or_error.map result ~f:(fun output ->
    { output with Output.view = Gpuio.View.column ?key ~style [ output.view ] })
;;

module Paging = struct
  module Direction = Gpuio.List_paging.Direction

  type t =
    { request : generation:int64 -> Direction.t -> unit E.t
    ; retry : generation:int64 -> Direction.t -> unit E.t
    ; cancel : generation:int64 -> Direction.t -> unit E.t
    }

  let create ~request ~retry ~cancel = { request; retry; cancel }
  let request t = t.request
  let retry t = t.retry
  let cancel t = t.cancel
end

module Demand = struct
  type ('key, 'data, 'cmp) t =
    { source : ('key, 'data, 'cmp) C.t
    ; generation : int64
    ; before : bool
    ; after : bool
    }

  let equal a b =
    phys_equal a.source b.source
    && Int64.equal a.generation b.generation
    && Bool.equal a.before b.before
    && Bool.equal a.after b.after
  ;;
end

let paged
      comparator
      snapshot
      ~paging
      ~row_key
      ~config
      ?key
      ?style
      ?accessibility
      ?on_tree_input
      ?(tree_moves = B.return false)
      ?pinned
      ?(auto_load = B.return true)
      ?on_viewport
      ~render_row
      graph
  =
  let open B.Let_syntax in
  let source =
    B.map snapshot ~f:(fun snapshot -> snapshot.Gpuio.List_paging.Snapshot.items)
  in
  let generation =
    B.map snapshot ~f:(fun snapshot -> snapshot.Gpuio.List_paging.Snapshot.generation)
  in
  let output =
    component
      comparator
      source
      ~row_key
      ~config
      ?key
      ?style
      ?accessibility
      ?on_tree_input
      ~tree_moves
      ~generation
      ?pinned
      ?on_viewport
      ~render_row
      graph
  in
  let demand =
    let%arr snapshot = snapshot
    and output = output
    and auto_load = auto_load in
    let ready : Gpuio.List_paging.Status.t -> bool = function
      | Ready -> true
      | Loading | End | Failed _ -> false
    in
    let viewport = Or_error.ok output |> Option.bind ~f:Output.viewport in
    { Demand.source = snapshot.items
    ; generation = snapshot.generation
    ; before =
        auto_load
        && ready snapshot.before
        && Option.exists viewport ~f:(fun v -> v.at_start)
    ; after =
        auto_load && ready snapshot.after && Option.exists viewport ~f:(fun v -> v.at_end)
    }
  in
  let callback =
    let%arr paging = paging in
    fun demand ->
      E.Many
        (List.filter_map
           [ demand.Demand.before, Gpuio.List_paging.Direction.Before
           ; demand.after, After
           ]
           ~f:(fun (load, direction) ->
             if load
             then Some (Paging.request paging ~generation:demand.generation direction)
             else None))
  in
  B.Edge.on_change ~equal:Demand.equal demand ~callback graph;
  output
;;
