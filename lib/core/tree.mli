open Core

module Id : sig
  (** Stable application identity, independent of labels and native indices.
      Requires 1..256 UTF-8 bytes without NUL. *)
  type t [@@deriving compare, equal, sexp_of]

  include Comparator.S with type t := t

  val of_string : string -> t Or_error.t
  val to_string : t -> string
end

module Children : sig
  (** An empty branch is still a folder. [More None] describes unloaded children;
      loaded IDs form a prefix of the branch's children. Cursors are opaque bytes,
      bounded to 4096 bytes, and are never interpreted as node IDs. *)
  type t =
    | Leaf
    | Branch of
        { ids : Id.t list
        ; next : List_paging.Boundary.t
        }
  [@@deriving equal, sexp_of]
end

module Node : sig
  type 'data t

  (** Labels require 1..4096 UTF-8 bytes without NUL. Child-reference uniqueness
      and membership are checked when admitting the complete forest. Payloads
      remain application-owned and are not serialized or compared by this module. *)
  val create
    :  label:string
    -> ?disabled:bool
    -> children:Children.t
    -> 'data
    -> 'data t Or_error.t

  val label : _ t -> string
  val is_disabled : _ t -> bool
  val children : _ t -> Children.t
  val data : 'data t -> 'data
  val with_data : 'data t -> 'data -> 'data t
end

module Position : sig
  (** Roots have depth 1. Sibling indices are zero-based in the loaded prefix.
      [sibling_count = None] means more siblings may be loaded later. *)
  type t =
    { parent : Id.t option
    ; depth : int
    ; index : int
    ; sibling_count : int option
    }
  [@@deriving equal, sexp_of]
end

(** Immutable loaded forest, with O(n) metadata and no row views or tasks.
    Structural admission costs O(n log n); point payload updates cost O(log n)
    and share the validated topology, order and unchanged payload wrappers.

    Each node belongs to exactly one root or parent's ordered child list.
    Reject duplicates, missing references, multiple parents, unreachable nodes,
    cycles and depth/metadata overflow atomically. Traversal is stack-safe.

    Revisions are local to a collection lineage: call [replace]/[set_data] on
    the latest accepted value. A new [create] starts another lineage; runtime
    controllers must pair these revisions with their own reset generation. *)
type 'data t

val max_nodes : int
val max_depth : int
val max_metadata_bytes : int
val create : roots:Id.t list -> (Id.t * 'data Node.t) list -> 'data t Or_error.t
val revision : _ t -> int64
val length : _ t -> int

(** Conservative text accounting: count each node's ID/label/cursor and every
    root/child ID reference, even when callers physically share their strings. *)
val metadata_bytes : _ t -> int

val roots : _ t -> Id.t list
val preorder : _ t -> Id.t list
val find : 'data t -> Id.t -> 'data Node.t option
val position : _ t -> Id.t -> Position.t option

(** Root-to-parent order, excluding the node itself. An absent node returns
    [None]; a root returns [Some []]. Does not inspect payloads. *)
val ancestors : _ t -> Id.t -> Id.t list option

val to_alist : 'data t -> (Id.t * 'data Node.t) list
val set_data : 'data t -> id:Id.t -> 'data -> 'data t Or_error.t

(** Conservative node invalidation using persistent-map sharing. Visits each
    added/removed ID or replaced node wrapper once, without comparing payloads.
    Point updates skip shared subtrees. Position-only changes are not reported;
    compare [preorder] snapshots separately when projecting hierarchy metadata. *)
val fold_changed_nodes
  :  'data t
  -> previous:'data t
  -> init:'acc
  -> f:('acc -> Id.t -> 'acc)
  -> 'acc

(** Complete atomic structural replacement. Existing IDs preserve incarnation
    even when reordered/moved. Deleted and later reintroduced IDs get a fresh
    incarnation. Changing a parent's children/boundary changes its child revision;
    label/disabled/payload updates and unrelated edits preserve that revision. *)
val replace
  :  'data t
  -> roots:Id.t list
  -> (Id.t * 'data Node.t) list
  -> 'data t Or_error.t

module Expert : sig
  (** Collection-local identity for generation-checked lazy-load adapters.
      Neither number is a globally unique token or a native node handle. *)
  val incarnation : _ t -> Id.t -> int64 option

  val children_revision : _ t -> Id.t -> int64 option
end
