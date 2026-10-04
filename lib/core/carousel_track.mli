open Core

(** Controlled model for the measured-track adapter under construction. The
    public view, bridge, measured rendering, motion, pointer/trackpad/wheel input,
    viewport keyboard and native automatic advancement exist. Full focus/accessibility
    qualification remains unfinished. Native geometry and
    cached observations never introduce a second selected-item owner. *)
module Id = Carousel.Id

module Item = Carousel.Item
module Axis = Carousel.Axis
module Auto_advance = Carousel.Auto_advance

(** Native presentation, independent of model revision. Progress is bounded to
    the segment even with an overshooting cubic curve. Reduced motion, inactive
    windows and unavailable geometry settle immediately. *)
module Motion : sig
  type t [@@deriving equal, sexp_of]

  val immediate : t

  (** 200 milliseconds with ease-out easing. *)
  val default : t

  (** Positive duration, at most ten seconds, rounded up to whole milliseconds.
      The default easing is [Animation.Easing.ease_out]. *)
  val create : ?easing:Animation.Easing.t -> Time_ns.Span.t -> t Or_error.t

  module Expert : sig
    val to_wire : t -> Gpuio_protocol.Carousel_track_wire.Motion.t option
  end
end

module Layout : sig
  type t [@@deriving equal, sexp_of]
end

module Proposal : sig
  type t [@@deriving equal, sexp_of]
end

module Request : sig
  type t = private
    | Previous
    | Next
    | First
    | Last
    | Select of Id.t
    | Layout of Layout.t
    | Auto_next of Proposal.t
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

(** Same item/name/metadata bounds as [Carousel]. Initially no measured layout.
    Defaults: first item, horizontal, enabled, no loop or automatic advancement.
    Freshly constructed models belong to fresh view keys; evolve mounted models
    with these update operations to preserve revision and collection lineage. *)
val create
  :  ?selected:Id.t
  -> ?axis:Axis.t
  -> ?looping:bool
  -> ?disabled:bool
  -> ?auto_advance:Auto_advance.t
  -> 'a Item.t list
  -> 'a t Or_error.t

val items : 'a t -> 'a Item.t list
val selected : 'a t -> 'a Item.t option
val find : 'a t -> Id.t -> 'a Item.t option
val axis : _ t -> Axis.t
val is_looping : _ t -> bool
val is_disabled : _ t -> bool
val auto_advance : _ t -> Auto_advance.t option
val has_layout : _ t -> bool

(** False before a valid measurement, while disabled or when no distinct stop
    exists. Native short-track fallback can prevent wrapping even when requested. *)
val can_previous : _ t -> bool

val can_next : _ t -> bool

(** Item order/identity, axis and loop-policy changes invalidate layout and advance
    collection lineage. Same-order payload/name updates preserve it until a new
    native measurement. Selection survives by ID, with Carousel's removal fallback.
    Revision/lineage exhaustion returns an error without wrapping. *)
val with_items : 'a t -> 'a Item.t list -> 'a t Or_error.t

val with_axis : 'a t -> Axis.t -> 'a t Or_error.t
val with_looping : 'a t -> bool -> 'a t Or_error.t
val with_disabled : 'a t -> bool -> 'a t Or_error.t
val with_auto_advance : 'a t -> Auto_advance.t option -> 'a t Or_error.t

(** Explicit selection works before measurement and while disabled. Missing IDs
    are errors. A duplicate snap position does not erase a logical item. *)
val select : 'a t -> Id.t -> 'a t Or_error.t

(** Reduce ordered manual actions against the latest measured stops. Previous/Next
    skip duplicate positions; First/Last/Select select logical items. Disabled
    manual/automatic requests do nothing. Layout updates still apply while
    disabled, reject stale epochs/lineages and may mark geometry unavailable.
    Auto proposals also require matching source identity, model revision, geometry
    epoch and the latest measured successor. All requests must first pass the
    bridge's live node/handler-generation fence; Expert conversion alone is not
    that fence. A newly mounted source may restart its local geometry epoch. *)
val apply_request : 'a t -> Request.t -> 'a t Or_error.t

val restart_auto_advance : 'a t -> 'a t Or_error.t

module Expert : sig
  val to_wire : _ t -> Gpuio_protocol.Carousel_track_wire.Config.t

  val request_of_wire
    :  window:Gpuio_protocol.Window_id.t
    -> node:Gpuio_protocol.Node_id.t
    -> handler:Gpuio_protocol.Handler_id.t
    -> Gpuio_protocol.Carousel_track_wire.Request.t
    -> Request.t Or_error.t
end
