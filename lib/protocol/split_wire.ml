open Core

module Axis = struct
  type t =
    | Horizontal
    | Vertical
  [@@deriving bin_io, equal, sexp_of]
end

module Config = struct
  type t =
    { label : string
    ; axis : Axis.t
    ; initial_first : float
    ; minimum_first : float
    ; maximum_first : float
    ; minimum_second : float
    ; keyboard_step : float
    ; reset_generation : int64
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    (not (String.is_empty (String.strip t.label)))
    && String.length t.label <= 4096
    && Stdlib.String.is_valid_utf_8 t.label
    && (not (String.contains t.label '\000'))
    && List.for_all
         [ t.initial_first
         ; t.minimum_first
         ; t.maximum_first
         ; t.minimum_second
         ; t.keyboard_step
         ]
         ~f:Float.is_finite
    && Float.(
         t.minimum_first >= 0.
         && t.minimum_first <= t.initial_first
         && t.initial_first <= t.maximum_first
         && t.maximum_first <= 16384.
         && t.minimum_second >= 0.
         && t.minimum_second <= 16384.
         && t.keyboard_step > 0.
         && t.keyboard_step <= 16384.)
    && Int64.(t.reset_generation >= 0L)
  ;;
end

module Snapshot = struct
  type t =
    { first : float
    ; second : float
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Float.is_finite t.first
    && Float.is_finite t.second
    && Float.(t.first >= 0. && t.second >= 0.)
  ;;
end
