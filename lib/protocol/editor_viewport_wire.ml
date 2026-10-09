open Core

let valid_pixels n = Float.is_finite n && Float.(n >= 0. && n <= 1e9)

module Offset = struct
  type t =
    { x : float
    ; y : float
    }
  [@@deriving bin_io, equal, sexp_of]

  let is_valid t = valid_pixels t.x && valid_pixels t.y
end

type t =
  { offset : Offset.t
  ; width : float
  ; height : float
  ; line_height : float
  ; first_buffer_line : int64
  ; buffer_line_limit : int64
  }
[@@deriving bin_io, equal, sexp_of]

let is_valid t =
  Offset.is_valid t.offset
  && valid_pixels t.width
  && valid_pixels t.height
  && valid_pixels t.line_height
  && Float.(t.line_height > 0.)
  && Int64.(
       t.first_buffer_line >= 0L
       && t.buffer_line_limit >= t.first_buffer_line
       && t.buffer_line_limit <= 262_145L)
;;
