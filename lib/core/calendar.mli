open Core

(** Civil Gregorian dates only, with no time-zone or ambient-clock conversion.
    The model is shared by inline calendars and popup date pickers. *)
val min_date : Date.t

val max_date : Date.t
val is_supported_date : Date.t -> bool

module Month : sig
  type t [@@deriving compare, equal, sexp_of]

  val create : year:int -> month:Month.t -> t Or_error.t
  val of_date : Date.t -> t Or_error.t
  val year : t -> int
  val month : t -> Month.t
  val first_day : t -> Date.t
  val last_day : t -> Date.t

  (** Checked offset; fails rather than wrapping or saturating at civil limits.
      Even an extreme [months] argument is validated before arithmetic. *)
  val shift : t -> months:int -> t Or_error.t

  (** Exactly six weeks of seven days; includes neighboring-month dates.
      Cells outside supported years are [None]. Work is always bounded by 42. *)
  val weeks : t -> first_weekday:Day_of_week.t -> Date.t option list list
end

module Range : sig
  type t [@@deriving equal, sexp_of]

  (** Inclusive ordered endpoints in the supported civil domain. *)
  val create : first:Date.t -> last:Date.t -> t Or_error.t

  val first : t -> Date.t
  val last : t -> Date.t
  val contains : t -> Date.t -> bool
end

module Mode : sig
  type t =
    | Single
    | Range
  [@@deriving equal, sexp_of]
end

module Selection : sig
  type t = private
    | Empty
    | Single of Date.t
    | Range_start of Date.t
    | Range of Range.t
  [@@deriving equal, sexp_of]

  val empty : t
  val single : Date.t -> t Or_error.t
  val range_start : Date.t -> t Or_error.t
  val range : Range.t -> t
  val fits : t -> mode:Mode.t -> bool
  val is_complete : t -> bool
end

module Range_policy : sig
  type t =
    | Every_day
    | Endpoints_only
  [@@deriving equal, sexp_of]
end

module Constraints : sig
  type t [@@deriving equal, sexp_of]

  (** Inclusive bounds default to the complete civil domain; range policy
      defaults to [Every_day]. Input bounds apply before deduplication:
      at most 512 dates, 128 ranges, 7 weekdays. Intervals merge when adjacent
      or overlapping. A policy disabling every date is valid. *)
  val create
    :  ?min:Date.t
    -> ?max:Date.t
    -> ?disabled_dates:Date.t list
    -> ?disabled_ranges:Range.t list
    -> ?disabled_weekdays:Day_of_week.t list
    -> ?range_policy:Range_policy.t
    -> unit
    -> t Or_error.t

  val unrestricted : t
  val min : t -> Date.t
  val max : t -> Date.t
  val disabled_dates : t -> Date.t list
  val disabled_ranges : t -> Range.t list
  val disabled_weekdays : t -> Day_of_week.t list
  val range_policy : t -> Range_policy.t
  val allows : t -> Date.t -> bool

  (** Bounded by constraint count, never by the number of days in a range.
      Empty is allowed in either mode. Checks historical values against the
      current policy without mutating them. *)
  val allows_selection : t -> Selection.t -> mode:Mode.t -> bool
end

module Selection_error : sig
  type t =
    | Wrong_mode
    | Unsupported_date
    | Disabled_date
    | Disabled_interior
  [@@deriving equal, sexp_of]
end

(** Pure native-selection reference: first click starts a range; an earlier
    second click restarts it; an equal/later click completes it. A rejected
    activation leaves the caller's previous selection intact. Old selections
    may be disallowed by newer constraints; starting a new one remains possible. *)
val activate
  :  Selection.t
  -> date:Date.t
  -> mode:Mode.t
  -> constraints:Constraints.t
  -> (Selection.t, Selection_error.t) Result.t

module Format : sig
  type t =
    | Iso
    | Day_month_year
    | Month_day_year
  [@@deriving equal, sexp_of]

  (** [Iso] uses YYYY-MM-DD; other modes use DD/MM/YYYY or MM/DD/YYYY.
      Exactly ten ASCII bytes, zero-padded. No whitespace or heuristic parsing. *)
  val format : t -> Date.t -> string Or_error.t

  val parse : t -> string -> Date.t Or_error.t
end

module Expert : sig
  (** Bridge representation: days since 0001-01-01, in [0..3652058].
      Validate before conversion; this is not a timestamp or Julian day. *)
  val date_to_ordinal : Date.t -> int64 Or_error.t

  val date_of_ordinal : int64 -> Date.t Or_error.t
end
