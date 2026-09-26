open Core

module Id : sig
  (** Stable application row identity, distinct from [Table_column.Id].
      Requires 1..256 UTF-8 bytes without NUL; labels/positions are not IDs. *)
  type t [@@deriving compare, equal, sexp_of]

  include Comparator.S with type t := t

  val of_string : string -> t Or_error.t
  val to_string : t -> string
end

module Source_id : sig
  (** Process-local lineage identity with no application payload. *)
  type t [@@deriving compare, equal, sexp_of]

  include Comparator.S with type t := t
end

module Row_ref : sig
  (** One membership lifetime of a row in one source lineage. Contains no
      application payload or collection snapshot. Removing and later adding
      the same ID creates a different lifetime. Not a serialized native handle. *)
  type t [@@deriving compare, equal, sexp_of]

  include Comparator.S with type t := t

  val id : t -> Id.t
end

(** Immutable, ordered application data. It owns no cells, views, Bonsai models,
    tasks or native handles. Retaining a source retains its application payloads;
    this is separate from the widget's bounded active-cell/cache contract.

    Point updates are O(log n) and share order/membership snapshots. Structural
    updates are O(n log n). No operation formats cells or sorts application
    values; the application supplies the accepted order. Revision numbers are
    local to a lineage: adapters pair them with [same_source] and their own query
    generation, and process changes from the latest accepted source. *)
type 'data t

val max_rows : int
val max_key_bytes : int

(** At most 1,000,000 rows and 64 MiB of key bytes, counted once per row.
    Reject duplicate keys atomically. A new [create] starts an independent
    lineage, even if supplied the same IDs and values. Empty sources are valid. *)
val create : (Id.t * 'data) list -> 'data t Or_error.t

val length : _ t -> int
val is_empty : _ t -> bool
val revision : _ t -> int64
val key_bytes : _ t -> int
val same_source : _ t -> _ t -> bool
val keys : _ t -> Id.t list
val find : 'data t -> Id.t -> 'data option
val index : _ t -> Id.t -> int option
val nth : 'data t -> int -> (Id.t * 'data) option
val range : 'data t -> first:int -> last:int -> (Id.t * 'data) list Or_error.t
val to_alist : 'data t -> (Id.t * 'data) list
val row_ref : _ t -> Id.t -> Row_ref.t option
val contains_ref : _ t -> Row_ref.t -> bool

(** Replacing a payload preserves key, position and membership lifetime. *)
val set : 'data t -> key:Id.t -> data:'data -> 'data t Or_error.t

(** Structural admission is atomic. Keys present in both old and new sources
    keep membership lifetime, including keys replaced in a single splice.
    Separate removal and reinsertion retire the old lifetime. No history of
    removed IDs is kept. Invalid changes leave the old source usable. *)
val replace : 'data t -> (Id.t * 'data) list -> 'data t Or_error.t

val splice : 'data t -> at:int -> remove:int -> (Id.t * 'data) list -> 'data t Or_error.t

(** Every existing ID must occur exactly once; payload versions and membership
    lifetimes are preserved. Query/sort cancellation is owned by the pager,
    not this pure operation. Never locally sort only the loaded portion of a
    remotely sorted query and present it as the complete dataset's order. *)
val reorder : 'data t -> Id.t list -> 'data t Or_error.t

(** Conservative invalidation, using persistent-map sharing. Visits additions,
    removals and explicit payload replacements; ignores position-only changes.
    Requires the same source lineage, rejecting unrelated snapshots. *)
val fold_changed_values
  :  'data t
  -> previous:'data t
  -> init:'acc
  -> f:('acc -> Id.t -> 'acc)
  -> 'acc Or_error.t

module Expert : sig
  module Identity : sig
    (** Shared immutable order/membership metadata, with no row payloads.
        Point updates share it; structural changes replace it. *)
    type t

    val source_id : t -> Source_id.t
    val rows : t -> Row_ref.t list
  end

  val identity : _ t -> Identity.t
  val row_key : Row_ref.t -> Key.t

  (** Shared data snapshot for bounded managed-row adapters. No mapping or
      copying takes place. Native row allocation and query epochs remain the
      adapter's responsibility. *)
  val items : 'data t -> (Id.t, 'data, Id.comparator_witness) List_collection.t
end
