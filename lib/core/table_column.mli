open Core

module Id : sig
  (** Stable column identity, independent of its label, width and position. *)
  type t [@@deriving compare, equal, sexp_of]

  include Comparator.S with type t := t

  (** 1..256 UTF-8 bytes without NUL. *)
  val of_string : string -> t Or_error.t

  val to_string : t -> string
end

module Pin : sig
  type t =
    | Unpinned
    | Left
  [@@deriving equal, sexp_of]
end

module Alignment : sig
  type t =
    | Left
    | Center
    | Right
  [@@deriving equal, sexp_of]
end

(** Immutable column description. Sizes are logical pixels. Native resize/reorder
    gestures propose changes; the application owns the accepted description. *)
type t [@@deriving equal, sexp_of]

(** Width/min/max must be finite, with [20 <= min_width <= width <= max_width
    <= 16384]. Defaults: width 160, min 40, max 4096. Label requires 1..4096 UTF-8
    bytes without NUL. Pin defaults to Unpinned; alignment to Left. Columns are
    resizable/movable by default; sorting is opt-in and application-owned. *)
val create
  :  id:Id.t
  -> label:string
  -> ?width:float
  -> ?min_width:float
  -> ?max_width:float
  -> ?pin:Pin.t
  -> ?alignment:Alignment.t
  -> ?resizable:bool
  -> ?movable:bool
  -> ?sortable:bool
  -> unit
  -> t Or_error.t

val id : t -> Id.t
val label : t -> string
val width : t -> float
val min_width : t -> float
val max_width : t -> float
val pin : t -> Pin.t
val alignment : t -> Alignment.t
val is_resizable : t -> bool
val is_movable : t -> bool
val is_sortable : t -> bool

(** Programmatic sizing rejects out-of-range values. [is_resizable] restricts
    user gestures, not programmatic updates. *)
val with_width : t -> float -> t Or_error.t

module Group : sig
  (** Header membership is keyed, never inferred from stale positional spans.
      A group has 1..64 distinct column IDs and a 1..4096-byte UTF-8 label. *)
  type t [@@deriving equal, sexp_of]

  val create : label:string -> columns:Id.t list -> t Or_error.t
  val label : t -> string
  val columns : t -> Id.t list
end

module Collection : sig
  type column = t
  type t [@@deriving equal, sexp_of]

  val max_columns : int
  val max_header_levels : int
  val max_text_bytes : int

  (** At most 64 columns, four additional header levels, and 256 KiB of text
      including repeated group member IDs. Empty schemas are valid without
      groups. Column IDs must be unique; all Left-pinned columns must precede
      unpinned columns. Each header level partitions every column exactly once
      into contiguous groups in display order. Lower levels refine upper levels;
      groups may not cross the pinned/unpinned boundary. Invalid schemas fail
      atomically rather than silently changing pinning or group membership. *)
  val create : ?header_groups:Group.t list list -> column list -> t Or_error.t

  val to_list : t -> column list
  val header_groups : t -> Group.t list list
  val find : t -> Id.t -> column option
  val index : t -> Id.t -> int option
  val pinned_count : t -> int
  val total_width : t -> float

  (** A user resize proposal: reject unknown/non-resizable columns and nonfinite
      values, otherwise clamp to that column's limits. Group widths are derived
      from members; no separate group sizing state can become stale. *)
  val resize : t -> column:Id.t -> width:float -> t Or_error.t

  (** Move a user-movable column immediately before another column, or to the
      end for [None]. Other columns retain relative order. The destination must
      exist; moving before self is a no-op. Pin partitions and all group
      memberships must remain valid. A non-movable column cannot be the source,
      but another column's move may change its numeric position. Group members
      are reordered to match display order after a valid move. *)
  val move : t -> column:Id.t -> before:Id.t option -> t Or_error.t
end

module Expert : sig
  (** Validated paired bridge schema. Decoding uses the same smart constructors
      as application code, including group refinement and pin boundaries. *)
  val to_wire : Collection.t -> Gpuio_protocol.Table_wire.Schema.t

  val of_wire : Gpuio_protocol.Table_wire.Schema.t -> Collection.t Or_error.t
end
