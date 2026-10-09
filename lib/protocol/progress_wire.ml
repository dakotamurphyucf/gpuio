open Core

(** Existing progress payload. Keep its field order unchanged. *)
type t =
  { label : string
  ; fraction : float option
  }
[@@deriving bin_io, equal, sexp_of]

module Shape = struct
  type t =
    | Linear
    | Circle
  [@@deriving bin_io, equal, sexp_of]
end

module Transition = struct
  type t =
    | Immediate
    | Tween of
        { duration_ms : int
        ; easing : Animation_wire.Easing.t
        }
  [@@deriving bin_io, equal, sexp_of]
end

module Presentation = struct
  type progress = t [@@deriving bin_io, equal, sexp_of]

  type t =
    { progress : progress
    ; shape : Shape.t
    ; transition : Transition.t
    }
  [@@deriving bin_io, equal, sexp_of]
end
