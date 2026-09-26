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

module Config = struct
  type t =
    { columns : Table_column.Collection.t
    ; sort : Sort.t option
    ; row_height : float
    ; overscan : float
    ; max_active_rows : int
    ; max_active_cells : int
    ; selection_mode : Selection_mode.t
    ; column_selection : bool
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
    if W.Config.valid (to_wire t ~schema_revision:1L ~query_generation:0L)
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
        ?(column_selection = false)
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
      { columns
      ; label
      ; sort
      ; row_height
      ; overscan
      ; max_active_rows
      ; max_active_cells
      ; selection_mode
      ; column_selection
      ; disabled
      ; scrollbar
      }
  ;;

  let columns t = t.columns
  let sort t = t.sort
  let row_height t = t.row_height
  let overscan t = t.overscan
  let max_active_rows t = t.max_active_rows
  let max_active_cells t = t.max_active_cells
  let selection_mode t = t.selection_mode
  let column_selection t = t.column_selection
  let is_disabled t = t.disabled
  let scrollbar t = t.scrollbar
  let label t = t.label
  let with_columns t columns = validate { t with columns }
  let with_sort t sort = validate { t with sort }
end

module Expert = struct
  let to_wire config ~schema_revision ~query_generation =
    let wire = Config.to_wire config ~schema_revision ~query_generation in
    if W.Config.valid wire
    then Ok wire
    else Or_error.error_string "invalid table bridge revision"
  ;;
end
