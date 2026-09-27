open Core
module W = Gpuio_protocol.Calendar_wire

let min_date = Date.create_exn ~y:1 ~m:Jan ~d:1
let max_date = Date.create_exn ~y:9999 ~m:Dec ~d:31
let is_supported_date date = Date.(date >= min_date && date <= max_date)

let validate_date date =
  if is_supported_date date
  then Ok date
  else Or_error.error_string "calendar date must be within years 1..9999"
;;

module Month = struct
  type t =
    { year : int
    ; month : Core.Month.t
    }
  [@@deriving compare, equal, sexp_of]

  let create ~year ~month =
    if year >= 1 && year <= 9999
    then Ok { year; month }
    else Or_error.error_string "calendar month must be within years 1..9999"
  ;;

  let of_date date = create ~year:(Date.year date) ~month:(Date.month date)
  let year t = t.year
  let month t = t.month
  let first_day t = Date.create_exn ~y:t.year ~m:t.month ~d:1

  let last_day t =
    Date.create_exn
      ~y:t.year
      ~m:t.month
      ~d:(Date.days_in_month ~year:t.year ~month:t.month)
  ;;

  let shift t ~months =
    let index = ((t.year - 1) * 12) + Core.Month.to_int t.month - 1 in
    if months < -index || months > (9999 * 12) - 1 - index
    then Or_error.error_string "calendar month navigation exceeds years 1..9999"
    else (
      let index = index + months in
      create ~year:((index / 12) + 1) ~month:(Core.Month.of_int_exn ((index % 12) + 1)))
  ;;

  let weeks t ~first_weekday =
    let first = first_day t in
    let offset = Day_of_week.num_days ~from:first_weekday ~to_:(Date.day_of_week first) in
    let ordinal = Date.diff first min_date in
    let last = Date.diff max_date min_date in
    List.init 6 ~f:(fun week ->
      List.init 7 ~f:(fun day ->
        let delta = (week * 7) + day - offset in
        let ordinal = ordinal + delta in
        if ordinal < 0 || ordinal > last then None else Some (Date.add_days first delta)))
  ;;
end

module Range = struct
  type t =
    { first : Date.t
    ; last : Date.t
    }
  [@@deriving equal, sexp_of]

  let create ~first ~last =
    if not (is_supported_date first && is_supported_date last)
    then Or_error.error_string "calendar range must be within years 1..9999"
    else if Date.(first > last)
    then Or_error.error_string "calendar range endpoints must be ordered"
    else Ok { first; last }
  ;;

  let first t = t.first
  let last t = t.last
  let contains t date = Date.(t.first <= date && date <= t.last)
end

module Mode = struct
  type t = W.Mode.t =
    | Single
    | Range
  [@@deriving equal, sexp_of]
end

module Selection = struct
  type t =
    | Empty
    | Single of Date.t
    | Range_start of Date.t
    | Range of Range.t
  [@@deriving equal, sexp_of]

  let empty = Empty
  let single date = Or_error.map (validate_date date) ~f:(fun date -> Single date)

  let range_start date =
    Or_error.map (validate_date date) ~f:(fun date -> Range_start date)
  ;;

  let range t = Range t

  let fits t ~mode =
    match t, mode with
    | Empty, _ | Single _, Mode.Single | (Range_start _ | Range _), Mode.Range -> true
    | Single _, Mode.Range | (Range_start _ | Range _), Mode.Single -> false
  ;;

  let is_complete = function
    | Single _ | Range _ -> true
    | Empty | Range_start _ -> false
  ;;
end

module Range_policy = struct
  type t = W.Range_policy.t =
    | Every_day
    | Endpoints_only
  [@@deriving equal, sexp_of]
end

module Constraints = struct
  type t =
    { min : Date.t
    ; max : Date.t
    ; disabled_dates : Date.t list
    ; disabled_ranges : Range.t list
    ; disabled_weekdays : Day_of_week.t list
    ; range_policy : Range_policy.t
    }
  [@@deriving equal, sexp_of]

  let merge_ranges ranges =
    let sorted =
      List.sort ranges ~compare:(fun a b -> Date.compare a.Range.first b.first)
    in
    List.fold sorted ~init:[] ~f:(fun acc next ->
      match acc with
      | previous :: rest when Date.diff next.Range.first previous.Range.last <= 1 ->
        { previous with last = Date.max previous.last next.last } :: rest
      | [] | _ :: _ -> next :: acc)
    |> List.rev
  ;;

  let create
        ?(min = min_date)
        ?(max = max_date)
        ?(disabled_dates = [])
        ?(disabled_ranges = [])
        ?(disabled_weekdays = [])
        ?(range_policy = Range_policy.Every_day)
        ()
    =
    if
      List.length disabled_dates > 512
      || List.length disabled_ranges > 128
      || List.length disabled_weekdays > 7
    then
      Or_error.error_string
        "calendar constraints exceed 512 dates, 128 ranges or 7 weekdays"
    else if
      (not (is_supported_date min && is_supported_date max)) || Date.compare min max > 0
    then Or_error.error_string "calendar bounds must be ordered within years 1..9999"
    else if not (List.for_all disabled_dates ~f:is_supported_date)
    then Or_error.error_string "disabled calendar dates must be within years 1..9999"
    else
      Ok
        { min
        ; max
        ; disabled_dates = List.dedup_and_sort disabled_dates ~compare:Date.compare
        ; disabled_ranges = merge_ranges disabled_ranges
        ; disabled_weekdays =
            List.dedup_and_sort disabled_weekdays ~compare:Day_of_week.compare
        ; range_policy
        }
  ;;

  let unrestricted = create () |> Or_error.ok_exn
  let min t = t.min
  let max t = t.max
  let disabled_dates t = t.disabled_dates
  let disabled_ranges t = t.disabled_ranges
  let disabled_weekdays t = t.disabled_weekdays
  let range_policy t = t.range_policy

  let allows t date =
    Date.(date >= t.min && date <= t.max)
    && (not (List.mem t.disabled_dates date ~equal:Date.equal))
    && (not (List.exists t.disabled_ranges ~f:(fun range -> Range.contains range date)))
    && not (List.mem t.disabled_weekdays (Date.day_of_week date) ~equal:Day_of_week.equal)
  ;;

  let allows_range t range =
    allows t range.Range.first
    && allows t range.last
    &&
    match t.range_policy with
    | Range_policy.Endpoints_only -> true
    | Every_day ->
      (not (List.exists t.disabled_dates ~f:(Range.contains range)))
      && (not
            (List.exists t.disabled_ranges ~f:(fun disabled ->
               Date.(disabled.Range.first <= range.last && disabled.last >= range.first))))
      && not
           (List.exists t.disabled_weekdays ~f:(fun weekday ->
              Day_of_week.num_days ~from:(Date.day_of_week range.first) ~to_:weekday
              <= Date.diff range.last range.first))
  ;;

  let allows_selection t selection ~mode =
    Selection.fits selection ~mode
    &&
    match selection with
    | Selection.Empty -> true
    | Single date | Range_start date -> allows t date
    | Range range -> allows_range t range
  ;;
end

module Selection_error = struct
  type t = W.Selection_error.t =
    | Wrong_mode
    | Unsupported_date
    | Disabled_date
    | Disabled_interior
  [@@deriving equal, sexp_of]
end

let activate selection ~date ~mode ~constraints =
  if not (Selection.fits selection ~mode)
  then Error Selection_error.Wrong_mode
  else if not (is_supported_date date)
  then Error Unsupported_date
  else if not (Constraints.allows constraints date)
  then Error Disabled_date
  else (
    let next =
      match mode, selection with
      | Mode.Single, _ -> Selection.Single date
      | Range, Selection.Range_start first when Date.(date >= first) ->
        Selection.Range { Range.first; last = date }
      | Range, (Empty | Single _ | Range_start _ | Range _) -> Selection.Range_start date
    in
    if Constraints.allows_selection constraints next ~mode
    then Ok next
    else (
      match next with
      | Range range when not (Constraints.allows constraints range.Range.first) ->
        Error Disabled_date
      | Empty | Single _ | Range_start _ | Range _ -> Error Disabled_interior))
;;

module Format = struct
  type t =
    | Iso
    | Day_month_year
    | Month_day_year
  [@@deriving equal, sexp_of]

  let format t date =
    Or_error.map (validate_date date) ~f:(fun date ->
      let year = Date.year date
      and month = Core.Month.to_int (Date.month date)
      and day = Date.day date in
      match t with
      | Iso -> sprintf "%04d-%02d-%02d" year month day
      | Day_month_year -> sprintf "%02d/%02d/%04d" day month year
      | Month_day_year -> sprintf "%02d/%02d/%04d" month day year)
  ;;

  let parse t text =
    let first_separator, second_separator, separator =
      match t with
      | Iso -> 4, 7, '-'
      | Day_month_year | Month_day_year -> 2, 5, '/'
    in
    if
      String.length text <> 10
      || not
           (String.for_alli text ~f:(fun index char ->
              if index = first_separator || index = second_separator
              then Char.equal char separator
              else Char.is_digit char))
    then Or_error.error_string "date must use the exact ten-byte ASCII format"
    else (
      let decimal offset length =
        String.sub text ~pos:offset ~len:length |> Int.of_string
      in
      let year, month, day =
        match t with
        | Iso -> decimal 0 4, decimal 5 2, decimal 8 2
        | Day_month_year -> decimal 6 4, decimal 3 2, decimal 0 2
        | Month_day_year -> decimal 6 4, decimal 0 2, decimal 3 2
      in
      match Core.Month.of_int month with
      | Some month
        when year >= 1
             && year <= 9999
             && day >= 1
             && day <= Date.days_in_month ~year ~month ->
        Ok (Date.create_exn ~y:year ~m:month ~d:day)
      | None | Some _ ->
        Or_error.error_string "date is not a valid supported Gregorian day")
  ;;
end

module Wire_conversion = struct
  let date_to_ordinal date =
    Or_error.map (validate_date date) ~f:(fun date ->
      Int64.of_int (Date.diff date min_date))
  ;;

  let date_of_ordinal ordinal =
    if Int64.(ordinal < 0L || ordinal > 3652058L)
    then Or_error.error_string "calendar ordinal must be in 0..3652058"
    else Ok (Date.add_days min_date (Int64.to_int_exn ordinal))
  ;;

  let range_to_wire t =
    { W.Range.first = Int64.of_int (Date.diff (Range.first t) min_date)
    ; last = Int64.of_int (Date.diff (Range.last t) min_date)
    }
  ;;

  let range_of_wire t =
    let open Or_error.Let_syntax in
    let%bind first = date_of_ordinal t.W.Range.first in
    let%bind last = date_of_ordinal t.last in
    Range.create ~first ~last
  ;;

  let selection_to_wire = function
    | Selection.Empty -> W.Selection.Empty
    | Single date -> Single (Int64.of_int (Date.diff date min_date))
    | Range_start date -> Range_start (Int64.of_int (Date.diff date min_date))
    | Range range -> Range (range_to_wire range)
  ;;

  let selection_of_wire = function
    | W.Selection.Empty -> Ok Selection.empty
    | Single date -> Or_error.bind (date_of_ordinal date) ~f:Selection.single
    | Range_start date -> Or_error.bind (date_of_ordinal date) ~f:Selection.range_start
    | Range range -> Or_error.map (range_of_wire range) ~f:Selection.range
  ;;

  let constraints_to_wire t =
    { W.Constraints.min = Int64.of_int (Date.diff (Constraints.min t) min_date)
    ; max = Int64.of_int (Date.diff (Constraints.max t) min_date)
    ; disabled_dates =
        List.map (Constraints.disabled_dates t) ~f:(fun d ->
          Int64.of_int (Date.diff d min_date))
    ; disabled_ranges = List.map (Constraints.disabled_ranges t) ~f:range_to_wire
    ; disabled_weekdays =
        List.map (Constraints.disabled_weekdays t) ~f:(fun d ->
          Int64.of_int (Day_of_week.to_int d))
    ; range_policy = Constraints.range_policy t
    }
  ;;

  let constraints_of_wire t =
    if not (W.Constraints.valid t)
    then Or_error.error_string "invalid calendar wire constraints"
    else
      let open Or_error.Let_syntax in
      let%bind min = date_of_ordinal t.min in
      let%bind max = date_of_ordinal t.max in
      let%bind disabled_dates =
        List.map t.disabled_dates ~f:date_of_ordinal |> Or_error.all
      in
      let%bind disabled_ranges =
        List.map t.disabled_ranges ~f:range_of_wire |> Or_error.all
      in
      let disabled_weekdays =
        List.map t.disabled_weekdays ~f:(fun d ->
          Day_of_week.of_int_exn (Int64.to_int_exn d))
      in
      Constraints.create
        ~min
        ~max
        ~disabled_dates
        ~disabled_ranges
        ~disabled_weekdays
        ~range_policy:t.range_policy
        ()
  ;;
end

module Labels = struct
  type t = W.Labels.t [@@deriving equal, sexp_of]

  let validate t =
    if W.Labels.valid t
    then Ok t
    else
      Or_error.error_string
        "calendar labels require 12 months, 7 weekdays and 7 short weekdays, with \
         nonblank UTF-8 labels of at most 128 bytes and no ASCII controls"
  ;;

  let create
        ~months
        ~weekdays
        ~short_weekdays
        ~previous
        ~next
        ~choose_month
        ~choose_year
        ~today
        ~clear
        ()
    =
    validate
      { W.Labels.months
      ; weekdays
      ; short_weekdays
      ; previous
      ; next
      ; choose_month
      ; choose_year
      ; today
      ; clear
      }
  ;;

  let english =
    create
      ~months:
        [ "January"
        ; "February"
        ; "March"
        ; "April"
        ; "May"
        ; "June"
        ; "July"
        ; "August"
        ; "September"
        ; "October"
        ; "November"
        ; "December"
        ]
      ~weekdays:
        [ "Sunday"; "Monday"; "Tuesday"; "Wednesday"; "Thursday"; "Friday"; "Saturday" ]
      ~short_weekdays:[ "Sun"; "Mon"; "Tue"; "Wed"; "Thu"; "Fri"; "Sat" ]
      ~previous:"Previous"
      ~next:"Next"
      ~choose_month:"Choose month"
      ~choose_year:"Choose year"
      ~today:"Today"
      ~clear:"Clear"
      ()
    |> Or_error.ok_exn
  ;;
end

module Config = struct
  type t = W.Config.t [@@deriving equal, sexp_of]

  let of_wire t =
    if not (W.Config.valid t)
    then Or_error.error_string "invalid calendar configuration"
    else
      Or_error.map
        (Wire_conversion.constraints_of_wire t.constraints)
        ~f:(fun constraints ->
          { t with constraints = Wire_conversion.constraints_to_wire constraints })
  ;;

  let create
        ?(mode = Mode.Single)
        ?(constraints = Constraints.unrestricted)
        ?(first_weekday = Day_of_week.Mon)
        ?(labels = Labels.english)
        ?today
        ~label
        ?(disabled = false)
        ?(read_only = false)
        ?(auto_focus = false)
        ()
    =
    let open Or_error.Let_syntax in
    let%bind today =
      Option.map today ~f:Wire_conversion.date_to_ordinal
      |> Option.value_map ~default:(Ok None) ~f:(Or_error.map ~f:Option.some)
    in
    of_wire
      { W.Config.mode
      ; constraints = Wire_conversion.constraints_to_wire constraints
      ; first_weekday = Int64.of_int (Day_of_week.to_int first_weekday)
      ; labels
      ; today
      ; label
      ; disabled
      ; read_only
      ; auto_focus
      }
  ;;

  let mode t = t.W.Config.mode

  let constraints t =
    Wire_conversion.constraints_of_wire t.W.Config.constraints |> Or_error.ok_exn
  ;;

  let first_weekday t = Day_of_week.of_int_exn (Int64.to_int_exn t.W.Config.first_weekday)
  let labels t = t.W.Config.labels

  let today t =
    Option.map t.W.Config.today ~f:(fun day ->
      Wire_conversion.date_of_ordinal day |> Or_error.ok_exn)
  ;;

  let label t = t.W.Config.label
  let is_disabled t = t.W.Config.disabled
  let is_read_only t = t.W.Config.read_only
end

module Presentation = struct
  type t = W.Presentation.t =
    | Days
    | Months
    | Years
  [@@deriving equal, sexp_of]
end

module Revision = struct
  type t = int64 [@@deriving compare, equal, sexp_of]

  let of_int64 value =
    if Int64.(value >= 0L)
    then Ok value
    else Or_error.error_string "calendar revision must be nonnegative"
  ;;

  let to_int64 t = t
end

let month_to_wire t =
  Int64.of_int (((Month.year t - 1) * 12) + Core.Month.to_int (Month.month t) - 1)
;;

let month_of_wire value =
  if not (W.valid_month value)
  then Or_error.error_string "invalid calendar month index"
  else (
    let index = Int64.to_int_exn value in
    Month.create
      ~year:((index / 12) + 1)
      ~month:(Core.Month.of_int_exn ((index % 12) + 1)))
;;

module Snapshot = struct
  type t =
    { window : Gpuio_protocol.Window_id.t
    ; node : Gpuio_protocol.Node_id.t
    ; wire : W.Snapshot.t
    }
  [@@deriving equal, sexp_of]

  let revision t = t.wire.revision
  let mode t = t.wire.mode
  let selection t = Wire_conversion.selection_of_wire t.wire.selection |> Or_error.ok_exn
  let selection_allowed t = t.wire.selection_allowed
  let month t = month_of_wire t.wire.month |> Or_error.ok_exn

  let focused_date t =
    Wire_conversion.date_of_ordinal t.wire.focused_date |> Or_error.ok_exn
  ;;

  let presentation t = t.wire.presentation
  let focused t = t.wire.focused
end

module Event = struct
  type t =
    | Observed of Snapshot.t
    | Changed of Snapshot.t
    | Selected of Snapshot.t
    | Rejected of Selection_error.t * Snapshot.t
  [@@deriving equal, sexp_of]
end

module Command = struct
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

module Command_error = struct
  type t = W.Error.t =
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

module Expert = struct
  include Wire_conversion

  let config_to_wire t = t
  let config_of_wire = Config.of_wire
  let month_to_wire = month_to_wire
  let month_of_wire = month_of_wire

  let snapshot_of_wire ~window ~node wire =
    if W.Snapshot.valid wire
    then Ok { Snapshot.window; node; wire }
    else Or_error.error_string "invalid calendar snapshot"
  ;;

  let event_of_wire ~window ~node event =
    if not (W.Event.valid event)
    then Or_error.error_string "invalid calendar event"
    else
      Or_error.map
        (snapshot_of_wire ~window ~node (W.Event.snapshot event))
        ~f:(fun snapshot ->
          match event with
          | W.Event.Observed _ -> Event.Observed snapshot
          | Changed _ -> Changed snapshot
          | Selected _ -> Selected snapshot
          | Rejected (error, _) -> Rejected (error, snapshot))
  ;;

  let command_to_wire command =
    let open Or_error.Let_syntax in
    let%bind wire =
      match command with
      | Command.Replace { selection; if_revision } ->
        Ok (W.Command.Replace { selection = selection_to_wire selection; if_revision })
      | Clear { if_revision } -> Ok (W.Command.Clear { if_revision })
      | Show_month month -> Ok (W.Command.Show_month (month_to_wire month))
      | Move_months count -> Ok (W.Command.Move_months (Int64.of_int count))
      | Focus_date date ->
        Or_error.map (date_to_ordinal date) ~f:(fun date -> W.Command.Focus_date date)
      | Focus -> Ok W.Command.Focus
      | Set_presentation presentation -> Ok (W.Command.Set_presentation presentation)
      | Read_snapshot -> Ok W.Command.Read_snapshot
    in
    if W.Command.valid wire
    then Ok wire
    else Or_error.error_string "invalid calendar command"
  ;;

  let error_of_wire t = t
  let window t = t.Snapshot.window
  let node t = t.Snapshot.node
end
