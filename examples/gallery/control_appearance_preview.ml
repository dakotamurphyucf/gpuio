open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View

let ok = Or_error.ok_exn
let px = Length.px_exn
let style = Style.create_exn
let key = Key.of_string_exn
let choice_id value = Choice.Id.of_string value |> ok

let component palette graph =
  let custom, toggle_custom = B.toggle ~default_model:true graph in
  let large, toggle_large = B.toggle ~default_model:false graph in
  let before, toggle_before = B.toggle ~default_model:false graph in
  let enabled, toggle_enabled = B.toggle ~default_model:true graph in
  let inert, toggle_inert = B.toggle ~default_model:false graph in
  let checked, toggle_checked = B.toggle ~default_model:true graph in
  let mixed, toggle_mixed = B.toggle ~default_model:false graph in
  let rich, toggle_rich = B.toggle ~default_model:false graph in
  let selected, set_selected = B.state (Some (choice_id "balanced")) graph in
  let open B.Let_syntax in
  let%arr p = palette
  and custom = custom
  and toggle_custom = toggle_custom
  and large = large
  and toggle_large = toggle_large
  and before = before
  and toggle_before = toggle_before
  and enabled = enabled
  and toggle_enabled = toggle_enabled
  and inert = inert
  and toggle_inert = toggle_inert
  and checked = checked
  and toggle_checked = toggle_checked
  and mixed = mixed
  and toggle_mixed = toggle_mixed
  and rich = rich
  and toggle_rich = toggle_rich
  and selected = selected
  and set_selected = set_selected in
  let appearance =
    Option.some_if
      custom
      (Control_appearance.create
         ~size:(if large then 32. else 18.)
         ~switch_width:(if large then 64. else 36.)
         ~gap:12.
         ~label_position:(if before then Before else After)
         ~indicator_style:
           (style
              [ Background (Background.solid (Palette.surface p))
              ; Foreground (Palette.border p)
              ; Border_width 2.
              ]
            |> fun base ->
            Style.with_state_exn
              base
              Checked
              [ Background (Background.solid (Palette.accent p))
              ; Foreground (Palette.accent p)
              ]
            |> fun base ->
            Style.with_state_exn
              base
              Indeterminate
              [ Background (Background.solid (Palette.foreground p))
              ; Foreground (Palette.foreground p)
              ]
            |> fun base -> Style.with_state_exn base Disabled [ Opacity 0.45 ])
         ~mark_style:(style [ Foreground (Palette.background p) ])
         ()
       |> ok)
  in
  let options =
    Choice.Collection.create
      [ Choice.create ~id:(choice_id "balanced") ~label:"Balanced" () |> ok
      ; Choice.create ~id:(choice_id "fast") ~label:"Fast" () |> ok
      ; Choice.create ~id:(choice_id "locked") ~label:"Unavailable" ~disabled:true ()
        |> ok
      ]
    |> ok
  in
  let label title detail =
    V.column
      ~style:(style [ Gap (px 4.) ])
      [ Palette.text p title; Palette.text p ~muted:true detail ]
  in
  let checkbox =
    let key = key "appearance-checkbox" in
    let state =
      if mixed then Check_state.Indeterminate else Check_state.of_bool checked
    in
    if rich
    then
      V.checkbox_with_label
        ~key
        ?appearance
        ~state
        ~on_toggle:toggle_checked
        ~accessible_name:"Include context"
        (label "Include context" "Attach the active conversation to your next request.")
      |> ok
    else V.checkbox ~key ?appearance ~state ~on_toggle:toggle_checked "Include context"
  in
  let switch =
    let key = key "appearance-switch" in
    if rich
    then
      V.switch_with_label
        ~key
        ?appearance
        ~checked
        ~on_toggle:toggle_checked
        ~accessible_name:"Stream responses"
        (label "Stream responses" "Read each response as it arrives.")
      |> ok
    else V.switch ~key ?appearance ~checked ~on_toggle:toggle_checked "Stream responses"
  in
  let radio =
    let config =
      Choice.Config.create ~label:"Indicator response mode" ~options ~selected () |> ok
    in
    let key = key "appearance-radio" in
    let style = style [ Direction Row; Gap (px 16.); Wrap Wrap ] in
    let on_select choice = set_selected (Some choice) in
    if rich
    then
      V.radio_group_with_labels
        ~key
        ?appearance
        ~style
        ~config
        ~on_select
        ~labels:
          [ choice_id "balanced", label "Balanced" "Depth and responsiveness"
          ; choice_id "locked", label "Unavailable" "Requires an additional model"
          ]
        ()
      |> ok
    else V.radio_group ~key ?appearance ~style ~config ~on_select ()
  in
  Palette.card
    p
    ~title:"Details that feel like your app"
    [ Palette.text
        p
        ~muted:true
        "Resize the native indicators, put labels first, or return to the defaults. \
         Selection and keyboard focus belong to the same controls throughout."
    ; V.row
        ~style:(style [ Gap (px 16.); Wrap Wrap ])
        [ V.checkbox
            ~state:(Check_state.of_bool custom)
            ~on_toggle:toggle_custom
            "Custom indicators"
        ; V.checkbox
            ~state:(Check_state.of_bool large)
            ~on_toggle:toggle_large
            "Large indicators"
        ; V.checkbox
            ~state:(Check_state.of_bool rich)
            ~on_toggle:toggle_rich
            "Rich control labels"
        ; V.checkbox
            ~state:(Check_state.of_bool before)
            ~on_toggle:toggle_before
            "Labels first"
        ]
    ; V.row
        ~style:(style [ Gap (px 16.); Wrap Wrap ])
        [ V.checkbox
            ~state:(Check_state.of_bool enabled)
            ~on_toggle:toggle_enabled
            "Enable indicator examples"
        ; V.checkbox
            ~state:(Check_state.of_bool inert)
            ~on_toggle:toggle_inert
            "Make indicator examples inert"
        ; V.checkbox
            ~state:(Check_state.of_bool mixed)
            ~on_toggle:toggle_mixed
            "Mixed checkbox example"
        ]
    ; V.column
        ~style:(style [ Gap (px 16.); Disabled (not enabled); Inert inert ])
        [ checkbox; switch; radio ]
    ; Palette.text p (sprintf "Indicator value: %s" (if checked then "on" else "off"))
    ; Palette.text
        p
        ("Indicator mode: "
         ^ Option.value_map selected ~default:"None" ~f:Choice.Id.to_string)
    ]
;;
