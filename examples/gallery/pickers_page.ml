open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module Calendar_controller = Gpuio_eio.Calendar
module Date_picker_controller = Gpuio_eio.Date_picker
module Color_controller = Gpuio_eio.Color_input
module Color_picker_controller = Gpuio_eio.Color_picker

let ok = Or_error.ok_exn
let px = Length.px_exn
let style = Style.create_exn
let date = Date.of_string "2026-09-14"
let month = Calendar.Month.of_date date |> ok
let initial_date = Calendar.Selection.single date |> ok
let initial_color = Color_value.Value.Color (Color_value.Rgba.of_hex "#89DDC9" |> ok)

let describe_date = function
  | Calendar.Selection.Empty -> "No date selected"
  | Single value -> Date.to_string value
  | Range_start value -> Date.to_string value ^ " → choose an end date"
  | Range range ->
    Date.to_string (Calendar.Range.first range)
    ^ " → "
    ^ Date.to_string (Calendar.Range.last range)
;;

let describe_color = function
  | Color_value.Value.Empty -> "No color"
  | Color value -> Color_value.Rgba.to_hex value
;;

let color_config ~read_only ~label =
  Color_input.Config.create
    ~labels:(Color_input.Labels.english ~control:label |> ok)
    ~palette:
      (List.map
         [ "Mint", "#89DDC9"; "Iris", "#A3B5FF"; "Coral", "#F6A89D" ]
         ~f:(fun (label, hex) ->
           Color_input.Palette_entry.create
             ~color:(Color_value.Rgba.of_hex hex |> ok)
             ~label
           |> ok))
    ~read_only
    ()
  |> ok
;;

let component window palette graph =
  let read_only, toggle_read_only = B.toggle ~default_model:false graph in
  let committed_date, set_date = B.state initial_date graph in
  let committed_color, set_color = B.state initial_color graph in
  let open B.Let_syntax in
  let config mode label =
    let%arr read_only = read_only in
    Calendar.Config.create ~mode ~label ~today:date ~read_only () |> ok
  in
  let inline_calendar =
    Calendar_controller.create
      window
      ~config:(config Range "Preview date range")
      ~initial:Calendar.Selection.empty
      ~initial_month:month
      graph
  in
  let date_picker =
    Date_picker_controller.create
      window
      ~config:(config Single "Preview appointment")
      ~value:committed_date
      ~initial_month:month
      ~on_change:set_date
      graph
  in
  let config label =
    let%arr read_only = read_only in
    color_config ~read_only ~label
  in
  let inline_color =
    Color_controller.create
      window
      ~config:(config "Preview palette")
      ~initial:initial_color
      graph
  in
  let color_picker =
    Color_picker_controller.create
      window
      ~config:(config "Preview accent")
      ~value:committed_color
      ~on_change:set_color
      graph
  in
  let%arr p = palette
  and read_only = read_only
  and toggle_read_only = toggle_read_only
  and inline_calendar = inline_calendar
  and date_picker = date_picker
  and committed_date = committed_date
  and inline_color = inline_color
  and color_picker = color_picker
  and committed_color = committed_color in
  let overlay label =
    Overlay.Config.create ~label ~width:420. ~dismiss_on_outside_pointer:true () |> ok
  in
  let width = px (Palette.size p 340.) in
  V.column
    ~style:(style [ Gap (px 20.) ])
    [ V.switch ~checked:read_only ~on_toggle:toggle_read_only "Read-only pickers"
    ; Palette.card
        p
        ~title:"Choose, review, confirm"
        [ V.row
            ~style:(style [ Gap (px 16.); Wrap Wrap ])
            [ Date_picker_controller.view
                ~overlay:(overlay "Appointment picker")
                ~apply_label:"Apply appointment"
                ~cancel_label:"Cancel appointment"
                ~label:"Choose appointment"
                date_picker
            ; Color_picker_controller.view
                ~overlay:(overlay "Accent picker")
                ~apply_label:"Apply accent"
                ~cancel_label:"Cancel accent"
                ~label:"Choose accent"
                color_picker
            ]
        ; Palette.text p ("Appointment: " ^ describe_date committed_date)
        ; Palette.text p ("Accent: " ^ describe_color committed_color)
        ; Palette.text
            p
            ~muted:true
            "Apply confirms your choice. Cancel or Escape keeps the previous value."
        ]
    ; Palette.card
        p
        ~title:"A calendar with a clear range"
        [ Calendar_controller.view ~style:(style [ Width width ]) inline_calendar
        ; Palette.text
            p
            (Option.value_map
               (Calendar_controller.snapshot inline_calendar)
               ~default:"Choose a start date"
               ~f:(fun snapshot -> describe_date (Calendar.Snapshot.selection snapshot)))
        ]
    ; Palette.card
        p
        ~title:"Color, with room to explore"
        [ Color_controller.view ~style:(style [ Width width ]) inline_color
        ; Palette.text
            p
            ~muted:true
            "Use a swatch, edit the hex value, or adjust individual channels."
        ]
    ]
;;
