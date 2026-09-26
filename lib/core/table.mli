open Core

module Direction : sig
  type t =
    | Ascending
    | Descending
  [@@deriving equal, sexp_of]
end

module Sort : sig
  (** Accepted application sort order. The table requests changes; it never sorts
      an incomplete paged dataset itself. Config validates the column is sortable. *)
  type t =
    { column : Table_column.Id.t
    ; direction : Direction.t
    }
  [@@deriving equal, sexp_of]
end

module Selection_mode : sig
  type t =
    | Rows
    | Cells
    | Rows_and_cells
  [@@deriving equal, sexp_of]
end

module Config : sig
  type t [@@deriving equal, sexp_of]

  (** Fixed logical row height defaults to 32 (20..4096); overscan defaults to
      64 (0..1,000,000). The default active-row limit is at most 64, reduced when
      necessary to fit [max_active_cells] (default 4096, maximum 16384).
      All columns in an active row count toward this budget, independently of
      horizontal paint virtualization. Explicit row budgets that exceed it fail.
      The existing global View node/byte budgets also apply to cell content.

      [label] requires 1..1024 UTF-8 bytes without NUL. Cells and rows are selectable
      by default; column selection is opt-in. Disabled input is inert. This pure
      configuration owns no row data, Views, callbacks, timers or native handles. *)
  val create
    :  columns:Table_column.Collection.t
    -> label:string
    -> ?sort:Sort.t
    -> ?row_height:float
    -> ?overscan:float
    -> ?max_active_rows:int
    -> ?max_active_cells:int
    -> ?selection_mode:Selection_mode.t
    -> ?column_selection:bool
    -> ?disabled:bool
    -> ?scrollbar:bool
    -> unit
    -> t Or_error.t

  val columns : t -> Table_column.Collection.t
  val sort : t -> Sort.t option
  val row_height : t -> float
  val overscan : t -> float
  val max_active_rows : t -> int
  val max_active_cells : t -> int
  val selection_mode : t -> Selection_mode.t
  val column_selection : t -> bool
  val is_disabled : t -> bool
  val scrollbar : t -> bool
  val label : t -> string

  (** Immutable replacement validates the existing sort and budgets against the
      new schema. Removing a sorted column requires clearing sort first. *)
  val with_columns : t -> Table_column.Collection.t -> t Or_error.t

  val with_sort : t -> Sort.t option -> t Or_error.t
end

module Cell : sig
  (** Copy text is retained separately from its View: at most 65536 UTF-8 bytes,
      without NUL. Empty copy text is valid. No formatting callback is retained. *)
  type t [@@deriving equal, sexp_of]

  val create : column:Table_column.Id.t -> copy_text:string -> t Or_error.t
  val column : t -> Table_column.Id.t
  val copy_text : t -> string
end

module Selection : sig
  type 'row t =
    | Empty
    | Row of 'row
    | Column of Table_column.Id.t
    | Cell of 'row * Table_column.Id.t
  [@@deriving equal, sexp_of]

  val filter_map : 'a t -> f:('a -> 'b option) -> 'b t option
end

module Request : sig
  (** Native proposals use stable identities. Sorting never mutates data. Context
      Empty clears the context target; it does not open an empty-area menu.
      Native Copy writes complete retained selections up to 1 MiB: a cell is
      verbatim; rows and columns use quoted TSV in display/logical order. Missing
      data or oversized output preserves the clipboard and still emits Copy.
      Copy requests are intents, not acknowledgements of an OS clipboard write;
      applications may handle exports requiring data outside the retained cells. *)
  type 'row t =
    | Select of 'row Selection.t
    | Activate of 'row * Table_column.Id.t option
    | Context of 'row Selection.t
    | Resize of (Table_column.Id.t * float) list
    | Move of Table_column.Id.t * Table_column.Id.t option
    | Sort of Table_column.Id.t * Direction.t option
    | Copy of 'row Selection.t
  [@@deriving equal, sexp_of]

  val filter_map : 'a t -> f:('a -> 'b option) -> 'b t option
end

module Target : sig
  type 'row t =
    | Set_selection of 'row Selection.t
    | Reveal of 'row * Table_column.Id.t option
    | Scroll_to of 'row * float
    | Scroll_to_column of Table_column.Id.t
    | Scroll_to_end
    | Reset_columns
  [@@deriving equal, sexp_of]
end

module Command : sig
  (** Low-level commands for mounted adapters. Serials must strictly increase
      for each new batch; identical retained batches are executed once. Query
      generations fence delayed commands. The adapter validates live targets and
      offsets smaller than the current row height before emitting a transaction. *)
  type 'row t [@@deriving equal, sexp_of]

  val create
    :  serial:int64
    -> query_generation:int64
    -> 'row Target.t
    -> 'row t Or_error.t

  val serial : _ t -> int64
  val query_generation : _ t -> int64
  val target : 'row t -> 'row Target.t
end

module Expert : sig
  (** Revisions belong to the mounted bridge adapter, not application config.
      No native handle is serialized through this conversion. *)
  val to_wire
    :  Config.t
    -> schema_revision:int64
    -> query_generation:int64
    -> Gpuio_protocol.Table_wire.Config.t Or_error.t

  val list_config : Config.t -> Virtual_list.Config.t
  val cell_to_wire : Cell.t -> Gpuio_protocol.Table_wire.Cell.t

  val request_of_wire
    :  Config.t
    -> Gpuio_protocol.Table_wire.Request.t
    -> find_key:(int64 -> 'row option)
    -> 'row Request.t option

  val command_to_wire
    :  Config.t
    -> 'row Command.t
    -> find_id:('row -> int64 option)
    -> Gpuio_protocol.Table_wire.Command.t Or_error.t
end
