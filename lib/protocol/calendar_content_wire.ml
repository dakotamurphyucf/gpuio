open Core

module Slot = struct
  type t =
    | Previous
    | Next
    | Choose_month
    | Choose_year
    | Today
    | Clear
    | Day of int64
    | Month of int64
    | Year of int64
    | Month_heading of int64
    | Weekday of int64 * int64
  [@@deriving bin_io, compare, equal, sexp_of]

  let valid = function
    | Previous | Next | Choose_month | Choose_year | Today | Clear -> true
    | Day date -> Calendar_wire.valid_date date
    | Month month -> Int64.(month >= 1L && month <= 12L)
    | Year year -> Int64.(year >= 1L && year <= 9999L)
    | Month_heading month -> Calendar_wire.valid_month month
    | Weekday (month, weekday) ->
      Calendar_wire.valid_month month && Calendar_wire.valid_weekday weekday
  ;;

  let key = function
    | Previous -> "previous"
    | Next -> "next"
    | Choose_month -> "choose-month"
    | Choose_year -> "choose-year"
    | Today -> "today"
    | Clear -> "clear"
    | Day date -> sprintf "day-%Ld" date
    | Month month -> sprintf "month-%Ld" month
    | Year year -> sprintf "year-%Ld" year
    | Month_heading month -> sprintf "heading-%Ld" month
    | Weekday (month, weekday) -> sprintf "weekday-%Ld-%Ld" month weekday
  ;;
end

module Item = struct
  type t =
    { slot : Slot.t
    ; description : string option
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Slot.valid t.slot
    && Option.for_all t.description ~f:(Calendar_wire.valid_label ~maximum:1024)
  ;;
end

type t = Item.t list [@@deriving bin_io, equal, sexp_of]

let max_items = 1024
let max_description_bytes = 65536

let valid items =
  let rec ordered = function
    | (a : Item.t) :: (b :: _ as rest) -> Slot.compare a.slot b.slot < 0 && ordered rest
    | [] | [ _ ] -> true
  in
  List.length items <= max_items
  && List.for_all items ~f:Item.valid
  && ordered items
  && List.sum
       (module Int)
       items
       ~f:(fun item -> Option.value_map item.Item.description ~default:0 ~f:String.length)
     <= max_description_bytes
;;
