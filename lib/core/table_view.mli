open Core

(** Fully mounted semantic tables composed from ordinary Views. Use the managed
    Table/Bonsai.Table APIs for large or remotely paged sources. *)
module Cell : sig
  module Kind : sig
    type t =
      | Data
      | Column_header
      | Row_header
    [@@deriving equal, sexp_of]
  end

  type 'action t

  (** Rich content with a stable sibling key and positive column span (1..64).
      Kind defaults to Data; explicitly choose Column_header for header cells.
      Style refines padding, borders, colors and typography. Grid placement is
      owned by the table. Embedded controls keep their native input behavior. *)
  val create
    :  key:Key.t
    -> ?kind:Kind.t
    -> ?span:int
    -> ?style:Style.t
    -> 'action View.t list
    -> 'action t Or_error.t
end

module Row : sig
  type 'action t

  (** Nonempty cells with unique sibling keys, totaling at most 64 columns.
      Final table construction requires exactly its declared column count. *)
  val create : key:Key.t -> ?style:Style.t -> 'action Cell.t list -> 'action t Or_error.t
end

module Section : sig
  type 'action t

  (** Unique row keys; an empty section is valid. *)
  val create : key:Key.t -> ?style:Style.t -> 'action Row.t list -> 'action t Or_error.t
end

(** Header, body sections and footer share 1..64 equal-fraction column tracks.
    Column spans determine actual grid geometry and semantic column indices.
    Row indices count all rows in that order, including headers and footers.
    At most 4,096 rows and 16,384 cells; normal View node/byte budgets also apply.

    Body section keys must be unique. Header/footer/caption use separate keyed
    scopes. Stable section/row/cell keys retain child native controls on updates.
    The table adds no selection, keyboard controller, resource or Tab stop.
    [label] is the explicit accessible name, independent of the visible caption.
    [cell_style] refines default 8px horizontal/4px vertical padding; per-cell
    styles take precedence. Root/group/row layouts and cell placement are owned
    by this builder. Other styles refine normal view presentation. *)
val create
  :  columns:int
  -> label:string
  -> ?key:Key.t
  -> ?style:Style.t
  -> ?cell_style:Style.t
  -> ?header:'action Section.t
  -> ?footer:'action Section.t
  -> ?caption:'action View.t
  -> 'action Section.t list
  -> 'action View.t Or_error.t
