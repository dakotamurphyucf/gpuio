open Core

module Target : sig
  (** A loaded node's controller lease and incarnation, not its list position.
      Holds no payload or historical snapshot. Payload updates, child paging and
      sibling reorder preserve a target; reset, deletion/recreation and a foreign
      controller retire it. View unmount is a separate adapter lifetime gate. *)
  type t

  val capture : _ Tree_loading.Snapshot.t -> Tree.Id.t -> t Or_error.t
  val id : t -> Tree.Id.t
  val equal : t -> t -> bool
  val is_current : t -> _ Tree_loading.Snapshot.t -> bool
end

module Placement : sig
  type t =
    | Before
    | After
    | Inside
  [@@deriving equal, sexp_of]
end

module Move : sig
  (** An application-approved proposal. No hierarchy is changed by reduction.
      Revalidate against the latest snapshot immediately before applying an
      asynchronous approval. Source/destination must remain visible and enabled;
      Inside requires a branch. Self/descendant destinations are rejected. *)
  type t

  val source : t -> Target.t
  val destination : t -> Target.t
  val placement : t -> Placement.t
  val is_current : t -> _ Tree_loading.Snapshot.t -> state:Tree_state.t -> bool
end

module Request : sig
  type t

  (** Relative input reduces against the latest logical cursor. It carries only
      the source lease, so queued Next/Previous requests accumulate in order. *)
  val navigate
    :  _ Tree_loading.Snapshot.t
    -> selection:Tree_state.Selection.t option
    -> Tree_state.Navigation.t
    -> t

  val select : Target.t -> Tree_state.Selection.t -> t
  val focus : Target.t -> t
  val set_expanded : Target.t -> bool -> t
  val activate : Target.t -> t

  (** Programmatic reveal expands loaded ancestors, preserves selection/anchor
      and optionally updates the logical cursor. Disabled targets are ignored;
      disabled ancestors may be expanded as programmatic preferences. It never
      loads unknown descendants or claims native OS focus. *)
  val reveal : Target.t -> focus:bool -> t

  val move : source:Target.t -> destination:Target.t -> Placement.t -> t
end

module Action : sig
  type t =
    | None
    | Activate of Tree.Id.t
    | Move of Move.t
end

module Outcome : sig
  type t

  val state : t -> Tree_state.t
  val action : t -> Action.t

  (** The native adapter should reveal this current stable target, then focus
      only if [focus] is true and the same target still exists when mounted. *)
  val reveal : t -> Target.t option

  val focus : t -> bool
end

(** Pure reduction against the latest loader snapshot, with no I/O/native effects.
    None means stale/ineligible input. User targeting requires a currently visible
    enabled row; programmatic reveal may open its ancestors. Expansion is explicit
    and idempotent, so duplicate accessibility requests never toggle back.
    Selection/focus/navigation do not implicitly activate an item. The adapter
    must also check its mounted lifetime before reducing queued events/effects. *)
val apply : Tree_state.t -> _ Tree_loading.Snapshot.t -> Request.t -> Outcome.t option
