open Core
module Id = Choice.Id

module Motion : sig
  type t [@@deriving equal, sexp_of]

  (** The default: change layout immediately. *)
  val immediate : t

  (** Native measured-height reveal with a critically damped spring. Initial
      mounting is settled. Retain permits an inert outgoing visual; Unmount
      removes descendants immediately and skips closing motion. Opening can
      animate newly mounted children. No per-frame OCaml callbacks are used.

      Reduced motion and hidden/inactive windows settle immediately. Motion is
      for ordinary vertical flow. Parent-dependent heights, row/grid placement,
      absolute positioning, or native interaction styles on the panel or its
      immediate parent use immediate layout. The
      border-box height animates; external margins/parent gaps do not. Use panel
      padding and zero parent gap for continuous spacing. Settled open content
      uses ordinary layout, including streaming and wrapping in the same frame. *)
  val standard : t

  (** The same lifecycle and layout rules, with validated custom parameters. *)
  val spring : Animation.Spring.t -> t
end

module Mode : sig
  type t =
    | Single of { allow_empty : bool }
    | Multiple
  [@@deriving equal, sexp_of]
end

module Request : sig
  type t =
    | Expand of Id.t
    | Collapse of Id.t
    | Toggle of Id.t
  [@@deriving equal, sexp_of]
end

(** Application-owned accordion expansion. Collections retain Choice's bounds
    (4096 items, 256 KiB text). Native focus and reveal motion are separate. *)
type t [@@deriving equal, sexp_of]

(** Reject duplicate/unknown expanded IDs and invalid single selection. Required
    single mode has exactly one expanded member when nonempty. Disabled members
    may remain expanded, but user requests cannot change them. *)
val create
  :  items:Choice.Collection.t
  -> mode:Mode.t
  -> expanded:Id.t list
  -> ?disabled:bool
  -> unit
  -> t Or_error.t

val items : t -> Choice.Collection.t
val mode : t -> Mode.t
val expanded : t -> Id.t list
val is_expanded : t -> Id.t -> bool
val is_disabled : t -> bool
val with_disabled : t -> bool -> t

(** Both operations normalize expansion in collection order: drop absent IDs;
    single mode keeps the first remaining expanded item. Required single mode
    without an expansion chooses the first enabled member, or the first member
    if all are disabled. An empty collection always has no expansion. *)
val with_items : t -> Choice.Collection.t -> t

val with_mode : t -> Mode.t -> t

(** Apply against the latest state in delivery order. Missing/disabled IDs and
    all requests while globally disabled do nothing. Required-single collapse
    does nothing. Expanding an enabled item in single mode replaces the previous
    selection (including a disabled historical selection). Repeated toggles are
    separate semantic requests, never stale assignments from a rendered bool.
    Expanded IDs are returned in collection order; operations are bounded by the
    number of configured items. No operation mounts/unmounts a Bonsai computation
    or implicitly cancels application tasks. *)
val apply_request : t -> Request.t -> t

module Expert : sig
  val motion_config
    :  Motion.t
    -> expanded:bool
    -> hidden:Content_policy.t
    -> Gpuio_protocol.Reveal_wire.t option
end
