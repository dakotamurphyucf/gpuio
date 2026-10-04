open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module State = Gpuio_gallery_model.Selection_state

module Entry = struct
  type t =
    { id : Command.Id.t
    ; label : string
    ; checked : bool
    ; enabled : bool
    ; loading : bool
    ; action : State.Action.t
    }
end

let ok = Or_error.ok_exn
let px = Length.px_exn
let style = Style.create_exn

let component palette graph =
  let state, act =
    B.state_machine0
      ~default_model:State.initial
      ~apply_action:(fun _ state action -> State.apply state action)
      graph
  in
  let vertical, toggle_vertical = B.toggle ~default_model:false graph in
  let connected, toggle_connected = B.toggle ~default_model:true graph in
  let single, toggle_single = B.toggle ~default_model:false graph in
  let open B.Let_syntax in
  let%arr p = palette
  and state = state
  and act = act
  and vertical = vertical
  and toggle_vertical = toggle_vertical
  and connected = connected
  and toggle_connected = toggle_connected
  and single = single
  and toggle_single = toggle_single in
  let enabled = State.enabled state in
  let toolbar label entries =
    let commands =
      List.map entries ~f:(fun { Entry.id; label; checked; enabled; loading; action } ->
        ( Command.create ~id ~label ~checked ~enabled ~on_invoke:(fun () -> act action) ()
          |> ok
        , loading ))
    in
    let count = List.length commands in
    let buttons =
      List.mapi commands ~f:(fun index (command, loading) ->
        let first = index = 0 in
        let last = index = count - 1 in
        (* Each shared seam is painted by the preceding button. A singleton
           keeps all four corners and edges, in either orientation. *)
        let corners =
          if not connected
          then [ Style.Property.Radius 8.; Border_width 1. ]
          else (
            let radius rounded = if rounded then 8. else 0. in
            [ Top_left_radius (radius first)
            ; Top_right_radius (radius (if vertical then first else last))
            ; Bottom_left_radius (radius (if vertical then last else first))
            ; Bottom_right_radius (radius last)
            ; Border_top_width (if vertical && not first then 0. else 1.)
            ; Border_left_width (if (not vertical) && not first then 0. else 1.)
            ; Border_right_width 1.
            ; Border_bottom_width 1.
            ])
        in
        V.command_button_with_content
          ~key:(Key.of_string_exn (Command.Id.to_string (Command.id command)))
          ~style:
            (style
               (corners
                @ [ Padding (px (Palette.size p 10.))
                  ; Border_color (Palette.border p)
                  ; Background (Background.solid (Palette.surface p))
                  ; Foreground (Palette.foreground p)
                  ])
             |> fun base ->
             Style.with_state_exn
               base
               Checked
               [ Background (Background.solid (Palette.border p))
               ; Foreground (Palette.accent p)
               ]
             |> fun checked ->
             Style.with_state_exn checked Focused [ Border_color (Palette.accent p) ]
             |> fun focused -> Style.with_state_exn focused Disabled [ Opacity 0.45 ])
          ~command:(Command.id command)
          ~config:(Button.Config.create ~loading ())
          (V.row
             ~style:(style [ Align_items Center; Gap (px 6.) ])
             ((if loading
               then
                 [ V.loading
                     ~style:(style [ Width (px 14.); Height (px 14.) ])
                     ~config:
                       (Loading.Config.create
                          ~kind:Spinner
                          ~label:"Formatting activity"
                          ()
                        |> ok)
                     ()
                 ]
               else [])
              @ [ V.text (Command.label command) ]))
        |> ok)
    in
    let row =
      (if vertical then V.column else V.row)
        ~style:(style [ Gap (px (if connected then 0. else 6.)); Align_items Stretch ])
        buttons
    in
    let row =
      V.with_accessibility
        row
        (Accessibility.create
           ~role:(Toolbar (if vertical then Vertical else Horizontal))
           ~label
           ()
         |> ok)
      |> ok
    in
    V.command_scope
      ~commands:(Command.Registry.create (List.map commands ~f:fst) |> ok)
      [ row ]
  in
  Palette.card
    p
    ~title:"Selection, with a clear owner"
    [ Palette.text
        p
        ~muted:true
        "Choose one alignment or combine formatting options. Tab moves between buttons; \
         Space or Enter activates."
    ; V.switch
        ~checked:enabled
        ~on_toggle:(act Toggle_enabled)
        "Enable formatting controls"
    ; V.checkbox
        ~state:(Check_state.of_bool (State.italic_enabled state))
        ~on_toggle:(act Toggle_italic_enabled)
        "Enable Italic option"
    ; V.checkbox
        ~state:(Check_state.of_bool (State.bold_loading state))
        ~on_toggle:(act Toggle_bold_loading)
        "Bold option busy"
    ; Palette.text
        p
        ~muted:true
        "Busy and disabled options keep their selection. Select all changes only \
         available options."
    ; V.checkbox
        ~state:(Check_state.of_bool vertical)
        ~on_toggle:toggle_vertical
        "Vertical formatting toolbars"
    ; V.row
        ~style:(style [ Gap (px 16.); Wrap Wrap ])
        [ V.checkbox
            ~state:(Check_state.of_bool connected)
            ~on_toggle:toggle_connected
            "Connected formatting buttons"
        ; V.checkbox
            ~state:(Check_state.of_bool single)
            ~on_toggle:toggle_single
            "Single item groups"
        ]
    ; toolbar
        "Text alignment"
        (List.map
           (if single then [ State.alignment state ] else State.Alignment.all)
           ~f:(fun alignment ->
             { Entry.id = State.Alignment.command_id alignment
             ; label = State.Alignment.label alignment
             ; checked = State.Alignment.equal alignment (State.alignment state)
             ; enabled
             ; loading = false
             ; action = State.Action.Align alignment
             }))
    ; toolbar
        "Text formatting"
        (List.map
           (if single then [ State.Format.Bold ] else State.Format.all)
           ~f:(fun format ->
             { Entry.id = State.Format.command_id format
             ; label = State.Format.label format
             ; checked = State.selected state format
             ; enabled = State.format_enabled state format
             ; loading = State.format_loading state format
             ; action = State.Action.Toggle_format format
             }))
    ; V.checkbox
        ~disabled:(not enabled)
        ~state:(State.master state)
        ~on_toggle:(act Toggle_all)
        "All formatting options"
    ; Palette.text
        p
        ("Formatting: "
         ^
         match List.filter State.Format.all ~f:(State.selected state) with
         | [] -> "Plain"
         | formats -> List.map formats ~f:State.Format.label |> String.concat ~sep:", ")
    ; Palette.text p ("Alignment: " ^ State.Alignment.label (State.alignment state))
    ]
;;
