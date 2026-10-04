open Core

module Mode : sig
  type t =
    | Single
    | Multiple
  [@@deriving equal, sexp_of]
end

module Boundary : sig
  type t =
    | Stop
    | Wrap
  [@@deriving equal, sexp_of]
end

module Navigation : sig
  type t =
    | Previous
    | Next
    | First
    | Last
  [@@deriving equal, sexp_of]
end

module Gesture : sig
  type t =
    | Replace
    | Toggle
    | Range of { extend : bool }
  [@@deriving equal, sexp_of]
end

module Confirmation : sig
  type t =
    | Primary
    | Secondary
  [@@deriving equal, sexp_of]
end

module Catalog : sig
  (** Metadata only, independent of application payloads and native row views.
      Reuse a catalog until source identity/order, visibility or eligibility changes.
      Construction is O(n log n), bounded by one million loaded/visible entries.
      Disabled keys may be hidden; visible keys must be a unique subset of loaded
      keys. Omitted [visible] uses the loaded order. No equality over payloads. *)
  type ('key, 'cmp) t

  val create
    :  ('key, 'cmp) List_collection.Identity.t
    -> ?visible:'key list
    -> ?disabled:'key list
    -> unit
    -> ('key, 'cmp) t Or_error.t

  val identity : ('key, 'cmp) t -> ('key, 'cmp) List_collection.Identity.t
  val visible : ('key, _) t -> 'key list
  val visible_index : ('key, 'cmp) t -> 'key -> int option

  (** True only for a visible entry that is not disabled. *)
  val is_enabled : ('key, 'cmp) t -> 'key -> bool
end

(** Application-owned logical cursor, committed membership, range anchor and
    context target. This is not native focus. All operations reduce against the
    latest catalog; stale/foreign/deleted target references are ignored.
    Keep a fresh model for an independent source; reconciliation clears all
    state if source identity changes. Point payload updates need no reconciliation.
    Operations on an unchanged catalog avoid full-source/selection scans. *)
type ('key, 'cmp) t

val create
  :  ('key, 'cmp) Catalog.t
  -> ?mode:Mode.t
  -> ?selected:'key list
  -> unit
  -> ('key, 'cmp) t Or_error.t

val mode : (_, _) t -> Mode.t
val cursor : ('key, _) t -> 'key List_collection.Item_ref.t option
val anchor : ('key, _) t -> 'key List_collection.Item_ref.t option
val context : ('key, _) t -> 'key List_collection.Item_ref.t option

(** Key-comparator order, independent of visible order. Hidden/disabled committed
    selections survive filtering; absent/reincarnated selections are pruned. *)
val selected : ('key, _) t -> 'key List_collection.Item_ref.t list

val is_selected : ('key, 'cmp) t -> 'key -> bool

(** Repair an ineligible cursor at its old visible index: next enabled, then
    preceding. An empty/all-disabled order clears cursor. A cleared cursor stays
    clear. Invalid anchors/context are cleared; hidden selections stay committed.
    Reconciliation scans selected metadata only when the catalog changes. *)
val reconcile : ('key, 'cmp) t -> ('key, 'cmp) Catalog.t -> ('key, 'cmp) t

(** Programmatic replacement rejects duplicate/absent keys and multiple values
    in Single mode. Hidden/disabled loaded keys are allowed. Clears range anchor. *)
val with_selected
  :  ('key, 'cmp) t
  -> ('key, 'cmp) Catalog.t
  -> 'key list
  -> ('key, 'cmp) t Or_error.t

(** Single keeps a selected cursor, otherwise the first selected loaded key in
    source order. It never selects an unrelated item. Clears range anchor. *)
val with_mode : ('key, 'cmp) t -> ('key, 'cmp) Catalog.t -> Mode.t -> ('key, 'cmp) t

val focus
  :  ('key, 'cmp) t
  -> ('key, 'cmp) Catalog.t
  -> 'key List_collection.Item_ref.t
  -> ('key, 'cmp) t

(** Selecting also moves the cursor. Range skips disabled keys and excludes hidden
    selections unless [extend]; a missing anchor falls back to eligible cursor,
    then target. Single treats every gesture as Replace. *)
val select
  :  ('key, 'cmp) t
  -> ('key, 'cmp) Catalog.t
  -> 'key List_collection.Item_ref.t
  -> Gesture.t
  -> ('key, 'cmp) t

(** Idempotent native accessibility intent; changes membership without moving the
    cursor or anchor. Ignores hidden/disabled/obsolete references. *)
val set_selected
  :  ('key, 'cmp) t
  -> ('key, 'cmp) Catalog.t
  -> 'key List_collection.Item_ref.t
  -> bool
  -> ('key, 'cmp) t

(** Relative navigation skips disabled entries in O(log n). With no cursor,
    Previous/Last choose the last enabled entry; Next/First choose the first.
    [selection=None] moves only the cursor; defaults are Stop and no selection. *)
val navigate
  :  ('key, 'cmp) t
  -> ('key, 'cmp) Catalog.t
  -> ?boundary:Boundary.t
  -> ?selection:Gesture.t
  -> Navigation.t
  -> ('key, 'cmp) t

(** Independent secondary target; never implicitly changes cursor/selection. *)
val with_context
  :  ('key, 'cmp) t
  -> ('key, 'cmp) Catalog.t
  -> 'key List_collection.Item_ref.t option
  -> ('key, 'cmp) t

(** Resolve the current eligible cursor as an activation intent, with no implicit
    selection mutation. Native adapters separately fence controller/handler epochs. *)
val confirm
  :  ('key, 'cmp) t
  -> ('key, 'cmp) Catalog.t
  -> Confirmation.t
  -> ('key List_collection.Item_ref.t * Confirmation.t) option

(** Clear cursor, range anchor and context; retain committed selection. Does not
    clear a search query or cancel application tasks. *)
val cancel : ('key, 'cmp) t -> ('key, 'cmp) Catalog.t -> ('key, 'cmp) t
