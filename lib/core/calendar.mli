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

module Slot : sig
  (** Identity of one native cell or control. Content must not replace its native
      date/action semantics. Month-qualified headings avoid sharing one retained
      content node between simultaneous panes. *)
  type t [@@deriving compare, equal, sexp_of]

  val previous : t
  val next : t
  val choose_month : t
  val choose_year : t
  val today : t
  val clear : t
  val day : Date.t -> t Or_error.t
  val month : Core.Month.t -> t
  val year : int -> t Or_error.t
  val month_heading : Month.t -> t
  val weekday : Month.t -> day:Day_of_week.t -> t
end

module Appearance : sig
  type t [@@deriving equal, sexp_of]

  (** 1..12 consecutive months, default one. Multiple day panes wrap at a minimum
      width of seven cell heights plus six gaps; one month retains flexible width.
      Cell height is 16..128px, gaps and
      padding 0..64px, radius 0..64px, outline 0..half cell height. Theme colors
      are optional. Presentation changes retain the native selection/focus and
      cursor; snapshot [month] remains the cursor month, not the first pane. *)
  val create
    :  ?months:int
    -> ?cell_height:float
    -> ?cell_gap:float
    -> ?month_gap:float
    -> ?padding:float
    -> ?cell_radius:float
    -> ?outline_width:float
    -> ?selected_background:Color.t
    -> ?selected_foreground:Color.t
    -> ?hover_background:Color.t
    -> ?today_border:Color.t
    -> ?focus_border:Color.t
    -> ?muted_foreground:Color.t
    -> unit
    -> t Or_error.t

  val default : t
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

module Labels : sig
  type t [@@deriving equal, sexp_of]

  (** Gregorian month labels are January-first (12 entries); weekday labels and
      their short visual forms are Sunday-first (7 entries each), independent
      of the configured first weekday. Each label is nonblank valid UTF-8,
      at most 128 bytes, without ASCII controls. These are display strings,
      never format programs. Changing labels does not reinterpret dates. *)
  val create
    :  months:string list
    -> weekdays:string list
    -> short_weekdays:string list
    -> previous:string
    -> next:string
    -> choose_month:string
    -> choose_year:string
    -> today:string
    -> clear:string
    -> unit
    -> t Or_error.t

  val english : t
end

module Config : sig
  type t [@@deriving equal, sexp_of]

  (** Mode defaults to Single and will be immutable per mounted placement.
      Constraints default to unrestricted, first weekday to Monday and labels
      to English. [today] is an optional visual marker supplied by the app;
      no system clock is read. It does not override disabled dates.
      Flags default false. [label] is required nonblank UTF-8 without ASCII
      controls, at most 4096 bytes. Configuration carries no selected value or
      displayed month: retained updates must not silently reset either. *)
  val create
    :  ?mode:Mode.t
    -> ?constraints:Constraints.t
    -> ?first_weekday:Day_of_week.t
    -> ?labels:Labels.t
    -> ?today:Date.t
    -> label:string
    -> ?disabled:bool
    -> ?read_only:bool
    -> ?auto_focus:bool
    -> unit
    -> t Or_error.t

  val mode : t -> Mode.t
  val constraints : t -> Constraints.t
  val first_weekday : t -> Day_of_week.t
  val labels : t -> Labels.t
  val today : t -> Date.t option
  val label : t -> string
  val is_disabled : t -> bool
  val is_read_only : t -> bool
end

module Presentation : sig
  type t =
    | Days
    | Months
    | Years
  [@@deriving equal, sexp_of]
end

module Revision : sig
  type t [@@deriving compare, equal, sexp_of]

  val of_int64 : int64 -> t Or_error.t
  val to_int64 : t -> int64
end

module Viewport : sig
  module Display : sig
    type t = private
      | Days of
          { first_month : Month.t
          ; months : int
          ; first_weekday : Day_of_week.t
          }
      | Months of { year : int }
      | Years of
          { first : int
          ; last : int
          }
    [@@deriving equal, sexp_of]
  end

  (** Logical displayed panes, independent of the keyboard cursor and selection
      revision. These describe a mounted calendar even when hidden, not pixel
      clipping or OS window visibility. Equality includes native owner and
      subscription identity and observation sequence, so an async response can
      be checked against the observation that requested it. *)
  type t [@@deriving equal, sexp_of]

  val display : t -> Display.t

  (** Consecutive day panes, or [] in month/year selection mode. *)
  val months : t -> Month.t list

  (** Sorted unique supported dates in the day panes' six-week grids, including
      neighboring-month cells. At most 504 dates; [] in month/year mode. This is
      a bounded prefetch set, not a selection or permission to select a date. *)
  val dates : t -> Date.t list
end

module Snapshot : sig
  (** Immutable observation bound to one window/node lifetime. Configuration
      may invalidate a historical selection; [selection_allowed] reports this
      without clearing it. The focused date is the keyboard cursor within the
      displayed month, independent of selection and actual native focus. *)
  type t [@@deriving equal, sexp_of]

  val revision : t -> Revision.t
  val mode : t -> Mode.t
  val selection : t -> Selection.t
  val selection_allowed : t -> bool
  val month : t -> Month.t
  val focused_date : t -> Date.t
  val presentation : t -> Presentation.t
  val focused : t -> bool
end

module Event : sig
  (** Observed covers initial/configuration/programmatic changes. Changed covers
      native navigation/focus and partial or complete selection changes. A native
      changed complete selection emits Changed then Selected with consecutive
      revisions. Unchanged clicks and explicit replacements never emit Selected.
      Rejected preserves the previous selection and is an ordered boundary. *)
  type t =
    | Observed of Snapshot.t
    | Changed of Snapshot.t
    | Selected of Snapshot.t
    | Rejected of Selection_error.t * Snapshot.t
  [@@deriving equal, sexp_of]
end

module Command : sig
  (** Replace/Clear optionally guard the current revision and remain available
      read-only/disabled; replacements must fit mode and current constraints.
      Show_month/Move_months preserve selection and clamp the day cursor into the
      target month. With multiple panes, Show_month aligns the first pane when
      the civil boundary permits; Move_months only keeps the cursor visible.
      Focus_date reveals and focuses that civil day, including a
      disabled day for discovery, but must pass native focus/visibility gates.
      Unsupported dates, out-of-domain navigation and offsets outside +/-119987
      fail; no operation wraps across the civil boundary. *)
  type t =
    | Replace of
        { selection : Selection.t
        ; if_revision : Revision.t option
        }
    | Clear of { if_revision : Revision.t option }
    | Show_month of Month.t
    | Move_months of int
    | Focus_date of Date.t
    | Focus
    | Set_presentation of Presentation.t
    | Read_snapshot
  [@@deriving equal, sexp_of]
end

module Command_error : sig
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
  [@@deriving equal, sexp_of]
end

module Expert : sig
  val viewport_of_wire
    :  window:Gpuio_protocol.Window_id.t
    -> node:Gpuio_protocol.Node_id.t
    -> observer:Gpuio_protocol.Handler_id.t
    -> Gpuio_protocol.Calendar_viewport_wire.t
    -> Viewport.t Or_error.t

  val slot_to_wire : Slot.t -> Gpuio_protocol.Calendar_content_wire.Slot.t
  val slot_of_wire : Gpuio_protocol.Calendar_content_wire.Slot.t -> Slot.t Or_error.t
  val slot_key : Slot.t -> string

  val appearance_to_wire
    :  Appearance.t
    -> theme:Theme.t
    -> Gpuio_protocol.Calendar_presentation_wire.t Or_error.t

  (** Bridge representation: days since 0001-01-01, in [0..3652058].
      Validate before conversion; this is not a timestamp or Julian day. *)
  val date_to_ordinal : Date.t -> int64 Or_error.t

  val date_of_ordinal : int64 -> Date.t Or_error.t
  val selection_to_wire : Selection.t -> Gpuio_protocol.Calendar_wire.Selection.t

  val selection_of_wire
    :  Gpuio_protocol.Calendar_wire.Selection.t
    -> Selection.t Or_error.t

  val constraints_to_wire : Constraints.t -> Gpuio_protocol.Calendar_wire.Constraints.t

  val constraints_of_wire
    :  Gpuio_protocol.Calendar_wire.Constraints.t
    -> Constraints.t Or_error.t

  val config_to_wire : Config.t -> Gpuio_protocol.Calendar_wire.Config.t
  val config_of_wire : Gpuio_protocol.Calendar_wire.Config.t -> Config.t Or_error.t
  val month_to_wire : Month.t -> int64
  val month_of_wire : int64 -> Month.t Or_error.t

  val snapshot_of_wire
    :  window:Gpuio_protocol.Window_id.t
    -> node:Gpuio_protocol.Node_id.t
    -> Gpuio_protocol.Calendar_wire.Snapshot.t
    -> Snapshot.t Or_error.t

  val event_of_wire
    :  window:Gpuio_protocol.Window_id.t
    -> node:Gpuio_protocol.Node_id.t
    -> Gpuio_protocol.Calendar_wire.Event.t
    -> Event.t Or_error.t

  val command_to_wire : Command.t -> Gpuio_protocol.Calendar_wire.Command.t Or_error.t
  val error_of_wire : Gpuio_protocol.Calendar_wire.Error.t -> Command_error.t
  val window : Snapshot.t -> Gpuio_protocol.Window_id.t
  val node : Snapshot.t -> Gpuio_protocol.Node_id.t
end
