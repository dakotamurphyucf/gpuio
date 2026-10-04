open Core

module Display = struct
  type t =
    | Days of
        { first_month : int64
        ; months : int64
        ; first_weekday : int64
        }
    | Months of { year : int64 }
    | Years of
        { first : int64
        ; last : int64
        }
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Days { first_month; months; first_weekday } ->
      Calendar_wire.valid_month first_month
      && Int64.(months >= 1L && months <= 12L && first_month <= 119988L - months)
      && Int64.(first_weekday >= 0L && first_weekday <= 6L)
    | Months { year } -> Int64.(year >= 1L && year <= 9999L)
    | Years { first; last } ->
      Int64.(
        first >= 1L
        && first <= 9999L
        && rem (first - 1L) 20L = 0L
        && last = min 9999L (first + 19L))
  ;;
end

type t =
  { sequence : int64
  ; display : Display.t
  }
[@@deriving bin_io, equal, sexp_of]

let valid t = Int64.(t.sequence >= 0L) && Display.valid t.display
