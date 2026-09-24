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
  [@@deriving bin_io, compare, equal, sexp_of]
end

module Target = struct
  type t =
    { property : Property.t
    ; value : float
    }
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
  [@@deriving bin_io, equal, sexp_of]
end

module Repeat = struct
  type t =
    | Once
    | Loop
    | Alternate
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
