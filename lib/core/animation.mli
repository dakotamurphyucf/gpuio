open Core

(** Declarative motion configuration. Numeric geometry uses logical pixels.
    Native frames do not call OCaml timing functions. *)
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
    | Opacity_factor
  [@@deriving equal, sexp_of]

  (** [Opacity] replaces the element's style opacity. [Opacity_factor] multiplies
      its resolved base/interaction opacity, including background and children,
      without adding a layout wrapper. The factor is in [0,1]; 1 preserves the
      original style. A target cannot contain both opacity properties. Ancestor
      opacity still multiplies normally. *)
end

module Target : sig
  type t [@@deriving equal, sexp_of]

  (** Nonempty unique properties. [Radius] expands to all four corners, so combining
      it with a corner is a duplicate. Values must be finite: opacity in [0,1],
      offsets in [-1_000_000,1_000_000], other geometry in [0,1_000_000]. *)
  val create : (Property.t * float) list -> t Or_error.t
end

module Easing : sig
  module Linear_stop : sig
    type t [@@deriving equal, sexp_of]

    (** A finite output with an optional input position in [0,1]. *)
    val create : ?input:float -> output:float -> unit -> t Or_error.t
  end

  module Step_position : sig
    type t =
      | Jump_start
      | Jump_end
      | Jump_none
      | Jump_both
    [@@deriving equal, sexp_of]
  end

  type t [@@deriving equal, sexp_of]

  val linear : t
  val ease : t
  val ease_in : t
  val ease_out : t
  val ease_in_out : t

  (** Polynomial curves [t^3] and [1 - (1 - t)^3], respectively. These differ
      from the CSS [ease_in]/[ease_out] presets. They use the existing native
      cubic-Bezier representation with linear x; no OCaml frame callback or
      protocol extension is needed. *)
  val ease_in_cubic : t

  val ease_out_cubic : t

  (** Piecewise polynomial: [4 * t^3] up to the midpoint, then
      [1 - 4 * (1 - t)^3]. Native evaluation preserves the exact midpoint and
      endpoints. This differs from the CSS [ease_in_out] preset. *)
  val ease_in_out_cubic : t

  (** Constant-time stepped motion. [count] is in [1,4294967295];
      [Jump_none] requires at least 2. At progress zero, [Jump_start] produces
      [1/count] and [Jump_both] produces [1/(count+1)]; the other modes produce
      zero. Each jump boundary selects the following step. Progress outside
      [0,1] clamps before evaluation; all modes produce 1 at progress 1. *)
  val steps : count:int -> position:Step_position.t -> t Or_error.t

  (** Piecewise-linear easing with 2..256 stops. Omitted endpoint positions
      default to 0 and 1; omitted interior positions are spaced evenly between
      anchors. Explicit positions must be nondecreasing. At a duplicate position,
      the last stop wins. Outside the first/last positions, output holds constant.
      Outputs may overshoot; native properties retain their own clamps. Finite
      animation completion still settles at the declared target. *)
  val linear_stops : Linear_stop.t list -> t Or_error.t

  (** X control points in [0,1]; Y control points may be any finite value. Overshoot is permitted,
      with each interpolated property clamped to its valid numeric range. *)
  val cubic_bezier : x1:float -> y1:float -> x2:float -> y2:float -> t Or_error.t
end

module Spring : sig
  type t [@@deriving equal, sexp_of]

  (** Physical parameters: stiffness in [0.01,10_000], damping in [0,1_000],
      mass in [0.01,1_000]. All values must be finite. [epsilon] defaults to
      0.001 animated units and is in [0.0001,1]. Velocity tolerance is epsilon
      times the natural frequency. [max_duration] defaults to 10 seconds,
      is positive, rounds up to milliseconds and is at most 60 seconds.
      At that deadline the spring settles exactly at its target, including
      undamped springs. Native animation programs evaluate springs without
      calling OCaml on each frame. *)
  val create
    :  ?epsilon:float
    -> ?max_duration:Time_ns.Span.t
    -> stiffness:float
    -> damping:float
    -> mass:float
    -> unit
    -> t Or_error.t
end

module Repeat : sig
  type t =
    | Once
    | Loop
    | Alternate
  [@@deriving equal, sexp_of]
end

module Timing : sig
  type t [@@deriving equal, sexp_of]

  val tween : ?easing:Easing.t -> Time_ns.Span.t -> t Or_error.t
  val spring : Spring.t -> t
end

module Stage : sig
  type t [@@deriving equal, sexp_of]

  (** Delay before this stage, on each cycle. Default zero; at most one day. *)
  val create
    :  ?delay:Time_ns.Span.t
    -> timing:Timing.t
    -> target:Target.t
    -> unit
    -> t Or_error.t
end

module Clock : sig
  type t [@@deriving equal, sexp_of]

  val independent : t
  val application : t

  (** Application-scoped name, 1..128 UTF-8 bytes without NUL. *)
  val group : string -> t Or_error.t
end

module Playback : sig
  type t =
    | Running
    | Paused
    | Cancelled
  [@@deriving equal, sexp_of]
end

(** Run numbers are scoped to one retained animated wrapper, not globally unique. *)
module Run_id : sig
  type t [@@deriving equal, compare, sexp_of]

  val to_int64 : t -> int64
end

module Program : sig
  module Stage_result : sig
    type t =
      | Played
      | Reduced_motion
    [@@deriving equal, sexp_of]
  end

  module Cancel_reason : sig
    type t =
      | Replaced
      | Removed
      | Window_closed
      | Requested
    [@@deriving equal, sexp_of]
  end

  module Observation : sig
    (** Stage indices are zero-based. [Played] records active time crossing the
        stage boundary, confirmed by paint; it does not promise a distinct frame
        showing every intermediate stage. *)
    type t =
      | Stage_completed of int * Stage_result.t
      | Finished
      | Cancelled of Cancel_reason.t
    [@@deriving equal, sexp_of]
  end

  module Event : sig
    (** One ordered, nonempty batch (at most 33 observations) from one native run.
        Repeats do not emit stage/cycle events. No callback runs for each frame. *)
    type t = private
      { run_id : Run_id.t
      ; observations : Observation.t list
      }
    [@@deriving equal, sexp_of]
  end

  type t [@@deriving equal, sexp_of]

  (** One to 32 stages with the same properties. Multi-stage programs and repeats
      require initial values. The initial delay is applied once; stage delays apply
      each cycle. The sum of stage maximum durations and delays is at most one day;
      the separate initial delay is also at most one day. Repeats need positive cycle duration.
      Shared clocks require repetition, timed stages, explicit initial values and
      zero initial delay. [clock] defaults to independent, [repeat] to once.

      The complete encoding is bounded to 16,384 bytes, reserving the maximum
      generation/restart encodings; large easing curves count toward that limit.
      Use [View.animate_program] to mount a retained native program. *)
  val create
    :  ?initial:Target.t
    -> ?delay:Time_ns.Span.t
    -> ?repeat:Repeat.t
    -> ?clock:Clock.t
    -> Stage.t list
    -> t Or_error.t

  (** Playback changes preserve the run; cancellation holds its last painted value
      and is terminal until a new program or restart. *)
  val with_playback : t -> Playback.t -> t

  (** Increment the restart token, resetting playback to Running. Retain the returned
      value for subsequent restarts. Exhaustion returns an error. *)
  val restart : t -> t Or_error.t

  (** Reverse the declared intervals, retaining each interval's timing/delay.
      Requires initial values. This is a new program, not a reversal of past events. *)
  val reverse : t -> t Or_error.t
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

module Cancel_reason : sig
  type t =
    | Replaced
    | Removed
    | Window_closed
  [@@deriving equal, sexp_of]
end

module Outcome : sig
  type t =
    | Finished
    | Cancelled of Cancel_reason.t
  [@@deriving equal, sexp_of]
end

module Event : sig
  type t = private
    { run_id : Run_id.t
    ; outcome : Outcome.t
    }
  [@@deriving equal, sexp_of]
end

module Expert : sig
  val easing_to_wire : Easing.t -> Gpuio_protocol.Wire.Animation.Easing.t

  val program_event_of_wire
    :  Gpuio_protocol.Wire.Animation_program.Signal.t list
    -> Program.Event.t Or_error.t

  val spring_to_wire : Spring.t -> Gpuio_protocol.Wire.Animation.Spring.t

  val program_to_wire
    :  Program.t
    -> generation:int64
    -> Gpuio_protocol.Wire.Animation_program.Config.t Or_error.t

  val event_of_wire : Gpuio_protocol.Wire.Animation.Endpoint.t -> Event.t Or_error.t

  val to_wire
    :  Config.t
    -> generation:int64
    -> Gpuio_protocol.Wire.Animation.Config.t Or_error.t
end
