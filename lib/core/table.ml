open Core
module W = Gpuio_protocol.Table_wire

module Direction = struct
  type t =
    | Ascending
    | Descending
  [@@deriving equal, sexp_of]
end

module Sort = struct
  type t =
    { column : Table_column.Id.t
    ; direction : Direction.t
    }
  [@@deriving equal, sexp_of]
end

module Selection_mode = struct
  type t =
    | Rows
    | Cells
    | Rows_and_cells
  [@@deriving equal, sexp_of]
end

module Boundary = W.Boundary
module Appearance = Table_appearance

module Column_viewport = struct
  module Column = struct
    type t =
      { id : Table_column.Id.t
      ; pin : Table_column.Pin.t
      ; fully_visible : bool
      }
    [@@deriving equal, sexp_of]

    let id t = t.id
    let pin t = t.pin
    let fully_visible t = t.fully_visible
  end

  type t = Column.t list [@@deriving equal, sexp_of]

  let columns t = t
end

module Config = struct
  type t =
    { appearance : Appearance.t
    ; columns : Table_column.Collection.t
    ; sort : Sort.t option
    ; row_height : float
    ; overscan : float
    ; max_active_rows : int
    ; max_active_cells : int
    ; selection_mode : Selection_mode.t
    ; column_selection : bool
    ; row_header : bool
    ; boundary : W.Boundary.t
    ; selectable_headers : Table_column.Id.t list option
    ; disabled : bool
    ; scrollbar : bool
    ; label : string
    }
  [@@deriving equal, sexp_of]

  let to_wire t ~schema_revision ~query_generation : W.Config.t =
    { schema_revision
    ; query_generation
    ; schema = Table_column.Expert.to_wire t.columns
    ; sort =
        Option.map t.sort ~f:(fun sort ->
          { W.Sort.column = Table_column.Id.to_string sort.Sort.column
          ; direction =
              (match sort.direction with
               | Ascending -> W.Direction.Ascending
               | Descending -> Descending)
          })
    ; row_height = t.row_height
    ; overscan = t.overscan
    ; max_active_rows = Int64.of_int t.max_active_rows
    ; max_active_cells = Int64.of_int t.max_active_cells
    ; selection_mode =
        (match t.selection_mode with
         | Rows -> W.Selection_mode.Rows
         | Cells -> Cells
         | Rows_and_cells -> Rows_and_cells)
    ; column_selection = t.column_selection
    ; disabled = t.disabled
    ; scrollbar = t.scrollbar
    ; label = t.label
    }
  ;;

  let validate t =
    if not (Appearance.Expert.valid_columns t.appearance t.columns)
    then Or_error.error_string "table padding refers to an unknown column"
    else if
      not
        (Option.for_all t.selectable_headers ~f:(fun ids ->
           List.length ids <= Table_column.Collection.max_columns
           && (not (List.contains_dup ids ~compare:Table_column.Id.compare))
           && List.for_all ids ~f:(fun id ->
             Option.is_some (Table_column.Collection.find t.columns id))))
    then
      Or_error.error_string "selectable table headers must be distinct existing columns"
    else if W.Config.valid (to_wire t ~schema_revision:1L ~query_generation:0L)
    then Ok t
    else
      Or_error.error_string
        "invalid table geometry, active-cell budget, sort column or label"
  ;;

  let create
        ~columns
        ~label
        ?sort
        ?(row_height = 32.)
        ?(overscan = 64.)
        ?max_active_rows
        ?(max_active_cells = 4096)
        ?(selection_mode = Selection_mode.Rows_and_cells)
        ?(appearance = Appearance.default)
        ?(column_selection = false)
        ?(row_header = true)
        ?(boundary = W.Boundary.Wrap)
        ?selectable_headers
        ?(disabled = false)
        ?(scrollbar = true)
        ()
    =
    let count = List.length (Table_column.Collection.to_list columns) in
    let max_active_rows =
      Option.value
        max_active_rows
        ~default:(Int.min 64 (max_active_cells / Int.max 1 count))
    in
    validate
      { appearance
      ; columns
      ; label
      ; sort
      ; row_height
      ; overscan
      ; max_active_rows
      ; max_active_cells
      ; selection_mode
      ; column_selection
      ; row_header
      ; boundary
      ; selectable_headers
      ; disabled
      ; scrollbar
      }
  ;;

  let appearance t = t.appearance
  let with_appearance t appearance = validate { t with appearance }
  let columns t = t.columns
  let sort t = t.sort
  let row_height t = t.row_height
  let overscan t = t.overscan
  let max_active_rows t = t.max_active_rows
  let max_active_cells t = t.max_active_cells
  let selection_mode t = t.selection_mode
  let column_selection t = t.column_selection
  let row_header t = t.row_header
  let boundary t = t.boundary
  let selectable_headers t = t.selectable_headers
  let with_row_header t row_header = { t with row_header }
  let with_boundary t boundary = { t with boundary }

  let with_selectable_headers t selectable_headers =
    validate { t with selectable_headers }
  ;;

  let header_selectable t id =
    t.column_selection
    && Option.is_some (Table_column.Collection.find t.columns id)
    && Option.for_all t.selectable_headers ~f:(fun ids ->
      List.mem ids id ~equal:Table_column.Id.equal)
  ;;

  let is_disabled t = t.disabled
  let scrollbar t = t.scrollbar
  let label t = t.label
  let with_columns t columns = validate { t with columns }
  let with_sort t sort = validate { t with sort }
end

module Cell = struct
  type t =
    { column : Table_column.Id.t
    ; copy_text : string
    }
  [@@deriving equal, sexp_of]

  let to_wire t : W.Cell.t =
    { column = Table_column.Id.to_string t.column; copy_text = t.copy_text }
  ;;

  let create ~column ~copy_text =
    let t = { column; copy_text } in
    if W.Cell.valid (to_wire t)
    then Ok t
    else Or_error.error_string "invalid table cell copy text"
  ;;

  let column t = t.column
  let copy_text t = t.copy_text
end

module Selection = struct
  type 'row t =
    | Empty
    | Row of 'row
    | Column of Table_column.Id.t
    | Cell of 'row * Table_column.Id.t
  [@@deriving equal, sexp_of]

  let filter_map t ~f =
    match t with
    | Empty -> Some Empty
    | Row row -> Option.map (f row) ~f:(fun row -> Row row)
    | Column col -> Some (Column col)
    | Cell (row, col) -> Option.map (f row) ~f:(fun row -> Cell (row, col))
  ;;
end

module Request = struct
  type 'row t =
    | Select of 'row Selection.t
    | Activate of 'row * Table_column.Id.t option
    | Context of 'row Selection.t
    | Resize of (Table_column.Id.t * float) list
    | Move of Table_column.Id.t * Table_column.Id.t option
    | Sort of Table_column.Id.t * Direction.t option
    | Copy of 'row Selection.t
  [@@deriving equal, sexp_of]

  let filter_map t ~f =
    match t with
    | Select s -> Option.map (Selection.filter_map s ~f) ~f:(fun s -> Select s)
    | Context s -> Option.map (Selection.filter_map s ~f) ~f:(fun s -> Context s)
    | Copy s -> Option.map (Selection.filter_map s ~f) ~f:(fun s -> Copy s)
    | Activate (r, c) -> Option.map (f r) ~f:(fun r -> Activate (r, c))
    | Resize widths -> Some (Resize widths)
    | Move (c, before) -> Some (Move (c, before))
    | Sort (c, dir) -> Some (Sort (c, dir))
  ;;
end

module Target = struct
  type 'row t =
    | Set_selection of 'row Selection.t
    | Reveal of 'row * Table_column.Id.t option
    | Scroll_to of 'row * float
    | Scroll_to_column of Table_column.Id.t
    | Scroll_to_end
    | Reset_columns
  [@@deriving equal, sexp_of]
end

module Command = struct
  type 'row t =
    { serial : int64
    ; query_generation : int64
    ; target : 'row Target.t
    }
  [@@deriving equal, sexp_of]

  let create ~serial ~query_generation target =
    let valid_target =
      match target with
      | Target.Scroll_to (_, offset) ->
        Float.is_finite offset && Float.(offset >= 0. && offset <= 4096.)
      | Set_selection _ | Reveal _ | Scroll_to_column _ | Scroll_to_end | Reset_columns ->
        true
    in
    if Int64.(serial > 0L && query_generation >= 0L) && valid_target
    then Ok { serial; query_generation; target }
    else Or_error.error_string "invalid table command serial, query or offset"
  ;;

  let serial t = t.serial
  let query_generation t = t.query_generation
  let target t = t.target
end

module Expert = struct
  let appearance_to_wire config ~theme =
    if Appearance.equal (Config.appearance config) Appearance.default
    then Ok None
    else
      Or_error.map
        (Appearance.Expert.to_wire (Config.appearance config) ~theme)
        ~f:Option.some
  ;;

  let behavior_to_wire config =
    if
      Config.row_header config
      && Boundary.equal (Config.boundary config) Wrap
      && Option.is_none (Config.selectable_headers config)
    then None
    else
      Some
        { W.Behavior.row_header = Config.row_header config
        ; boundary = Config.boundary config
        ; selectable_headers =
            Option.map
              (Config.selectable_headers config)
              ~f:(List.map ~f:Table_column.Id.to_string)
        }
  ;;

  let to_wire config ~schema_revision ~query_generation =
    let wire = Config.to_wire config ~schema_revision ~query_generation in
    if W.Config.valid wire
    then Ok wire
    else Or_error.error_string "invalid table bridge revision"
  ;;

  let list_config t =
    Virtual_list.Config.create
      ~height:(Fixed (Config.row_height t))
      ~overscan:(Config.overscan t)
      ~max_active:(Config.max_active_rows t)
      ~scroll:Keep_position
      ~scrollbar:(Config.scrollbar t)
      ()
    |> Or_error.ok_exn
  ;;

  let cell_to_wire = Cell.to_wire

  let column config name =
    Or_error.bind (Table_column.Id.of_string name) ~f:(fun id ->
      Option.value_map
        (Table_column.Collection.find (Config.columns config) id)
        ~default:(Or_error.error_string "unknown table column")
        ~f:Or_error.return)
    |> Result.ok
  ;;

  let column_id config name = Option.map (column config name) ~f:Table_column.id

  let column_viewport_of_wire config (viewport : W.Column_viewport.t) =
    if not (W.Column_viewport.valid viewport)
    then None
    else
      List.map viewport.columns ~f:(fun (name, pin, fully_visible) ->
        Option.bind (column config name) ~f:(fun column ->
          let pin =
            match pin with
            | W.Pin.Left -> Table_column.Pin.Left
            | Unpinned -> Unpinned
          in
          if Table_column.Pin.equal pin (Table_column.pin column)
          then
            Some
              { Column_viewport.Column.id = Table_column.id column; pin; fully_visible }
          else None))
      |> Option.all
  ;;

  let optional_column config = function
    | None -> Some None
    | Some name -> Option.map (column_id config name) ~f:Option.some
  ;;

  let selection_of_wire config selection ~find_key =
    let open Option.Let_syntax in
    match selection with
    | W.Selection.Empty -> Some Selection.Empty
    | Row row when not (Selection_mode.equal (Config.selection_mode config) Cells) ->
      let%map row = find_key row in
      Selection.Row row
    | Column col when Config.column_selection config ->
      let%bind col = column_id config col in
      if Config.header_selectable config col then Some (Selection.Column col) else None
    | Cell (row, col) when not (Selection_mode.equal (Config.selection_mode config) Rows)
      ->
      let%bind row = find_key row in
      let%map col = column_id config col in
      Selection.Cell (row, col)
    | Row _ | Column _ | Cell _ -> None
  ;;

  let request_of_wire config request ~find_key =
    if Config.is_disabled config || not (W.Request.valid request)
    then None
    else
      let open Option.Let_syntax in
      match request with
      | W.Request.Select s ->
        let%map s = selection_of_wire config s ~find_key in
        Request.Select s
      | Context s ->
        let%map s = selection_of_wire config s ~find_key in
        Request.Context s
      | Copy W.Selection.Empty -> None
      | Copy s ->
        let%map s = selection_of_wire config s ~find_key in
        Request.Copy s
      | Activate (row, col) ->
        let%bind row = find_key row in
        let%map col = optional_column config col in
        Request.Activate (row, col)
      | Resize widths ->
        let%map widths =
          Option.all
            (List.map widths ~f:(fun (name, width) ->
               let%bind col = column config name in
               if
                 Float.(
                   width >= Table_column.min_width col
                   && width <= Table_column.max_width col)
                 && (Table_column.is_resizable col
                     || Float.equal width (Table_column.width col))
               then Some (Table_column.id col, width)
               else None))
        in
        Request.Resize widths
      | Move (col, before) ->
        let%bind col = column_id config col in
        let%bind before = optional_column config before in
        let%map _ =
          Table_column.Collection.move (Config.columns config) ~column:col ~before
          |> Result.ok
        in
        Request.Move (col, before)
      | Sort (name, dir) ->
        let%bind col = column config name in
        if not (Table_column.is_sortable col)
        then None
        else
          Some
            (Request.Sort
               ( Table_column.id col
               , Option.map dir ~f:(function
                   | W.Direction.Ascending -> Direction.Ascending
                   | Descending -> Descending) ))
  ;;

  let command_to_wire config command ~find_id =
    let row key =
      Option.value_map
        (find_id key)
        ~default:(Or_error.error_string "unknown table row")
        ~f:Or_error.return
    in
    let col id =
      let name = Table_column.Id.to_string id in
      if Option.is_some (column config name)
      then Ok name
      else Or_error.error_string "unknown table column"
    in
    let open Or_error.Let_syntax in
    let%bind target =
      match Command.target command with
      | Target.Set_selection selection ->
        let%bind selection =
          match selection with
          | Selection.Empty -> Ok W.Selection.Empty
          | Row key ->
            let%map row = row key in
            W.Selection.Row row
          | Column id ->
            let%map col = col id in
            W.Selection.Column col
          | Cell (key, id) ->
            let%bind row = row key in
            let%map col = col id in
            W.Selection.Cell (row, col)
        in
        if Option.is_some (selection_of_wire config selection ~find_key:Option.some)
        then Ok (W.Target.Set_selection selection)
        else Or_error.error_string "table selection mode disallows command"
      | Reveal (key, id) ->
        let%bind row = row key in
        let%map col =
          match id with
          | None -> Ok None
          | Some id -> Or_error.map (col id) ~f:Option.some
        in
        W.Target.Reveal (row, col)
      | Scroll_to (key, offset) ->
        let%bind row = row key in
        if Float.(offset < Config.row_height config)
        then Ok (W.Target.Scroll_to (row, offset))
        else Or_error.error_string "table offset must be smaller than row height"
      | Scroll_to_column id ->
        let%map col = col id in
        W.Target.Scroll_to_column col
      | Scroll_to_end -> Ok W.Target.Scroll_to_end
      | Reset_columns -> Ok W.Target.Reset_columns
    in
    let wire : W.Command.t =
      { serial = Command.serial command
      ; query_generation = Command.query_generation command
      ; target
      }
    in
    if W.Command.valid wire
    then Ok wire
    else Or_error.error_string "invalid table command"
  ;;
end
