open Core

module Property : sig
  type t =
    | Width
    | Height
    | Top
    | Right
    | Bottom
    | Left
    | Opacity
    | Top_left_radius
    | Top_right_radius
    | Bottom_left_radius
    | Bottom_right_radius
    | Opacity_factor
  [@@deriving bin_io, compare, equal, sexp_of]
end

module Target : sig
  type t =
    { property : Property.t
    ; value : float
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Step_position : sig
  type t =
    | Jump_start
    | Jump_end
    | Jump_none
    | Jump_both
  [@@deriving bin_io, equal, sexp_of]
end

module Easing : sig
  type t =
    | Linear
    | Ease
    | Ease_in
    | Ease_out
    | Ease_in_out
    | Cubic_bezier of float * float * float * float
    | Ease_in_out_cubic
    | Steps of int64 * Step_position.t
    | Linear_stops of (float * float) list
  [@@deriving bin_io, equal, sexp_of]

  val valid_linear_stops : (float * float) list -> bool
  val valid_steps : count:int64 -> position:Step_position.t -> bool
end

module Spring : sig
  type t =
    { stiffness : float
    ; damping : float
    ; mass : float
    ; epsilon : float
    ; max_duration_ms : int64
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Iteration_count : sig
  type t =
    { high : int64
    ; low : int64
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Direction : sig
  type t =
    | Normal
    | Reverse
    | Alternate
    | Alternate_reverse
  [@@deriving bin_io, equal, sexp_of]
end

module Repeat : sig
  type t =
    | Once
    | Loop
    | Alternate
    | Finite of Iteration_count.t * Direction.t
    | Infinite of Direction.t
  [@@deriving bin_io, equal, sexp_of]
end

module Preference : sig
  type t =
    | System
    | Reduce
    | Full
  [@@deriving bin_io, equal, sexp_of]
end

module Cancel_reason : sig
  type t =
    | Replaced
    | Removed
    | Window_closed
  [@@deriving bin_io, equal, sexp_of]
end

module Outcome : sig
  type t =
    | Finished
    | Cancelled of Cancel_reason.t
  [@@deriving bin_io, equal, sexp_of]
end

module Endpoint : sig
  type t =
    { generation : int64
    ; outcome : Outcome.t
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Config : sig
  type t =
    { generation : int64
    ; targets : Target.t list
    ; initial : Target.t list option
    ; duration_ms : int64
    ; delay_ms : int64
    ; easing : Easing.t
    ; repeat : Repeat.t
    }
  [@@deriving bin_io, equal, sexp_of]
end
