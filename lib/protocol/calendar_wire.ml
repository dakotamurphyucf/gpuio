open Core

let max_config_bytes = 24576
let valid_date date = Int64.(date >= 0L && date <= 3652058L)
let valid_month month = Int64.(month >= 0L && month < 119988L)
let valid_weekday weekday = Int64.(weekday >= 0L && weekday <= 6L)

module Mode = struct
  type t =
    | Single
    | Range
  [@@deriving bin_io, equal, sexp_of]
end

module Range = struct
  type t =
    { first : int64
    ; last : int64
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t = valid_date t.first && valid_date t.last && Int64.(t.first <= t.last)
end

module Selection = struct
  type t =
    | Empty
    | Single of int64
    | Range_start of int64
    | Range of Range.t
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Empty -> true
    | Single date | Range_start date -> valid_date date
    | Range range -> Range.valid range
  ;;

  let fits t ~mode =
    match t, mode with
    | Empty, _ | Single _, Mode.Single | (Range_start _ | Range _), Mode.Range -> true
    | Single _, Mode.Range | (Range_start _ | Range _), Mode.Single -> false
  ;;
end

module Range_policy = struct
  type t =
    | Every_day
    | Endpoints_only
  [@@deriving bin_io, equal, sexp_of]
end

module Constraints = struct
  type t =
    { min : int64
    ; max : int64
    ; disabled_dates : int64 list
    ; disabled_ranges : Range.t list
    ; disabled_weekdays : int64 list
    ; range_policy : Range_policy.t
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    valid_date t.min
    && valid_date t.max
    && Int64.(t.min <= t.max)
    && List.length t.disabled_dates <= 512
    && List.for_all t.disabled_dates ~f:valid_date
    && List.length t.disabled_ranges <= 128
    && List.for_all t.disabled_ranges ~f:Range.valid
    && List.length t.disabled_weekdays <= 7
    && List.for_all t.disabled_weekdays ~f:valid_weekday
  ;;
end

let valid_label text ~maximum =
  String.length text <= maximum
  && Stdlib.String.is_valid_utf_8 text
  && (not
        (String.exists text ~f:(fun char ->
           Char.to_int char < 32 || Char.to_int char = 127)))
  && not (String.is_empty (String.strip text ~drop:(Char.equal ' ')))
;;

module Labels = struct
  type t =
    { months : string list
    ; weekdays : string list
    ; short_weekdays : string list
    ; previous : string
    ; next : string
    ; choose_month : string
    ; choose_year : string
    ; today : string
    ; clear : string
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    let labels count values =
      List.length values = count && List.for_all values ~f:(valid_label ~maximum:128)
    in
    labels 12 t.months
    && labels 7 t.weekdays
    && labels 7 t.short_weekdays
    && List.for_all
         [ t.previous; t.next; t.choose_month; t.choose_year; t.today; t.clear ]
         ~f:(valid_label ~maximum:128)
  ;;
end

module Config = struct
  type t =
    { mode : Mode.t
    ; constraints : Constraints.t
    ; first_weekday : int64
    ; labels : Labels.t
    ; today : int64 option
    ; label : string
    ; disabled : bool
    ; read_only : bool
    ; auto_focus : bool
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Constraints.valid t.constraints
    && valid_weekday t.first_weekday
    && Labels.valid t.labels
    && Option.for_all t.today ~f:valid_date
    && valid_label t.label ~maximum:4096
  ;;
end

module Presentation = struct
  type t =
    | Days
    | Months
    | Years
  [@@deriving bin_io, equal, sexp_of]
end

module Snapshot = struct
  type t =
    { revision : int64
    ; mode : Mode.t
    ; selection : Selection.t
    ; selection_allowed : bool
    ; month : int64
    ; focused_date : int64
    ; presentation : Presentation.t
    ; focused : bool
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Int64.(t.revision >= 0L)
    && Selection.valid t.selection
    && Selection.fits t.selection ~mode:t.mode
    && (match t.selection with
        | Empty -> t.selection_allowed
        | Single _ | Range_start _ | Range _ -> true)
    && valid_month t.month
    && valid_date t.focused_date
    &&
    let date =
      Date.add_days (Date.create_exn ~y:1 ~m:Jan ~d:1) (Int64.to_int_exn t.focused_date)
    in
    Int64.equal
      t.month
      (Int64.of_int (((Date.year date - 1) * 12) + Month.to_int (Date.month date) - 1))
  ;;
end

module Selection_error = struct
  type t =
    | Wrong_mode
    | Unsupported_date
    | Disabled_date
    | Disabled_interior
  [@@deriving bin_io, equal, sexp_of]
end

module Event = struct
  type t =
    | Observed of Snapshot.t
    | Changed of Snapshot.t
    | Selected of Snapshot.t
    | Rejected of Selection_error.t * Snapshot.t
  [@@deriving bin_io, equal, sexp_of]

  let snapshot = function
    | Observed s | Changed s | Selected s | Rejected (_, s) -> s
  ;;

  let valid t =
    let s = snapshot t in
    Snapshot.valid s
    &&
    match t with
    | Observed _ -> true
    | Changed _ | Rejected _ -> Int64.(s.revision > 0L)
    | Selected _ ->
      Int64.(s.revision > 0L)
      && s.selection_allowed
      &&
        (match s.selection with
        | Single _ | Range _ -> true
        | Empty | Range_start _ -> false)
  ;;
end

module Command = struct
  type t =
    | Replace of
        { selection : Selection.t
        ; if_revision : int64 option
        }
    | Clear of { if_revision : int64 option }
    | Show_month of int64
    | Move_months of int64
    | Focus_date of int64
    | Focus
    | Set_presentation of Presentation.t
    | Read_snapshot
  [@@deriving bin_io, equal, sexp_of]

  let valid_revision revision = Option.for_all revision ~f:(fun r -> Int64.(r >= 0L))

  let valid = function
    | Replace { selection; if_revision } ->
      Selection.valid selection && valid_revision if_revision
    | Clear { if_revision } -> valid_revision if_revision
    | Show_month month -> valid_month month
    | Move_months delta -> Int64.(delta >= -119987L && delta <= 119987L)
    | Focus_date date -> valid_date date
    | Focus | Set_presentation _ | Read_snapshot -> true
  ;;
end

module Error = struct
  type t =
    | Not_mounted
    | Closed
    | Stale_input
    | Stale_revision
    | Limit_exceeded
    | Busy
    | Native_failure
    | Invalid_config
    | Wrong_mode
    | Disabled_date
    | Disabled_interior
    | Focus_blocked
    | Disabled
    | Read_only
    | Invalid_value
  [@@deriving bin_io, equal, sexp_of]
end

module Response = struct
  type t =
    | Applied of Snapshot.t
    | Failed of Error.t
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Applied snapshot -> Snapshot.valid snapshot
    | Failed _ -> true
  ;;
end
