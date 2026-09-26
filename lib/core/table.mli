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

module Expert : sig
  (** Revisions belong to the mounted bridge adapter, not application config.
      No native handle is serialized through this conversion. *)
  val to_wire
    :  Config.t
    -> schema_revision:int64
    -> query_generation:int64
    -> Gpuio_protocol.Table_wire.Config.t Or_error.t
end
