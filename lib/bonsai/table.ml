open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module D = Gpuio.Table_data
module Row = D.Row_ref
module T = Gpuio.Table
module Config = T.Config
module Selection = T.Selection
module Request = T.Request
module Target = T.Target
module V = Gpuio.Virtual_list
module C = Gpuio.Table_column
module Key = Gpuio.Key

module Cell = struct
  type t =
    { metadata : T.Cell.t
    ; view : unit E.t Gpuio.View.t
    }

  let create ~column ~copy_text view =
    let%map.Or_error metadata = T.Cell.create ~column ~copy_text in
    { metadata; view }
  ;;

  let text ~column text =
    let%map.Or_error metadata = T.Cell.create ~column ~copy_text:text in
    { metadata; view = Gpuio.View.Expert.table_text metadata }
  ;;
end

module Controller = struct
  type t = { submit : Row.t Target.t list -> unit E.t }

  let batch t targets =
    if List.length targets > 64
    then Or_error.error_string "table batch exceeds 64 commands"
    else (
      let%map.Or_error (_ : Row.t T.Command.t list) =
        List.map targets ~f:(T.Command.create ~serial:1L ~query_generation:0L)
        |> Or_error.all
      in
      t.submit targets)
  ;;

  let select t selection = t.submit [ Target.Set_selection selection ]
  let reveal t ?column row = t.submit [ Target.Reveal (row, column) ]
  let scroll_to t ?(offset = 0.) row = batch t [ Target.Scroll_to (row, offset) ]
  let scroll_to_column t column = t.submit [ Target.Scroll_to_column column ]
  let scroll_to_end t = t.submit [ Target.Scroll_to_end ]
  let reset_columns t = t.submit [ Target.Reset_columns ]
end

module Output = struct
  type 'data t =
    { view : unit E.t Gpuio.View.t
    ; controller : Controller.t
    ; source : 'data D.t
    ; selection : Row.t Selection.t
    ; viewport : V.Viewport.t option
    ; column_viewport : T.Column_viewport.t option
    ; active_rows : int
    ; active_cells : int
    ; budget_exhausted : bool
    }

  let view t = t.view
  let controller t = t.controller
  let selection t = t.selection

  let target t id =
    Option.value_map
      (D.row_ref t.source id)
      ~default:(Or_error.error_string "absent table row")
      ~f:Or_error.return
  ;;

  let viewport t = t.viewport
  let column_viewport t = t.column_viewport
  let active_rows t = t.active_rows
  let active_cells t = t.active_cells
  let budget_exhausted t = t.budget_exhausted
end

module Metadata = struct
  type t =
    { order : V.Order.t
    ; rows : Row.t String.Map.t
    }

  let create identity =
    let refs = D.Expert.Identity.rows identity in
    let pairs = List.map refs ~f:(fun row -> D.Expert.row_key row, row) in
    { order = V.Order.create (List.map pairs ~f:fst) |> Or_error.ok_exn
    ; rows =
        List.map pairs ~f:(fun (key, row) -> Key.to_string key, row)
        |> String.Map.of_alist_exn
    }
  ;;

  let find t key = Map.find t.rows (Key.to_string key)
end

let same_order a b =
  let a = V.Order.keys a
  and b = V.Order.keys b in
  phys_equal a b || List.equal Key.equal a b
;;

module Observation = struct
  type t =
    { query : int64
    ; order : (V.Order.t[@sexp.opaque])
    ; config : Config.t
    ; viewport : V.Viewport.t
    }
  [@@deriving sexp_of]
end

module Pending = struct
  type t =
    { query : int64
    ; commands : Row.t T.Command.t list
    }
  [@@deriving sexp_of]
end

module Column_observation = struct
  type t =
    { query : int64
    ; config : Config.t
    ; viewport : T.Column_viewport.t
    }
  [@@deriving sexp_of]
end

module Preparation = struct
  type t =
    { query : int64
    ; order : (V.Order.t[@sexp.opaque])
    ; config : Config.t
    ; rows : Row.t list
    }
  [@@deriving sexp_of]
end

module Model = struct
  type t =
    { query : int64 option
    ; requested : Row.t list
    ; preparation : Preparation.t option
    ; pins : Row.t list
    ; observation : Observation.t option
    ; column_observation : Column_observation.t option
    ; selection : Row.t Selection.t
    ; serial : int64
    ; displayed_serial : int64
    ; selection_tick : int64
    ; pending : Pending.t option
    }
  [@@deriving sexp_of]

  let empty =
    { query = None
    ; requested = []
    ; preparation = None
    ; pins = []
    ; observation = None
    ; column_observation = None
    ; selection = Empty
    ; serial = 0L
    ; displayed_serial = 0L
    ; selection_tick = 0L
    ; pending = None
    }
  ;;
end

module Action = struct
  type t =
    | Observe of int64 * Observation.t * Row.t list * Row.t list
    | Observe_columns of Column_observation.t
    | Retain of int64 * Row.t list
    | Input of int64 * Config.t * Row.t Request.t
    | Commands of int64 * Row.t Target.t list
    | Displayed of int64 * int64 * Row.t Selection.t
  [@@deriving sexp_of]
end

type 'data input =
  { source : 'data D.t
  ; metadata : Metadata.t
  ; config : Config.t
  ; query : int64
  ; on_request : Row.t Request.t -> unit E.t
  ; lifetime : Managed_rows.Lifetime.t
  }

let current_row source row = if D.contains_ref source row then Some row else None

let command_wire input command =
  T.Expert.command_to_wire input.config command ~find_id:(fun row ->
    if D.contains_ref input.source row then Some 1L else None)
;;

let repaired_selection input selection =
  let cmd =
    T.Command.create ~serial:1L ~query_generation:input.query (Set_selection selection)
  in
  match Or_error.bind cmd ~f:(command_wire input) with
  | Ok _ -> selection
  | Error _ -> Selection.Empty
;;

(* Anticipation changes only bounded materialization. The native viewport and
   its eventual observation still own actual geometry and scroll clamping. *)
let prepare_scroll input (model : Model.t) targets =
  let pins =
    List.filter model.pins ~f:(D.contains_ref input.source) |> Set.of_list (module Row)
  in
  let capacity = Int.max 0 (Config.max_active_rows input.config - Set.length pins) in
  let count = D.length input.source in
  let observation =
    Option.filter model.observation ~f:(fun observed ->
      Int64.equal observed.query input.query
      && same_order observed.order input.metadata.order
      && Config.equal observed.config input.config
      && observed.viewport.visible_last > observed.viewport.visible_first)
  in
  let span =
    Option.value_map observation ~default:capacity ~f:(fun observed ->
      observed.viewport.visible_last - observed.viewport.visible_first + 1)
    |> Int.max 1
    |> Int.min capacity
  in
  let initial =
    Option.value_map observation ~default:0 ~f:(fun observed ->
      observed.viewport.visible_first)
  in
  let clamp first = Int.clamp_exn first ~min:0 ~max:(Int.max 0 (count - span)) in
  let position row = D.index input.source (Row.id row) |> Option.value_exn in
  let destination =
    List.fold targets ~init:None ~f:(fun destination target ->
      match target with
      | Target.Scroll_to (row, _) -> Some (clamp (position row))
      | Scroll_to_end -> Some (clamp count)
      | Reveal (row, _) ->
        let row = position row in
        let first = Option.value destination ~default:initial in
        (* Keep the extra safety row out of the nearest-edge decision. *)
        let visible = Int.max 1 (span - 1) in
        if row < first
        then Some (clamp row)
        else if row >= first + visible
        then Some (clamp (row - visible + 1))
        else destination
      | Set_selection _ | Scroll_to_column _ | Reset_columns -> destination)
  in
  Option.map destination ~f:(fun first ->
    let overscan =
      Float.iround_up_exn (Config.overscan input.config /. Config.row_height input.config)
      |> Int.min capacity
    in
    let last = Int.min count (first + span) in
    let positions =
      List.range first last
      @ List.range (Int.max 0 (first - overscan)) first
      @ List.range last (Int.min count (last + overscan))
    in
    let rows =
      List.filter_map positions ~f:(fun index ->
        D.nth input.source index
        |> Option.bind ~f:(fun (id, _) -> D.row_ref input.source id))
      |> List.filter ~f:(fun row -> not (Set.mem pins row))
      |> fun rows -> List.take rows capacity
    in
    { Preparation.query = input.query
    ; order = input.metadata.order
    ; config = input.config
    ; rows
    })
;;

let apply_action context input (model : Model.t) action =
  match input with
  | B.Computation_status.Inactive -> model
  | Active input ->
    let current query = Int64.equal query input.query in
    (match action with
     | Action.Displayed (serial, tick, selection) ->
       if Int64.(serial < model.displayed_serial)
       then model
       else
         { model with
           pending = (if Int64.equal serial model.serial then None else model.pending)
         ; displayed_serial = serial
         ; selection =
             repaired_selection
               input
               (if Int64.equal tick model.selection_tick
                then selection
                else model.selection)
         }
     | Observe_columns observation
       when current observation.query && Config.equal observation.config input.config ->
       { model with column_observation = Some observation }
     | Observe (serial, observation, requested, pins)
       when Int64.equal serial model.serial
            && current observation.query
            && same_order observation.order input.metadata.order
            && Config.equal observation.config input.config ->
       { model with
         query = Some input.query
       ; requested
       ; preparation = None
       ; pins
       ; observation = Some observation
       }
     | Retain (query, pins) when current query ->
       { model with pins = List.dedup_and_sort (pins @ model.pins) ~compare:Row.compare }
     | Input (query, config, request)
       when current query && Config.equal config input.config ->
       (match Request.filter_map request ~f:(current_row input.source) with
        | None -> model
        | Some request ->
          B.Apply_action_context.schedule_event
            context
            (Managed_rows.Lifetime.guard input.lifetime (input.on_request request));
          (match request with
           | Request.Select selection ->
             if Int64.equal model.selection_tick Int64.max_value
             then failwith "table selection observation sequence exhausted";
             { model with
               selection = repaired_selection input selection
             ; selection_tick = Int64.succ model.selection_tick
             ; pending = None
             ; preparation =
                 (if Option.is_some model.pending then None else model.preparation)
             }
           | Activate _ | Context _ | Resize _ | Move _ | Sort _ | Copy _ -> model))
     | Commands (query, targets) when current query ->
       let commands =
         List.fold_result
           targets
           ~init:(model.serial, [])
           ~f:(fun (serial, commands) target ->
             if Int64.equal serial Int64.max_value
             then Or_error.error_string "table command serial exhausted"
             else (
               let serial = Int64.succ serial in
               let%bind.Or_error command =
                 T.Command.create ~serial ~query_generation:query target
               in
               let%map.Or_error _ = command_wire input command in
               serial, command :: commands))
       in
       (match commands with
        | Error _ -> model
        | Ok (serial, reversed) ->
          let commands = List.rev reversed in
          let preparation =
            match prepare_scroll input model targets with
            | Some _ as prepared -> prepared
            | None -> if Option.is_some model.pending then None else model.preparation
          in
          { model with serial; pending = Some { query; commands }; preparation })
     | Observe _ | Observe_columns _ | Retain _ | Input _ | Commands _ -> model)
;;

let fill =
  Gpuio.Style.create_exn
    [ Width (Gpuio.Length.percent_exn 100.)
    ; Height (Gpuio.Length.percent_exn 100.)
    ; Min_width (Gpuio.Length.px_exn 0.)
    ; Min_height (Gpuio.Length.px_exn 0.)
    ]
;;

let active_rows input model =
  let pins =
    List.filter model.Model.pins ~f:(D.contains_ref input.source)
    |> List.dedup_and_sort ~compare:Row.compare
  in
  let requested =
    match model.preparation with
    | Some prepared
      when Int64.equal prepared.query input.query
           && same_order prepared.order input.metadata.order
           && Config.equal prepared.config input.config -> prepared.rows
    | None | Some _ ->
      if Option.exists model.query ~f:(Int64.equal input.query)
      then List.filter model.requested ~f:(D.contains_ref input.source)
      else []
  in
  let max_active = Config.max_active_rows input.config in
  if List.length pins > max_active
  then Or_error.error_string "table pins exceed active-row budget"
  else (
    let _, reversed =
      List.fold
        (pins @ requested)
        ~init:(Set.empty (module Row), [])
        ~f:(fun (seen, rows) row ->
          if Set.mem seen row || Set.length seen >= max_active
          then seen, rows
          else Set.add seen row, row :: rows)
    in
    let exhausted =
      Set.length (Set.of_list (module Row) (pins @ requested)) > max_active
    in
    Ok (List.rev reversed, exhausted))
;;

let target_map target ~f =
  let open Option.Let_syntax in
  match target with
  | Target.Set_selection selection ->
    let%map selection = Selection.filter_map selection ~f in
    Target.Set_selection selection
  | Reveal (row, col) ->
    let%map row = f row in
    Target.Reveal (row, col)
  | Scroll_to (row, offset) ->
    let%map row = f row in
    Target.Scroll_to (row, offset)
  | Scroll_to_column col -> Some (Target.Scroll_to_column col)
  | Scroll_to_end -> Some Target.Scroll_to_end
  | Reset_columns -> Some Target.Reset_columns
;;

let pending_commands input model =
  match model.Model.pending with
  | None -> []
  | Some pending when not (Int64.equal pending.query input.query) -> []
  | Some pending ->
    if
      not
        (List.for_all pending.commands ~f:(fun command ->
           Or_error.is_ok (command_wire input command)))
    then []
    else
      List.map pending.commands ~f:(fun command ->
        let target =
          target_map (T.Command.target command) ~f:(fun row ->
            Some (D.Expert.row_key row))
          |> Option.value_exn
        in
        T.Command.create
          ~serial:(T.Command.serial command)
          ~query_generation:input.query
          target
        |> Or_error.ok_exn)
;;

let pending_selection input model =
  let selection =
    match model.Model.pending with
    | Some pending
      when Int64.equal pending.query input.query
           && List.for_all pending.commands ~f:(fun command ->
             Or_error.is_ok (command_wire input command)) ->
      List.fold pending.commands ~init:model.selection ~f:(fun selection command ->
        match T.Command.target command with
        | Target.Set_selection selection -> selection
        | Reveal _ | Scroll_to _ | Scroll_to_column _ | Scroll_to_end | Reset_columns ->
          selection)
    | None | Some _ -> model.selection
  in
  repaired_selection input selection
;;

let inner
      source
      ~config
      ~key
      ~style
      ~headers
      ~header_presentation
      ~render_row_presentation
      ~query_generation
      ~on_request
      ~lifetime
      ~render_cell
      graph
  =
  let open B.Let_syntax in
  let identity = B.map source ~f:D.Expert.identity |> B.cutoff ~equal:phys_equal in
  let metadata =
    B.map identity ~f:Metadata.create
    |> B.cutoff ~equal:(fun a b -> same_order a.Metadata.order b.Metadata.order)
  in
  let input =
    let%arr source = source
    and config = config
    and query = query_generation
    and metadata = metadata
    and on_request = on_request
    and lifetime = lifetime in
    { source; metadata; config; query; on_request; lifetime }
  in
  let model, inject =
    B.state_machine1
      ~default_model:Model.empty
      ~equal:phys_equal
      ~sexp_of_model:Model.sexp_of_t
      ~sexp_of_action:Action.sexp_of_t
      ~apply_action
      input
      graph
  in
  let active =
    let%arr input = input
    and model = model in
    active_rows input model
  in
  let members =
    let%arr input = input
    and active = active in
    let rows =
      match active with
      | Ok (rows, _) -> rows
      | Error _ -> []
    in
    List.filter_map rows ~f:(fun row ->
      Option.map (D.find input.source (Row.id row)) ~f:(fun data -> row, data))
    |> Map.of_alist_exn (module Row)
  in
  let columns =
    B.map config ~f:(fun config ->
      C.Collection.to_list (Config.columns config)
      |> List.map ~f:(fun column -> C.id column, column)
      |> Map.of_alist_exn (module C.Id))
  in
  let query_rows =
    let%arr query = query_generation
    and members = members in
    Int64.Map.singleton query members
  in
  let rendered =
    Managed_rows.assoc
      (module Int64)
      query_rows
      ~f:(fun _ members _ graph ->
        Managed_rows.assoc
          (module Row)
          members
          ~f:(fun row data lifetime graph ->
            let presentation = render_row_presentation ~row ~data ~lifetime graph in
            let cells =
              Managed_rows.assoc
                (module C.Id)
                columns
                ~f:(fun _ column lifetime graph ->
                  render_cell ~row ~data ~column ~lifetime graph)
                graph
            in
            B.map2 presentation cells ~f:(fun presentation cells -> presentation, cells))
          graph)
      graph
    |> B.map ~f:(fun queries -> Map.data queries |> List.hd_exn)
  in
  let result =
    let%arr input = input
    and metadata = metadata
    and model = model
    and inject = inject
    and active = active
    and rendered = rendered
    and headers = headers
    and header_presentation = header_presentation
    and style = style in
    let open Or_error.Let_syntax in
    let%bind rows, exhausted = active in
    let schema = C.Collection.to_list (Config.columns input.config) in
    let%bind cells =
      List.map rows ~f:(fun row ->
        let _, cells = Map.find_exn rendered row in
        let%map cells =
          List.map schema ~f:(fun column ->
            let%bind cell = Map.find_exn cells (C.id column) in
            if C.Id.equal (T.Cell.column cell.Cell.metadata) (C.id column)
            then Ok (cell.metadata, cell.view)
            else Or_error.error_string "cell renderer returned a different column")
          |> Or_error.all
        in
        D.Expert.row_key row, cells)
      |> Or_error.all
    in
    let%bind row_presentations =
      List.map rows ~f:(fun row ->
        let presentation, _ = Map.find_exn rendered row in
        Or_error.map presentation ~f:(fun presentation ->
          D.Expert.row_key row, presentation))
      |> Or_error.all
    in
    let query = input.query in
    let lifetime = input.lifetime in
    let config = input.config in
    let order = metadata.order in
    let guarded action = Managed_rows.Lifetime.guard lifetime (inject action) in
    let refs keys = List.filter_map keys ~f:(Metadata.find metadata) in
    let%map view =
      Gpuio.View.Expert.managed_table
        ?key
        ~source_key:
          (D.Expert.Identity.source_id (D.Expert.identity input.source)
           |> D.Source_id.sexp_of_t
           |> Sexp.to_string
           |> Key.of_string_exn)
        ~style
        ~headers
        ~header_presentation
        ~row_presentations
        ~config:input.config
        ~query_generation:query
        ~order:metadata.order
        ~commands:(pending_commands input model)
        ~on_column_viewport:(fun viewport ->
          guarded (Observe_columns { query; config; viewport }))
        ~on_viewport:(fun viewport ->
          guarded
            (Observe
               ( model.serial
               , { query; order; config; viewport }
               , refs viewport.requested
               , refs viewport.pinned )))
        ~on_retain:(fun keys -> guarded (Retain (query, refs keys)))
        ~on_input:(fun request ->
          match Request.filter_map request ~f:(Metadata.find metadata) with
          | None -> E.Ignore
          | Some request -> guarded (Input (query, config, request)))
        cells
    in
    let viewport =
      Option.bind model.observation ~f:(fun observation ->
        if
          Int64.equal observation.query query
          && same_order observation.order input.metadata.order
          && Config.equal observation.config input.config
        then Some observation.viewport
        else None)
    in
    { Output.view
    ; source = input.source
    ; controller =
        { Controller.submit = (fun targets -> guarded (Commands (query, targets))) }
    ; selection = pending_selection input model
    ; viewport
    ; column_viewport =
        Option.bind model.column_observation ~f:(fun observation ->
          if
            Int64.equal observation.query query
            && Config.equal observation.config input.config
          then Some observation.viewport
          else None)
    ; active_rows = List.length rows
    ; active_cells = List.length rows * List.length schema
    ; budget_exhausted =
        exhausted || Option.exists viewport ~f:(fun v -> v.budget_exhausted)
    }
  in
  let displayed =
    let%arr result = result
    and model = model
    and inject = inject in
    match result with
    | Ok output
      when Option.is_some model.pending
           || not (Selection.equal Row.equal output.Output.selection model.selection) ->
      inject (Displayed (model.serial, model.selection_tick, output.Output.selection))
    | Ok _ | Error _ -> E.Ignore
  in
  B.Edge.after_display displayed graph;
  result
;;

let component
      source
      ~config
      ?key
      ?(style = B.return fill)
      ?(scrollbar = B.return None)
      ?(headers = B.return [])
      ?(header_presentation = B.return Gpuio.Table_presentation.Header.empty)
      ?(render_row_presentation =
        fun ~row:_ ~data:_ ~lifetime:_ _ ->
          B.return (Ok Gpuio.Table_presentation.Row.empty))
      ?(query_generation = B.return 0L)
      ?(on_request = B.return (fun _ -> E.Ignore))
      ~render_cell
      graph
  =
  let open B.Let_syntax in
  let sources =
    B.map source ~f:(fun source ->
      Map.singleton
        (module D.Source_id)
        (D.Expert.Identity.source_id (D.Expert.identity source))
        source)
  in
  let outputs =
    Managed_rows.assoc
      (module D.Source_id)
      sources
      ~f:(fun _ source lifetime graph ->
        inner
          source
          ~config
          ~key
          ~style
          ~headers
          ~header_presentation
          ~render_row_presentation
          ~query_generation
          ~on_request
          ~lifetime
          ~render_cell
          graph)
      graph
  in
  let%arr outputs = outputs
  and scrollbar = scrollbar in
  let%bind.Or_error output = Map.data outputs |> List.hd_exn in
  let%map.Or_error view = Gpuio.View.with_scrollbar output.Output.view scrollbar in
  { output with Output.view }
;;

module Paging = Virtual_list.Paging

module Demand = struct
  type t =
    { identity : D.Expert.Identity.t
    ; query : int64
    ; before : bool
    ; after : bool
    }

  let equal a b =
    phys_equal a.identity b.identity
    && Int64.equal a.query b.query
    && Bool.equal a.before b.before
    && Bool.equal a.after b.after
  ;;
end

let paged
      snapshot
      ~paging
      ~config
      ?key
      ?style
      ?scrollbar
      ?headers
      ?header_presentation
      ?render_row_presentation
      ?(auto_load = B.return true)
      ?on_request
      ~render_cell
      graph
  =
  let open B.Let_syntax in
  let source = B.map snapshot ~f:(fun s -> s.Gpuio.Table_paging.Snapshot.data) in
  let query_generation =
    B.map snapshot ~f:(fun s -> s.Gpuio.Table_paging.Snapshot.generation)
  in
  let output =
    component
      source
      ~config
      ?key
      ?style
      ?scrollbar
      ?headers
      ?header_presentation
      ?render_row_presentation
      ~query_generation
      ?on_request
      ~render_cell
      graph
  in
  let demand =
    let%arr snapshot = snapshot
    and output = output
    and auto_load = auto_load in
    let ready : Gpuio.Table_paging.Status.t -> bool = function
      | Ready -> true
      | Loading | End | Failed _ -> false
    in
    let viewport = Or_error.ok output |> Option.bind ~f:Output.viewport in
    { Demand.identity = D.Expert.identity snapshot.data
    ; query = snapshot.generation
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
           [ demand.Demand.before, Gpuio.Table_paging.Direction.Before
           ; demand.after, After
           ]
           ~f:(fun (needed, direction) ->
             if needed
             then Some (Paging.request paging ~generation:demand.query direction)
             else None))
  in
  B.Edge.on_change ~equal:Demand.equal demand ~callback graph;
  output
;;
