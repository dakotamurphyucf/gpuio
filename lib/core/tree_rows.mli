open Core

module Key : sig
  (** Transient projection identity, separate from application [Tree.Id]. Never
      persist it as a preference. Keys survive value/state changes and reorder;
      collapsed, deleted and reincarnated rows receive fresh keys on reappearance. *)
  type t [@@deriving compare, equal, sexp_of]

  include Comparator.S with type t := t

  val to_view_key : t -> Key.t
end

module Item : sig
  type 'data t = private
    { id : Tree.Id.t
    ; node : 'data Tree.Node.t
    ; position : Tree.Position.t
    ; selected : bool
    ; expanded : bool option
    ; active : bool
    ; loading : Tree_loading.Status.t option
    }

  (** TreeItem semantics from this validated row snapshot, including its label,
      unknown sibling total, expansion/selection/disabled and load-busy state.
      Apply to the row's container; the list adapter transfers it to the native
      row wrapper so there is one accessible item per application node. *)
  val accessibility : _ t -> Accessibility.t
end

module Boundary : sig
  (** An expanded branch's incomplete child prefix ends in one synthetic row,
      after all loaded descendants. It is not a selectable application item and
      does not contribute to sibling counts. [depth] is its parent's depth + 1. *)
  type t = private
    { parent : Tree.Id.t
    ; depth : int
    ; status : Tree_loading.Status.t
    }
end

module Row : sig
  type 'data t =
    | Item of 'data Item.t
    | Boundary of Boundary.t
end

(** Immutable projection into OCH-13's keyed data collection, not rendered views.
    At most two rows per loaded node and no historical ID registry, row models,
    fibers or native handles. Owns one current source snapshot and reconciled
    preference state; callers retaining old projections retain their old payloads.

    Topology/expansion changes rebuild metadata in O(n log n). Payload, selection,
    cursor and load-status changes use shared-map invalidation and update only
    affected visible rows, sharing collection keys and unchanged value wrappers.
    No arbitrary payload comparison is performed. *)
type 'data t

val create : 'data Tree_loading.Snapshot.t -> state:Tree_state.t -> 'data t Or_error.t

(** Use the latest projection of the same loader instance and generation. A reset requires
    [create] with fresh preferences; backwards source revisions and generation
    changes are rejected atomically. Compact monotonic keys never encode or hash
    user IDs, so 256-byte IDs and synthetic rows cannot collide. *)
val update
  :  'data t
  -> 'data Tree_loading.Snapshot.t
  -> state:Tree_state.t
  -> 'data t Or_error.t

val source : 'data t -> 'data Tree_loading.Snapshot.t
val state : _ t -> Tree_state.t
val collection : 'data t -> (Key.t, 'data Row.t, Key.comparator_witness) List_collection.t
val item_key : _ t -> Tree.Id.t -> Key.t option
val boundary_key : _ t -> Tree.Id.t -> Key.t option

(** Returns a current row only. A retained key for a collapsed/deleted row cannot
    target a different row that later occupies the same list position. *)
val find : 'data t -> Key.t -> 'data Row.t option
