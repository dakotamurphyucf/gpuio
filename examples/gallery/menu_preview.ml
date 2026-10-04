open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn

let component palette graph =
  let open_, set_open = B.state false graph in
  let observe, toggle_observe = B.toggle ~default_model:true graph in
  let disabled, toggle_disabled = B.toggle ~default_model:false graph in
  let right, toggle_right = B.toggle ~default_model:false graph in
  let trailing, toggle_trailing = B.toggle ~default_model:false graph in
  let count, invoke =
    B.state_machine0 ~default_model:0 ~apply_action:(fun _ count () -> count + 1) graph
  in
  let open B.Let_syntax in
  let%arr p = palette
  and open_ = open_
  and set_open = set_open
  and observe = observe
  and toggle_observe = toggle_observe
  and disabled = disabled
  and toggle_disabled = toggle_disabled
  and right = right
  and toggle_right = toggle_right
  and trailing = trailing
  and toggle_trailing = toggle_trailing
  and count = count
  and invoke = invoke in
  let command = Command.Id.of_string "gallery.menu.preview" |> ok in
  let commands =
    Command.Registry.create
      [ Command.create ~id:command ~label:"Run menu preview" ~on_invoke:invoke () |> ok ]
    |> ok
  in
  let menu =
    Menu.create
      ~label:"Preview menu"
      ~disabled
      [ Command command
      ; Submenu (Menu.create ~label:"More previews" [ Command command ] |> ok)
      ]
    |> ok
  in
  let placement =
    Placement.create
      ~side:(if right then Right else Bottom)
      ~align:(if trailing then End else Start)
      ~offset:8.
      ()
    |> ok
  in
  Palette.card
    p
    ~title:"Menus that report their state"
    [ Palette.text
        p
        ~muted:true
        "Open the menu with a click or keyboard. Navigation stays native; the visibility \
         label follows its open and close observations."
    ; V.row
        ~style:(style [ Gap (px 16.); Wrap Wrap ])
        [ V.checkbox
            ~state:(Check_state.of_bool observe)
            ~on_toggle:toggle_observe
            "Observe menu visibility"
        ; V.checkbox
            ~state:(Check_state.of_bool disabled)
            ~on_toggle:toggle_disabled
            "Disable preview menu"
        ; V.checkbox
            ~state:(Check_state.of_bool right)
            ~on_toggle:toggle_right
            "Prefer menu on right"
        ; V.checkbox
            ~state:(Check_state.of_bool trailing)
            ~on_toggle:toggle_trailing
            "Align menu to trailing edge"
        ]
    ; V.command_scope
        ~commands
        [ V.row
            ~style:(style [ Align_items Center; Gap (px 16.) ])
            [ V.menu_button
                ~key:(Key.of_string_exn "observed-preview-menu")
                ~style:
                  (style
                     [ Padding (px 12.)
                     ; Radius 8.
                     ; Background (Background.solid (Palette.border p))
                     ; Foreground (Palette.foreground p)
                     ])
                ~placement
                ?on_open_change:(if observe then Some set_open else None)
                ~menu
                ()
            ; Palette.text
                p
                ("Menu visibility: "
                 ^
                 if not observe
                 then "not observed"
                 else if open_
                 then "open"
                 else "closed")
            ]
        ]
    ; Palette.text p (sprintf "Menu preview requests: %d" count)
    ]
;;
