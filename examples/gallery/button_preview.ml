open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn

let component palette graph =
  let busy, toggle_busy = B.toggle ~default_model:false graph in
  let preserve, toggle_preserve = B.toggle ~default_model:false graph in
  let skip, toggle_skip = B.toggle ~default_model:false graph in
  let rich, toggle_rich = B.toggle ~default_model:true graph in
  let count, increment =
    B.state_machine0 ~default_model:0 ~apply_action:(fun _ count () -> count + 1) graph
  in
  let open B.Let_syntax in
  let%arr p = palette
  and busy = busy
  and toggle_busy = toggle_busy
  and preserve = preserve
  and toggle_preserve = toggle_preserve
  and skip = skip
  and toggle_skip = toggle_skip
  and rich = rich
  and toggle_rich = toggle_rich
  and count = count
  and increment = increment in
  let command = Command.Id.of_string "gallery.button.publish" |> ok in
  let commands =
    Command.Registry.create
      [ Command.create ~id:command ~label:"Publish draft" ~on_invoke:increment () |> ok ]
    |> ok
  in
  let focus =
    if preserve
    then Button.Focus.Preserve
    else Focusable (Tab_order.create ~tab_stop:(not skip) () |> ok)
  in
  let config = Button.Config.create ~loading:busy ~focus () in
  let button_style =
    style
      [ Padding (px (Palette.size p 16.))
      ; Radius 12.
      ; Background (Background.solid (Palette.surface p))
      ; Foreground (Palette.foreground p)
      ; Border_color (Palette.accent p)
      ; Opacity (if busy then 0.8 else 1.)
      ]
  in
  let primary =
    if rich
    then
      V.command_button_with_content
        ~key:(Key.of_string_exn "publish-primary")
        ~style:button_style
        ~config
        ~command
        (V.row
           ~style:(style [ Align_items Center; Gap (px 14.) ])
           [ V.loading
               ~style:
                 (style
                    [ Width (px 22.); Height (px 22.); Foreground (Palette.accent p) ])
               ~config:
                 (Loading.Config.create
                    ~kind:Spinner
                    ~label:"Publish activity"
                    ~animated:busy
                    ()
                  |> ok)
               ()
           ; V.column
               ~style:(style [ Gap (px 4.) ])
               [ Palette.text p (if busy then "Publishing draft…" else "Ready to publish")
               ; Palette.text p ~muted:true "A considered update, ready to share"
               ]
           ])
      |> ok
    else
      V.command_button
        ~key:(Key.of_string_exn "publish-primary")
        ~style:button_style
        ~config
        ~command
        ()
  in
  let action_group label button =
    V.column [ button ]
    |> fun view ->
    V.with_accessibility view (Accessibility.create ~role:Group ~label () |> ok) |> ok
  in
  Palette.card
    p
    ~title:"An action with room for detail"
    [ Palette.text
        p
        ~muted:true
        "Keep a clear action while work is in progress. The secondary button shares the \
         same action and remains available."
    ; V.row
        ~style:(style [ Gap (px 16.); Wrap Wrap ])
        [ V.checkbox
            ~state:(Check_state.of_bool busy)
            ~on_toggle:toggle_busy
            "Primary button busy"
        ; V.checkbox
            ~state:(Check_state.of_bool rich)
            ~on_toggle:toggle_rich
            "Detailed button content"
        ; V.checkbox
            ~state:(Check_state.of_bool preserve)
            ~on_toggle:toggle_preserve
            "Preserve existing focus"
        ; V.checkbox
            ~disabled:preserve
            ~state:(Check_state.of_bool skip)
            ~on_toggle:toggle_skip
            "Skip primary with Tab"
        ]
    ; V.command_scope
        ~commands
        [ V.row
            ~style:(style [ Gap (px 16.); Align_items Center; Wrap Wrap ])
            [ action_group "Primary publish action" primary
            ; action_group
                "Secondary publish action"
                (V.command_button
                   ~key:(Key.of_string_exn "publish-secondary")
                   ~command
                   ())
            ]
        ]
    ; Palette.text p (sprintf "Publish requests: %d" count)
    ; Palette.text
        p
        ~muted:true
        "This preview counts requests without publishing anything. Preserving focus is \
         useful for actions next to a text editor; skipping Tab still permits pointer \
         focus."
    ]
;;
