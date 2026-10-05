open Core

module Property = struct
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

module Target = struct
  type t =
    { property : Property.t
    ; value : float
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Step_position = struct
  type t =
    | Jump_start
    | Jump_end
    | Jump_none
    | Jump_both
  [@@deriving bin_io, equal, sexp_of]
end

module Easing = struct
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

  let valid_linear_stops stops =
    let length = List.length stops in
    length >= 2
    && length <= 256
    && List.for_all stops ~f:(fun (input, output) ->
      Float.is_finite input
      && Float.(input >= 0. && input <= 1.)
      && Float.is_finite output)
    && List.is_sorted stops ~compare:(fun (a, _) (b, _) -> Float.compare a b)
  ;;

  let valid_steps ~count ~position =
    Int64.(count >= 1L && count <= 4_294_967_295L)
    && ((not (Step_position.equal position Jump_none)) || Int64.(count >= 2L))
  ;;
end

module Spring = struct
  type t =
    { stiffness : float
    ; damping : float
    ; mass : float
    ; epsilon : float
    ; max_duration_ms : int64
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Iteration_count = struct
  type t =
    { high : int64
    ; low : int64
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Direction = struct
  type t =
    | Normal
    | Reverse
    | Alternate
    | Alternate_reverse
  [@@deriving bin_io, equal, sexp_of]
end

module Repeat = struct
  type t =
    | Once
    | Loop
    | Alternate
    | Finite of Iteration_count.t * Direction.t
    | Infinite of Direction.t
  [@@deriving bin_io, equal, sexp_of]
end

module Preference = struct
  type t =
    | System
    | Reduce
    | Full
  [@@deriving bin_io, equal, sexp_of]
end

module Cancel_reason = struct
  type t =
    | Replaced
    | Removed
    | Window_closed
  [@@deriving bin_io, equal, sexp_of]
end

module Outcome = struct
  type t =
    | Finished
    | Cancelled of Cancel_reason.t
  [@@deriving bin_io, equal, sexp_of]
end

module Endpoint = struct
  type t =
    { generation : int64
    ; outcome : Outcome.t
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Config = struct
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
