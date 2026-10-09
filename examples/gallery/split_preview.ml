open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn

let component palette graph =
  let mode, cycle =
    B.state_machine0
      ~default_model:0
      ~apply_action:(fun _ mode () -> (mode + 1) % 3)
      graph
  in
  let disabled, toggle_disabled = B.toggle ~default_model:false graph in
  let primary_disabled, toggle_primary = B.toggle ~default_model:false graph in
  let loading, toggle_loading = B.toggle ~default_model:false graph in
  let count, invoke =
    B.state_machine0 ~default_model:0 ~apply_action:(fun _ count () -> count + 1) graph
  in
  let open B.Let_syntax in
  let%arr p = palette
  and mode = mode
  and cycle = cycle
  and disabled = disabled
  and toggle_disabled = toggle_disabled
  and primary_disabled = primary_disabled
  and toggle_primary = toggle_primary
  and loading = loading
  and toggle_loading = toggle_loading
  and count = count
  and invoke = invoke in
  let command = Command.Id.of_string "gallery.split.run" |> ok in
  let commands =
    Command.Registry.create
      [ Command.create ~id:command ~label:"Run split action" ~on_invoke:invoke () |> ok ]
    |> ok
  in
  let part =
    style
      [ Height (px (Palette.size p 40.))
      ; Background (Background.solid (Color.rgba ~red:0 ~green:0 ~blue:0 ~alpha:0 |> ok))
      ; Padding_left (px 14.)
      ; Padding_right (px 14.)
      ; Radius 8.
      ; Border_width 1.
      ; Border_color (Palette.border p)
      ; Foreground (Palette.foreground p)
      ]
    |> fun s ->
    Style.with_state_exn s Hovered [ Background (Background.solid (Palette.accent p)) ]
    |> fun s ->
    Style.with_state_exn s Pressed [ Background (Background.solid (Palette.border p)) ]
  in
  let appearance =
    Split_button.Appearance.create
      ~surface:(style [ Background (Background.solid (Palette.border p)) ])
      ~menu_open:(style [ Background (Background.solid (Palette.accent p)) ])
      ()
    |> ok
  in
  let primary =
    V.command_button
      ~key:(Key.of_string_exn "split-primary")
      ~style:(Style.merge [ part; style [ Disabled primary_disabled ] ])
      ~config:(Button.Config.create ~loading ())
      ~command
      ()
  in
  let menu =
    V.menu_button
      ~key:(Key.of_string_exn "split-menu")
      ~style:part
      ~menu:(Menu.create ~label:"More split actions" [ Command command ] |> ok)
      ()
  in
  let primary =
    V.tooltip
      ~key:(Key.of_string_exn "split-primary-help")
      ~config:(Tooltip.Config.create ~label:"Run the current action" () |> ok)
      ~anchor:primary
      ~content:(Palette.text p "Run the current action")
      ()
  in
  let pair =
    V.split_button
      ~key:(Key.of_string_exn "split-pair")
      ~style:(style [ Disabled disabled ])
      ~appearance
      ?primary:(if mode = 2 then None else Some primary)
      ?menu:(if mode = 1 then None else Some menu)
      ()
    |> ok
  in
  Palette.card
    p
    ~title:"Two actions, one control"
    [ Palette.text
        p
        ~muted:true
        "Hover either half, then open the menu and move away. The shared surface stays \
         native; each action keeps its own focus and availability."
    ; V.row
        ~style:(style [ Gap (px 16.); Wrap Wrap ])
        [ Palette.button
            p
            ("Split mode: "
             ^
             match mode with
             | 0 -> "both"
             | 1 -> "action only"
             | _ -> "menu only")
            (cycle ())
        ; V.checkbox
            ~state:(Check_state.of_bool disabled)
            ~on_toggle:toggle_disabled
            "Disable split pair"
        ; V.checkbox
            ~state:(Check_state.of_bool primary_disabled)
            ~on_toggle:toggle_primary
            "Disable primary action"
        ; V.checkbox
            ~state:(Check_state.of_bool loading)
            ~on_toggle:toggle_loading
            "Primary action loading"
        ]
    ; V.command_scope ~commands [ pair ]
    ; Palette.text p (sprintf "Split action requests: %d" count)
    ]
;;
