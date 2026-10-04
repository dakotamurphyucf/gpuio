open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn

let component palette graph =
  let selected, set_selected = B.state "balanced" graph in
  let custom_order, toggle_order = B.toggle ~default_model:false graph in
  let skip_fast, toggle_skip = B.toggle ~default_model:false graph in
  let disabled, toggle_disabled = B.toggle ~default_model:false graph in
  let rich, toggle_rich = B.toggle ~default_model:true graph in
  let open B.Let_syntax in
  let%arr p = palette
  and selected = selected
  and set_selected = set_selected
  and custom_order = custom_order
  and toggle_order = toggle_order
  and skip_fast = skip_fast
  and toggle_skip = toggle_skip
  and disabled = disabled
  and toggle_disabled = toggle_disabled
  and rich = rich
  and toggle_rich = toggle_rich in
  let choices =
    [ "balanced", "Balanced", "Time to think, room to explore"
    ; "fast", "Fast", "A concise answer with less waiting"
    ; "deep", "Deep", "Work through the details together"
    ]
  in
  let radios =
    List.mapi choices ~f:(fun index (id, title, detail) ->
      let key = Key.of_string_exn ("standalone-" ^ id) in
      let checked = String.equal selected id in
      let on_select = set_selected id in
      let position = Radio.Position.create ~index ~count:(List.length choices) |> ok in
      let tab_order =
        if custom_order || skip_fast
        then
          Some
            (Tab_order.create
               ~tab_stop:(not (skip_fast && String.equal id "fast"))
               ~index:(if custom_order then -index else 0)
               ()
             |> ok)
        else None
      in
      if rich
      then
        V.radio_with_label
          ~key
          ~checked
          ~on_select
          ~disabled
          ?tab_order
          ~position
          ~accessible_name:title
          (V.column
             ~style:(style [ Gap (px 4.) ])
             [ Palette.text p title; Palette.text p ~muted:true detail ])
        |> ok
      else V.radio ~key ~checked ~on_select ~disabled ?tab_order ~position title)
  in
  let group = V.column ~style:(style [ Gap (px 16.) ]) radios in
  let group =
    View.with_accessibility
      group
      (Accessibility.create ~role:(Radio_group Vertical) ~label:"Response depth" () |> ok)
    |> ok
  in
  Palette.card
    p
    ~title:"Compose a choice your way"
    [ Palette.text
        p
        ~muted:true
        "Independent radio controls share application selection. Tab visits each choice; \
         selecting the active choice leaves it selected. Use the managed radio group for \
         arrow-key navigation."
    ; V.row
        ~style:(style [ Gap (px 16.); Wrap Wrap ])
        [ V.checkbox
            ~state:(Check_state.of_bool custom_order)
            ~on_toggle:toggle_order
            "Reverse choice Tab order"
        ; V.checkbox
            ~state:(Check_state.of_bool skip_fast)
            ~on_toggle:toggle_skip
            "Skip Fast with Tab"
        ; V.checkbox
            ~state:(Check_state.of_bool disabled)
            ~on_toggle:toggle_disabled
            "Disable standalone choices"
        ; V.checkbox
            ~state:(Check_state.of_bool rich)
            ~on_toggle:toggle_rich
            "Detailed choice labels"
        ]
    ; group
    ; Palette.text p ("Response depth: " ^ selected)
    ; Palette.text
        p
        ~muted:true
        "Skipping a Tab stop keeps pointer selection available. Resetting both \
         navigation options restores normal order without replacing the controls."
    ]
;;
