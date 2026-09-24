open Core

(** Immutable, ordered application data. Collection lifetime is independent of
    viewport membership. Keys are unique, stable application identities; replacing
    a value preserves its key and position. No row views or Bonsai models are stored.

    Metadata and values occupy O(n) space. Point replacement and lookup take
    O(log n). Structural splices rebuild O(n) positional metadata; they must not
    be used for each streamed text fragment. *)
type ('key, 'data, 'cmp) t

val empty
  :  (module Comparator.S with type t = 'key and type comparator_witness = 'cmp)
  -> ('key, 'data, 'cmp) t

val of_alist
  :  (module Comparator.S with type t = 'key and type comparator_witness = 'cmp)
  -> ('key * 'data) list
  -> ('key, 'data, 'cmp) t Or_error.t

val length : (_, _, _) t -> int
val is_empty : (_, _, _) t -> bool

(** Immutable order snapshot, shared by point updates. Native adapters can cut
    off order processing by snapshot identity without walking every record for
    each streamed fragment. Structural operations create a new snapshot. *)
val keys : ('key, _, _) t -> 'key list

val find : ('key, 'data, _) t -> 'key -> 'data option
val index : ('key, _, _) t -> 'key -> int option
val nth : ('key, 'data, _) t -> int -> ('key * 'data) option

(** A half-open interval. Invalid bounds return an error, including negative
    bounds, a reversed interval, or an endpoint beyond [length]. *)
val range : ('key, 'data, _) t -> first:int -> last:int -> ('key * 'data) list Or_error.t

(** Returns an error for an absent key. A value update never changes ordering. *)
val set
  :  ('key, 'data, 'cmp) t
  -> key:'key
  -> data:'data
  -> ('key, 'data, 'cmp) t Or_error.t

(** Atomically replace [remove] entries beginning at [at]. Reusing keys from the
    removed interval is permitted. Duplicating a key outside it is rejected.
    [at = length t] with [remove = 0] appends; an empty interval at zero prepends. *)
val splice
  :  ('key, 'data, 'cmp) t
  -> at:int
  -> remove:int
  -> ('key * 'data) list
  -> ('key, 'data, 'cmp) t Or_error.t

(** Every existing key must occur exactly once. Values are preserved. *)
val reorder : ('key, 'data, 'cmp) t -> 'key list -> ('key, 'data, 'cmp) t Or_error.t

val to_alist : ('key, 'data, _) t -> ('key * 'data) list

(** Conservative change notification using persistent-map sharing. Visits keys
    added, removed, or whose entry was rebuilt, including position changes.
    No application-data equality is assumed: setting the same value may report
    a change. This is an invalidation query, not a semantic collection diff.
    A point update visits only its key without scanning the unchanged tree. *)
val fold_changed_keys
  :  ('key, 'data, 'cmp) t
  -> previous:('key, 'data, 'cmp) t
  -> init:'acc
  -> f:('acc -> 'key -> 'acc)
  -> 'acc

(** Like [fold_changed_keys], but ignores position-only changes. Explicit [set]
    or replacement rows in [splice] invalidate their keys even when the supplied
    data is physically shared. Reorder/prepend preserve existing value versions,
    avoiding full-history height invalidation. No data equality is required. *)
val fold_changed_values
  :  ('key, 'data, 'cmp) t
  -> previous:('key, 'data, 'cmp) t
  -> init:'acc
  -> f:('acc -> 'key -> 'acc)
  -> 'acc
