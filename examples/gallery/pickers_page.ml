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

let month_heading month =
  let labels =
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
  in
  sprintf
    "%s %d"
    (List.nth_exn labels (Month.to_int (Calendar.Month.month month) - 1))
    (Calendar.Month.year month)
;;

let date_presets =
  let make id label selection =
    Date_picker.Preset.create ~id:(Choice.Id.of_string id |> ok) ~label ~selection |> ok
  in
  Date_picker.Preset.Collection.create
    [ make "demo-day" "Demo day" initial_date
    ; make
        "next-week"
        "One week later"
        (Calendar.Selection.single (Date.add_days date 7) |> ok)
    ; make "clear" "Clear date" Calendar.Selection.empty
    ]
  |> ok
;;

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

let color_entries colors =
  List.map colors ~f:(fun (label, hex) ->
    Color_input.Palette_entry.create ~color:(Color_value.Rgba.of_hex hex |> ok) ~label
    |> ok)
;;

let color_sections ~extended =
  let featured =
    Color_input.Palette_section.featured
      ~label:"Favorites"
      (color_entries [ "Mint", "#89DDC9"; "Iris", "#A3B5FF"; "Coral", "#F6A89D" ])
    |> ok
  in
  let families =
    [ "Stone", [ "#E7E5E4"; "#A8A29E"; "#57534E" ]
    ; "Red", [ "#FECACA"; "#F87171"; "#B91C1C" ]
    ; "Orange", [ "#FED7AA"; "#FB923C"; "#C2410C" ]
    ; "Yellow", [ "#FEF08A"; "#FACC15"; "#A16207" ]
    ; "Green", [ "#BBF7D0"; "#4ADE80"; "#15803D" ]
    ; "Cyan", [ "#A5F3FC"; "#22D3EE"; "#0E7490" ]
    ; "Blue", [ "#BFDBFE"; "#60A5FA"; "#1D4ED8" ]
    ; "Purple", [ "#E9D5FF"; "#C084FC"; "#7E22CE" ]
    ; "Pink", [ "#FBCFE8"; "#F472B6"; "#BE185D" ]
    ]
  in
  featured
  :: List.filter_map families ~f:(fun (label, colors) ->
    if extended || String.equal label "Blue"
    then
      Some
        (Color_input.Palette_section.group
           ~label
           (List.mapi colors ~f:(fun index color ->
              sprintf "%s shade %d" label (index + 1), color)
            |> color_entries)
         |> ok)
    else None)
;;

let color_config ~extended ~read_only ~label =
  Color_input.Config.create
    ~labels:(Color_input.Labels.english ~control:label |> ok)
    ~palette_sections:(color_sections ~extended)
    ~read_only
    ()
  |> ok
;;

let component window palette graph =
  let choices = Choice_picker_preview.component window palette graph in
  let months, set_months = B.state 2 graph in
  let show_events, toggle_events = B.toggle ~default_model:true graph in
  let event_count, set_event_count = B.state 2 graph in
  let inline_viewport, set_inline_viewport = B.state None graph in
  let picker_viewport, set_picker_viewport = B.state None graph in
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
  let config ~extended label =
    let%arr read_only = read_only in
    color_config ~extended ~read_only ~label
  in
  let inline_color =
    Color_controller.create
      window
      ~config:(config ~extended:true "Preview palette")
      ~initial:initial_color
      graph
  in
  let color_picker =
    Color_picker_controller.create
      window
      ~config:(config ~extended:false "Preview accent")
      ~value:committed_color
      ~on_change:set_color
      graph
  in
  let%arr p = palette
  and months = months
  and set_months = set_months
  and show_events = show_events
  and toggle_events = toggle_events
  and inline_viewport = inline_viewport
  and set_inline_viewport = set_inline_viewport
  and picker_viewport = picker_viewport
  and set_picker_viewport = set_picker_viewport
  and event_count = event_count
  and set_event_count = set_event_count
  and choices = choices
  and read_only = read_only
  and toggle_read_only = toggle_read_only
  and inline_calendar = inline_calendar
  and date_picker = date_picker
  and committed_date = committed_date
  and set_date = set_date
  and inline_color = inline_color
  and color_picker = color_picker
  and committed_color = committed_color
  and set_color = set_color in
  let overlay label =
    Overlay.Config.create ~label ~width:420. ~dismiss_on_outside_pointer:true () |> ok
  in
  let width = px (Palette.size p 340.) in
  let color_appearance =
    Color_input.Appearance.create
      ~swatch_size:(Palette.size p 28.)
      ~featured_size:(Palette.size p 36.)
      ~swatch_gap:(Palette.size p 6.)
      ~section_gap:(Palette.size p 12.)
      ~swatch_radius:(Palette.size p 6.)
      ~outline_width:(Palette.size p 2.)
      ~channel_height:(Palette.size p 28.)
      ~control_gap:(Palette.size p 10.)
      ~padding:(Palette.size p 10.)
      ~selected_border:(Palette.accent p)
      ~hover_border:(Palette.foreground p)
      ~panels:
        (Color_input.Panels.tabs ~palette_label:"Palette" ~channels_label:"HSLA" () |> ok)
      ()
    |> ok
  in
  let calendar_appearance months =
    Calendar.Appearance.create
      ~months
      ~cell_height:(Palette.size p 32.)
      ~cell_gap:(Palette.size p 4.)
      ~month_gap:(Palette.size p 16.)
      ~padding:(Palette.size p 8.)
      ~cell_radius:(Palette.size p 6.)
      ~selected_background:(Palette.accent p)
      ~selected_foreground:(Palette.background p)
      ~hover_background:(Palette.border p)
      ~today_border:(Palette.accent p)
      ~focus_border:(Palette.foreground p)
      ~muted_foreground:(Palette.muted p)
      ()
    |> ok
  in
  let calendar_content viewport =
    let module C = View.Calendar_content in
    let item ?description slot content = C.Item.create ?description ~slot content |> ok in
    let days =
      let dates =
        Option.value_map
          viewport
          ~default:[ date; Date.add_days date 3; Date.add_days date 7 ]
          ~f:Calendar.Viewport.dates
      in
      List.filter_map dates ~f:(fun date ->
        let day = Date.day date in
        if day = 14 || day = 17 || day = 21
        then Some (date, if day = 14 then event_count else if day = 17 then 1 else 3)
        else None)
      |> List.map ~f:(fun (date, count) ->
        item
          ~description:(sprintf "%d scheduled events" count)
          (Calendar.Slot.day date |> ok)
          (V.row
             ~style:(style [ Gap (px 4.); Align_items Center; Justify_content Center ])
             [ V.text (Int.to_string (Date.day date))
             ; V.text
                 ~style:
                   (style
                      [ Font_size 9.
                      ; Width (px 14.)
                      ; Height (px 14.)
                      ; Text_align Center
                      ; Radius 7.
                      ; Border_width 1.
                      ; Border_color (Palette.background p)
                      ; Background (Background.solid (Palette.accent p))
                      ; Foreground (Palette.background p)
                      ])
                 (Int.to_string count)
             ]))
    in
    C.create
      (if show_events
       then
         [ item Calendar.Slot.previous (V.text ~style:(style [ Font_size 20. ]) "←")
         ; item Calendar.Slot.next (V.text ~style:(style [ Font_size 20. ]) "→")
         ]
         @ List.map
             (Option.value_map viewport ~default:[ month ] ~f:Calendar.Viewport.months)
             ~f:(fun month ->
               item
                 ~description:"Demo appointments"
                 (Calendar.Slot.month_heading month)
                 (V.row
                    ~style:(style [ Gap (px 6.); Justify_content Center ])
                    [ V.text (month_heading month)
                    ; Palette.text p ~muted:true ~size:11. "SCHEDULE"
                    ]))
         @ days
       else [])
    |> ok
  in
  let trigger_style =
    style
      [ Padding (px (Palette.size p 12.))
      ; Radius 10.
      ; Border_width 1.
      ; Border_color (Palette.border p)
      ; Background (Background.solid (Palette.background p))
      ; Foreground (Palette.foreground p)
      ]
    |> fun s ->
    Style.with_state_exn s Hovered [ Background (Background.solid (Palette.border p)) ]
  in
  let caption title value =
    V.column
      ~style:(style [ Gap (px 3.); Align_items Start ])
      [ Palette.text p ~size:11. ~muted:true title; Palette.text p value ]
  in
  let trigger badge title value =
    V.row
      ~style:(style [ Gap (px 12.); Align_items Center ])
      [ badge; caption title value; Palette.text p ~muted:true "⌄" ]
  in
  let date_badge =
    V.text
      ~style:
        (style
           [ Font_size (Palette.size p 20.)
           ; Font_weight 600
           ; Foreground (Palette.accent p)
           ; Padding (px 6.)
           ])
      (match committed_date with
       | Calendar.Selection.Single date -> Int.to_string (Date.day date)
       | Empty | Range_start _ | Range _ -> "—")
  in
  let color_badge =
    let color =
      match committed_color with
      | Color_value.Value.Empty -> Palette.border p
      | Color value -> Color_value.Rgba.to_color value
    in
    V.column
      ~style:
        (style
           [ Width (px (Palette.size p 32.))
           ; Height (px (Palette.size p 32.))
           ; Radius 8.
           ; Background (Background.solid color)
           ; Border_width 1.
           ; Border_color (Palette.border p)
           ])
      []
  in
  V.column
    ~style:(style [ Gap (px 20.) ])
    [ choices
    ; V.switch ~checked:read_only ~on_toggle:toggle_read_only "Read-only pickers"
    ; Palette.card
        p
        ~title:"Choose, review, confirm"
        [ V.row
            ~style:(style [ Gap (px 16.); Wrap Wrap ])
            [ Date_picker_controller.view_with_trigger
                ~trigger_style
                ~trigger:(trigger date_badge "Appointment" (describe_date committed_date))
                ~appearance:(calendar_appearance 1)
                ~calendar_content:(calendar_content picker_viewport)
                ~on_calendar_viewport_change:(fun viewport ->
                  set_picker_viewport (Some viewport))
                ~presets:date_presets
                ~overlay:(overlay "Appointment picker")
                ~apply_label:"Apply appointment"
                ~cancel_label:"Cancel appointment"
                ~accessible_name:"Choose appointment"
                date_picker
              |> ok
            ; Color_picker_controller.view_with_trigger
                ~style:(style [ Max_height (px 520.); Overflow_y Scroll ])
                ~trigger_style
                ~appearance:color_appearance
                ~trigger:(trigger color_badge "Accent" (describe_color committed_color))
                ~overlay:(overlay "Accent picker")
                ~apply_label:"Apply accent"
                ~cancel_label:"Cancel accent"
                ~accessible_name:"Choose accent"
                color_picker
              |> ok
            ]
        ; V.row
            ~style:(style [ Gap (px 8.); Wrap Wrap ])
            [ Palette.button
                p
                ~disabled:read_only
                "Clear appointment"
                (Bonsai.Effect.Many
                   [ Date_picker_controller.cancel date_picker
                   ; set_date Calendar.Selection.empty
                   ])
            ; Palette.button
                p
                ~disabled:read_only
                "Clear accent"
                (Bonsai.Effect.Many
                   [ Color_picker_controller.cancel color_picker
                   ; set_color Color_value.Value.Empty
                   ])
            ]
        ; Palette.text p ("Appointment: " ^ describe_date committed_date)
        ; Palette.text p ("Accent: " ^ describe_color committed_color)
        ; Palette.text
            p
            ~muted:true
            "Presets update the date draft. Apply confirms your choice; Cancel or Escape \
             keeps the previous value."
        ]
    ; Palette.card
        p
        ~title:"A calendar with a clear range"
        [ V.row
            ~style:(style [ Gap (px 8.); Wrap Wrap ])
            (List.map [ 1; 2; 3; 12 ] ~f:(fun count ->
               Palette.button
                 p
                 ~selected:(months = count)
                 (sprintf "%d month%s" count (if count = 1 then "" else "s"))
                 (set_months count)))
        ; V.row
            ~style:(style [ Gap (px 8.); Wrap Wrap ])
            [ Palette.button p ~selected:show_events "Event badges" toggle_events
            ; Palette.button p "Update events" (set_event_count ((event_count mod 9) + 1))
            ]
        ; Palette.text
            p
            ~muted:true
            (Option.value_map
               inline_viewport
               ~default:"Waiting for native calendar panes…"
               ~f:(fun viewport ->
                 let dates = Calendar.Viewport.dates viewport in
                 match List.hd dates, List.last dates with
                 | Some first, Some last ->
                   sprintf
                     "Event content for %s – %s · %d grid dates"
                     (Date.to_string first)
                     (Date.to_string last)
                     (List.length dates)
                 | _ -> "Choose a month or year to load day content"))
        ; Calendar_controller.view
            ~appearance:(calendar_appearance months)
            ~content:(calendar_content inline_viewport)
            ~on_viewport_change:(fun viewport -> set_inline_viewport (Some viewport))
            ~style:
              (style
                 [ Width (px (Palette.size p (if months = 1 then 340. else 720.)))
                 ; Max_width (Length.percent_exn 100.)
                 ])
            inline_calendar
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
        [ Color_controller.view
            ~appearance:color_appearance
            ~style:(style [ Width width ])
            inline_color
        ; Palette.text
            p
            ~muted:true
            "Hover over a swatch to preview it without changing your draft. Select a \
             swatch, edit the hex value, or adjust individual channels."
        ]
    ]
;;
