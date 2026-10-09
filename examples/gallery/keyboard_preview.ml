open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module K = Presentation.Kbd
module Editor = Gpuio_eio.Text_input

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn
let key = Key.of_string_exn

let component window palette graph =
  let linux, toggle_linux = B.toggle ~default_model:false graph in
  let refined, toggle_refined = B.toggle ~default_model:false graph in
  let registered, toggle_registered = B.toggle ~default_model:false graph in
  let enabled, toggle_enabled = B.toggle ~default_model:true graph in
  let shown, toggle_shown = B.toggle ~default_model:true graph in
  let reversed, toggle_reversed = B.toggle ~default_model:false graph in
  let selected, next_key =
    B.state_machine0 ~default_model:0 ~apply_action:(fun _ n () -> (n + 1) % 4) graph
  in
  let actions, invoke =
    B.state_machine0 ~default_model:0 ~apply_action:(fun _ n () -> n + 1) graph
  in
  let editor =
    Editor.create
      window
      ~initial_text:"Type here; keycaps are just labels."
      ~config:
        (B.return
           (Text_input.Config.create ~mode:Single_line ~label:"Keyboard preview draft" ()
            |> ok))
      graph
  in
  let open B.Let_syntax in
  let%arr p = palette
  and linux = linux
  and toggle_linux = toggle_linux
  and refined = refined
  and toggle_refined = toggle_refined
  and registered = registered
  and toggle_registered = toggle_registered
  and enabled = enabled
  and toggle_enabled = toggle_enabled
  and shown = shown
  and toggle_shown = toggle_shown
  and reversed = reversed
  and toggle_reversed = toggle_reversed
  and selected = selected
  and next_key = next_key
  and actions = actions
  and invoke = invoke
  and editor = editor in
  let platform = if linux then Shortcut.Platform.Linux else Macos in
  let key_name = [| "k"; "delete"; "left"; "é" |].(selected) in
  let shortcut =
    Shortcut.create ~key:key_name ~modifiers:[ Primary ] ~priority:Override () |> ok
  in
  let command =
    Command.create
      ~id:(Command.Id.of_string "gallery.keyboard-example" |> ok)
      ~label:"Keyboard example"
      ~shortcuts:[ shortcut ]
      ~enabled
      ~on_invoke:(fun () -> invoke ())
      ()
    |> ok
  in
  let declared = List.hd_exn (Command.shortcuts command) in
  let label name variant =
    V.column
      ~key:(key name)
      ~style:(style [ Gap (px 8.); Font_size 18.; Foreground (Palette.foreground p) ])
      [ Palette.text p ~muted:true name
      ; K.create
          (Palette.appearance p)
          ~platform
          ~key:(key "keycap")
          ~variant
          ?style:
            (Option.some_if refined (style [ Font_size 16.; Padding (px 6.); Radius 2. ]))
          declared
        |> ok
      ]
    |> fun view ->
    V.with_accessibility
      view
      (Accessibility.create ~role:Group ~label:("Keyboard " ^ name ^ " example") () |> ok)
    |> ok
  in
  let labels =
    if shown
    then [ label "filled" Filled; label "outline" Outline; label "plain" Plain ]
    else []
  in
  let labels = if reversed then List.rev labels else labels in
  let checkbox label checked on_toggle =
    V.checkbox ~state:(if checked then Checked else Unchecked) ~on_toggle label
  in
  V.command_scope
    ~commands:(Command.Registry.create (if registered then [ command ] else []) |> ok)
    [ V.column
        ~style:(style [ Gap (px 14.) ])
        [ Palette.text p ~muted:true "The same chord. A platform-aware display."
        ; V.row ~style:(style [ Gap (px 32.); Align_items Center ]) labels
        ; Palette.text
            p
            ~muted:true
            "Display platform changes these labels; command routing always uses this \
             machine's platform."
        ; Editor.view
            ~style:(style [ Height (px 36.); Width (Length.percent_exn 100.) ])
            editor
        ; V.row
            ~style:(style [ Gap (px 12.); Wrap Wrap ])
            [ Palette.button p ("Keyboard key: " ^ key_name) (next_key ())
            ; checkbox "Linux keyboard labels" linux toggle_linux
            ; checkbox "Refine keyboard labels" refined toggle_refined
            ; checkbox "Show keyboard labels" shown toggle_shown
            ; checkbox "Reverse keyboard labels" reversed toggle_reversed
            ]
        ; V.row
            ~style:(style [ Gap (px 12.); Wrap Wrap ])
            [ checkbox "Register example shortcut" registered toggle_registered
            ; checkbox "Enable example command" enabled toggle_enabled
            ]
        ; Palette.text p ~muted:true (sprintf "Keyboard invocations: %d" actions)
        ]
    ]
;;
