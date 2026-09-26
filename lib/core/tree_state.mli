open Core

module Mode : sig
  type t =
    | Single
    | Multiple
  [@@deriving equal, sexp_of]
end

module Selection : sig
  type t =
    | Replace
    | Toggle
    | Range of { extend : bool }
  [@@deriving equal, sexp_of]
end

module Navigation : sig
  type t =
    | Previous
    | Next
    | First
    | Last
    | Parent
    | Child
  [@@deriving equal, sexp_of]
end

(** Application-owned expansion, selection, range anchor and logical active item.
    This owns no row computations, native focus handles, payloads or tasks. The
    active item is a keyboard cursor, not a claim of actual OS keyboard focus.

    Preferences are bounded by the loaded [Tree]. Hidden selections/expansion
    survive collapse. Deleted/reintroduced IDs are distinguished by incarnation.
    Create fresh state when resetting to a new [Tree.create] lineage; collection-
    local incarnations are not globally unique. Native adapters must additionally
    reject stale event/controller generations before calling user operations. *)
type t

val create
  :  _ Tree.t
  -> ?mode:Mode.t
  -> ?selected:Tree.Id.t list
  -> ?expanded:Tree.Id.t list
  -> unit
  -> t Or_error.t

val mode : t -> Mode.t

(** ID-sorted preference snapshots, independent of visible order. *)
val selected : t -> Tree.Id.t list

val expanded : t -> Tree.Id.t list
val is_selected : t -> Tree.Id.t -> bool
val is_expanded : t -> Tree.Id.t -> bool
val active : t -> Tree.Id.t option
val anchor : t -> Tree.Id.t option

(** Cached visible loaded order. It includes disabled items but excludes children
    of collapsed branches. No row view is built. Selection/focus and payload-only
    updates share this snapshot; hierarchy/expansion changes rebuild it.
    Call [reconcile] after changing the source tree. Other operations do so
    internally before applying their change. *)
val visible : t -> Tree.Id.t list

val visible_index : t -> Tree.Id.t -> int option

(** Visits each ID whose selected/expanded incarnation or logical active state
    changed once. Uses map sharing; does not scan all selected items on a cursor
    move. Visibility, position, mode and anchor changes are separate concerns. *)
val fold_changed_items
  :  t
  -> previous:t
  -> init:'acc
  -> f:('acc -> Tree.Id.t -> 'acc)
  -> 'acc

(** Prune absent/reincarnated preferences and expansion of nodes now made leaves.
    Keep hidden selections. If an active node becomes hidden/disabled/deleted,
    choose its nearest surviving visible enabled ancestor; otherwise use the next
    enabled item at its previous position, or the last preceding enabled item.
    An empty/fully disabled order clears the logical cursor. *)
val reconcile : t -> _ Tree.t -> t

(** Programmatic preferences allow disabled/hidden nodes, reject absent/duplicate
    IDs and leaf expansion. Single selection allows at most one item.
    Switching to Single keeps a selected active item, otherwise the first selected
    item in full tree order. Neither operation chooses an unrelated selection.
    Programmatic selection replacement and mode changes clear the range anchor. *)
val with_selected : t -> _ Tree.t -> Tree.Id.t list -> t Or_error.t

val with_expanded : t -> _ Tree.t -> Tree.Id.t list -> t Or_error.t
val with_mode : t -> _ Tree.t -> Mode.t -> t

(** User operations ignore hidden, disabled or missing targets. Single mode
    treats all selection gestures as Replace. In Multiple mode, Range selects
    enabled items in current visible order; [extend] unions with the old selection.
    Hidden or removed anchors fall back to the current visible active item, then
    the target. Replace/Toggle set the anchor; a range keeps its chosen anchor.
    Choosing a target updates the logical active item but does not activate it. *)
val select : t -> _ Tree.t -> Tree.Id.t -> Selection.t -> t

(** Set one eligible item's selection membership idempotently, without moving
    the cursor or range anchor. In Multiple mode other selections are preserved;
    in Single mode selecting replaces them. Deselecting removes only the target.
    Unlike a gesture, this preserves the desired state of queued AX setters. *)
val set_selected : t -> _ Tree.t -> Tree.Id.t -> bool -> t

val toggle_expanded : t -> _ Tree.t -> Tree.Id.t -> t

(** Update only the logical cursor, with the same user eligibility rules. *)
val focus : t -> _ Tree.t -> Tree.Id.t -> t

(** Logical tree keyboard reduction. Previous/Next/First/Last skip disabled rows.
    Parent collapses an expanded active branch, otherwise chooses its nearest
    enabled ancestor. Child expands a closed branch, otherwise chooses its first
    enabled direct child. Opening/closing preserves selection and range anchor.

    [selection = None] moves only the cursor; [Some gesture] applies that gesture
    to a destination. With no cursor, Previous/Last start at the last enabled row;
    other directions start at the first. This produces no OS focus or load effect;
    adapters observe the result and issue generation-checked native/load requests. *)
val navigate : t -> _ Tree.t -> selection:Selection.t option -> Navigation.t -> t

(** Programmatically expose a loaded enabled target by expanding its ancestors.
    Preserves selection and anchor, and changes the logical cursor only when
    [focus] is true. Disabled ancestors can be expanded as preferences. Missing
    or disabled targets leave the reconciled state unchanged. Native scrolling,
    focus ownership and generation checks belong to the interaction adapter. *)
val reveal : t -> _ Tree.t -> Tree.Id.t -> focus:bool -> t

(** Apply bounded Unicode typeahead to the current visible loaded tree. A match
    moves the logical cursor and replaces selection; a miss only updates the
    bounded prefix. Native adapters supply reset/cycling policy with the input.
    No timer, load, native focus or activation is performed here. *)
val typeahead : t -> _ Tree.t -> Tree_typeahead.Input.t -> t * Tree.Id.t option
