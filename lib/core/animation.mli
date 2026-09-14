open Core

(** Declarative motion configuration. Numeric geometry uses logical pixels.
    View/native transport integration is under development in OCH-12. *)
module Property : sig
  type t =
    | Width
    | Height
    | Top
    | Right
    | Bottom
    | Left
    | Opacity
    | Radius
    | Top_left_radius
    | Top_right_radius
    | Bottom_left_radius
    | Bottom_right_radius
  [@@deriving equal, sexp_of]
end

module Target : sig
  type t [@@deriving equal, sexp_of]

  (** Nonempty unique properties. [Radius] expands to all four corners, so combining
      it with a corner is a duplicate. Values must be finite: opacity in [0,1],
      offsets in [-1_000_000,1_000_000], other geometry in [0,1_000_000]. *)
  val create : (Property.t * float) list -> t Or_error.t
end

module Easing : sig
  type t [@@deriving equal, sexp_of]

  val linear : t
  val ease : t
  val ease_in : t
  val ease_out : t
  val ease_in_out : t

  (** X control points in [0,1]; Y control points may be any finite value. Overshoot is permitted,
      with each interpolated property clamped to its valid numeric range. *)
  val cubic_bezier : x1:float -> y1:float -> x2:float -> y2:float -> t Or_error.t
end

module Repeat : sig
  type t =
    | Once
    | Loop
    | Alternate
  [@@deriving equal, sexp_of]
end

module Preference : sig
  type t =
    | System
    | Reduce
    | Full
  [@@deriving equal, sexp_of]
end

module Config : sig
  type t [@@deriving equal, sexp_of]

  (** Default duration 200 ms, no delay, linear timing, once. Durations are between
      zero and one day, rounded up to whole milliseconds. Initial and target must
      name the same properties. With no initial values, the first mount is placed
      immediately; subsequent targets start at the last painted values. Repetition
      requires initial values and a positive duration. Initial values define the
      repeating range; an interrupted first cycle starts at the painted values.

      Explicitly hidden animations pause. Reduced motion settles one-shot runs and
      renders repeated runs at their initial values without requesting frames. *)
  val create
    :  ?initial:Target.t
    -> ?duration:Time_ns.Span.t
    -> ?delay:Time_ns.Span.t
    -> ?easing:Easing.t
    -> ?repeat:Repeat.t
    -> target:Target.t
    -> unit
    -> t Or_error.t
end

module Expert : sig
  val to_wire
    :  Config.t
    -> generation:int64
    -> Gpuio_protocol.Wire.Animation.Config.t Or_error.t
end
