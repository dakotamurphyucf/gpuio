open Core

(** Checked presentation for an existing native viewport. This value owns no
    scroll handle, offset, callbacks or timer. *)
module Axis : sig
  type t =
    | Horizontal
    | Vertical
    | Both
  [@@deriving equal, sexp_of]
end

module Mode : sig
  type t =
    | Scrolling
    | Hover
    | Always
  [@@deriving equal, sexp_of]
end

module Entrance : sig
  type t =
    | Fade
    | Slide_and_fade
  [@@deriving equal, sexp_of]
end

module Track : sig
  type t [@@deriving equal, sexp_of]

  (** Width is across the scrolling axis, finite in 0..16384 logical pixels.
      Omitted fields inherit; transparent background is an explicit override. *)
  val create
    :  ?background:Color.t
    -> ?border:Color.t
    -> ?width:float
    -> unit
    -> t Or_error.t

  val empty : t
end

module Thumb : sig
  type t [@@deriving equal, sexp_of]

  (** All dimensions are finite in 0..16384 logical pixels. Width is across
      the scrolling axis; minimum length is along it. Native geometry clamps
      oversized insets/radii/minima to the viewport. Omitted fields inherit. *)
  val create
    :  ?background:Background.t
    -> ?width:float
    -> ?inset:float
    -> ?radius:float
    -> ?min_length:float
    -> unit
    -> t Or_error.t

  val empty : t
end

module Appearance : sig
  type t [@@deriving equal, sexp_of]

  (** State overrides inherit from base, then native defaults. Pressed does
      not inherit hover-only overrides. Empty parts have no explicit fields. *)
  val create
    :  ?track:Track.t
    -> ?track_hover:Track.t
    -> ?track_pressed:Track.t
    -> ?thumb:Thumb.t
    -> ?thumb_hover:Thumb.t
    -> ?thumb_pressed:Thumb.t
    -> unit
    -> t

  val default : t
end

module Motion : sig
  type t [@@deriving equal, sexp_of]

  (** Each duration is 0..60 seconds, rounded up to whole milliseconds.
      Defaults: two-second idle hold, immediate enter/exit/expansion, Fade.
      An idle hold is visibility policy; reduced motion still preserves it. *)
  val create
    :  ?idle:Time_ns.Span.t
    -> ?enter:Time_ns.Span.t
    -> ?exit:Time_ns.Span.t
    -> ?expand:Time_ns.Span.t
    -> ?entrance:Entrance.t
    -> ?thumb_hover_entrance:Entrance.t
    -> unit
    -> t Or_error.t

  val default : t
end

type t [@@deriving equal, sexp_of]

(** [label] names the scroll area: nonblank UTF-8 without NUL, at most 1024
    bytes. Defaults: Both axes, Scrolling visibility, inherited appearance and
    immediate motion. Axis selects bars, not the viewport's scrolling policy;
    an axis without usable overflow has no bar even with Always visibility. *)
val create
  :  label:string
  -> ?axis:Axis.t
  -> ?mode:Mode.t
  -> ?appearance:Appearance.t
  -> ?motion:Motion.t
  -> unit
  -> t Or_error.t

val label : t -> string

module Expert : sig
  (** Resolve every state, including nonvisible parts. Missing theme tokens
      reject the whole description; no native global theme is mutated. *)
  val to_wire : t -> theme:Theme.t -> Gpuio_protocol.Wire.Scrollbar.t Or_error.t
end
