open Core

(** Application-owned selection. Native presentation and deadlines do not own a
    second selected item. Payloads, Bonsai models and Eio tasks stay with the app. *)
module Id : sig
  type t [@@deriving compare, equal, sexp_of]

  val of_string : string -> t Or_error.t
  val to_string : t -> string
end

module Item : sig
  type 'a t

  (** IDs are nonempty UTF-8 without NUL, at most 256 bytes. Labels use the same
      text rules with a 4096-byte bound. Stable IDs preserve native item identity. *)
  val create : id:Id.t -> label:string -> 'a -> 'a t Or_error.t

  val id : _ t -> Id.t
  val label : _ t -> string
  val data : 'a t -> 'a
  val with_data : 'a t -> 'a -> 'a t
end

module Motion = Navigation_stack.Motion

module Axis : sig
  type t =
    | Horizontal
    | Vertical
  [@@deriving equal, sexp_of]
end

module Auto_advance : sig
  type t [@@deriving equal, sexp_of]

  (** Default five seconds; interval is in 1 second..1 hour, rounded up to whole
      milliseconds. Native timing starts after settled visible paint. Hover,
      focus, drag, inactive/hidden window, hidden ancestors and reduced motion
      pause it. Resuming starts a full interval, without catching up missed ticks.
      This specifies the carousel adapter under implementation. *)
  val create : ?interval:Time_ns.Span.t -> unit -> t Or_error.t

  val interval : t -> Time_ns.Span.t
end

module Request : sig
  type t = private
    | Previous
    | Next
    | First
    | Last
    | Select of Id.t
    | Auto_next of
        { revision : int64
        ; from : Id.t
        ; target : Id.t
        }
  [@@deriving equal, sexp_of]

  val previous : t
  val next : t
  val first : t
  val last : t
  val select : Id.t -> t
end

type 'a t

val max_items : int
val max_metadata_bytes : int

(** At most 128 unique items and 256 KiB of IDs/labels. Defaults to the first item,
    no looping, enabled, no auto-advance. Only an empty collection has no selection.
    Ordinary native resource quotas apply independently of the item count.
    Evolve a mounted instance with the update operations below; a freshly created
    model starts a new revision lineage and belongs to a fresh view key. *)
val create
  :  ?selected:Id.t
  -> ?looping:bool
  -> ?disabled:bool
  -> ?auto_advance:Auto_advance.t
  -> 'a Item.t list
  -> 'a t Or_error.t

val items : 'a t -> 'a Item.t list
val selected : 'a t -> 'a Item.t option
val find : 'a t -> Id.t -> 'a Item.t option
val is_looping : _ t -> bool
val is_disabled : _ t -> bool
val auto_advance : _ t -> Auto_advance.t option
val can_previous : _ t -> bool
val can_next : _ t -> bool

(** Preserve selection by ID on reorder. If removed, select the item at the old
    index, clamped to the new length; empty-to-nonempty selects first. Same-order
    label/payload refreshes retain the automatic proposal revision. *)
val with_items : 'a t -> 'a Item.t list -> 'a t Or_error.t

val with_looping : 'a t -> bool -> 'a t Or_error.t
val with_disabled : 'a t -> bool -> 'a t Or_error.t
val with_auto_advance : 'a t -> Auto_advance.t option -> 'a t Or_error.t

(** Explicit selection is allowed while disabled; absent IDs are errors. *)
val select : 'a t -> Id.t -> 'a t Or_error.t

(** Apply manual relative requests to the latest model without coalescing them.
    Disabled/stale requests are harmless. An automatic proposal must match the
    current revision, source and successor. Selection/order/policy changes advance
    the revision, so even looping back to an old source cannot revive an old tick.
    Updates return an error on revision exhaustion; they never wrap or reuse it. *)
val apply_request : 'a t -> Request.t -> 'a t Or_error.t

(** Explicitly invalidate pending automatic proposals and rearm after a rejected
    proposal even if selection/policy is unchanged. At most one native proposal
    waits for a changed revision; ignoring it never creates an event backlog. *)
val restart_auto_advance : 'a t -> 'a t Or_error.t

module Expert : sig
  val to_wire : _ t -> axis:Axis.t -> Gpuio_protocol.Carousel_wire.Config.t
  val request_of_wire : Gpuio_protocol.Carousel_wire.Request.t -> Request.t Or_error.t
end
