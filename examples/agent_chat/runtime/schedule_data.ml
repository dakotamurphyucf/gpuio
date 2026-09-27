open Core
module C = Gpuio.Calendar

let ok = Or_error.ok_exn
let date day = Date.create_exn ~y:2026 ~m:Oct ~d:day
let month = C.Month.of_date (date 1) |> ok

let constraints =
  C.Constraints.create
    ~min:(date 1)
    ~max:(date 31)
    ~disabled_weekdays:[ Sat; Sun ]
    ~disabled_dates:[ date 20 ]
    ~range_policy:Endpoints_only
    ()
  |> ok
;;

let config ~mode ~label = C.Config.create ~mode ~label ~constraints () |> ok

module Review = struct
  type t =
    { date : Date.t
    ; label : string
    }

  let date t = t.date
  let label t = t.label
end

let describe = function
  | C.Selection.Empty -> "No date selected"
  | Single date -> Date.to_string date
  | Range_start date -> Date.to_string date ^ " → choose an end date"
  | Range range ->
    Date.to_string (C.Range.first range) ^ " – " ^ Date.to_string (C.Range.last range)
;;

let reviews selection =
  let mode =
    match selection with
    | C.Selection.Single _ -> C.Mode.Single
    | Empty | Range_start _ | Range _ -> Range
  in
  if
    match selection with
    | C.Selection.Range_start _ -> true
    | Empty | Single _ | Range _ -> false
  then Or_error.error_string "Choose an end date before applying the review filter"
  else if not (C.Constraints.allows_selection constraints selection ~mode)
  then Or_error.error_string "Choose available October 2026 endpoints"
  else
    List.init 31 ~f:(fun index -> date (index + 1))
    |> List.filter ~f:(fun date ->
      C.Constraints.allows constraints date
      &&
      match selection with
      | Empty -> true
      | Single selected -> Date.equal selected date
      | Range range -> C.Range.contains range date
      | Range_start _ -> false)
    |> List.map ~f:(fun date ->
      { Review.date
      ; label =
          (match Date.day date mod 3 with
           | 0 -> "Source notes"
           | 1 -> "Workspace changes"
           | _ -> "Generated artifacts")
      })
    |> Result.return
;;
