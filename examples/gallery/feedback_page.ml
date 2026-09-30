open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module State = Gpuio_gallery_model.Feedback_state

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn
let id name = Command.Id.of_string name |> ok
let advance = id "advance-preview"
let notify = id "save-preview"
let choose = id "choose-preview-command"
let copy = id "copy-preview-selection"
let shortcut key modifiers = Shortcut.create ~key ~modifiers () |> ok

let menu =
  Menu.create
    ~label:"Preview actions"
    [ Command advance
    ; Command notify
    ; Separator
    ; Submenu (Menu.create ~label:"Editing" [ Command copy ] |> ok)
    ]
  |> ok
;;

let component window palette graph =
  let model, inject =
    B.state_machine0
      ~default_model:State.initial
      ~apply_action:(fun _ model action -> State.apply model action)
      graph
  in
  let chooser, set_chooser = B.state false graph in
  let input =
    Gpuio_eio.Text_input.create
      window
      ~initial_text:"A small idea, ready to share."
      ~config:
        (B.return
           (Text_input.Config.create ~mode:Single_line ~label:"Command preview draft" ()
            |> ok))
      graph
  in
  let open B.Let_syntax in
  B.Edge.lifecycle
    ~on_deactivate:
      (let%arr inject = inject
       and set_chooser = set_chooser in
       Bonsai.Effect.Many [ inject State.Action.Leave; set_chooser false ])
    graph;
  let%arr p = palette
  and model = model
  and inject = inject
  and chooser = chooser
  and set_chooser = set_chooser
  and input = input in
  let button_style =
    style
      [ Padding (px (Palette.size p 10.))
      ; Radius 8.
      ; Foreground (Palette.foreground p)
      ; Background (Background.solid (Palette.border p))
      ]
  in
  let commands =
    Command.Registry.create
      [ Command.create
          ~id:advance
          ~label:"Advance preview"
          ~enabled:(State.is_enabled model)
          ~shortcuts:[ shortcut "k" [ Primary ] ]
          ~on_invoke:(fun () -> inject Advance)
          ()
        |> ok
      ; Command.create
          ~id:notify
          ~label:"Save preview"
          ~on_invoke:(fun () -> inject Notify)
          ()
        |> ok
      ; Command.create
          ~id:choose
          ~label:"Find a command"
          ~shortcuts:[ shortcut "p" [ Primary; Shift ] ]
          ~on_invoke:(fun () -> set_chooser true)
          ()
        |> ok
      ; Command.native ~id:copy ~label:"Copy preview selection" Copy |> ok
      ]
    |> ok
  in
  let value =
    match State.stage model with
    | Idle -> Progress.Value.determinate ~fraction:0. |> ok
    | Working -> Progress.Value.indeterminate
    | Halfway -> Progress.Value.determinate ~fraction:0.5 |> ok
    | Complete -> Progress.Value.determinate ~fraction:1. |> ok
  in
  let notifications =
    Option.to_list (State.notification model)
    |> List.map ~f:(fun serial ->
      V.toast
        ~key:(Key.of_string_exn (sprintf "preview-save-%d" serial))
        ~config:
          (Toast.Config.create
             ~label:"Preview saved"
             ~close_label:"Dismiss saved preview"
             ~timeout:(Toast.Timeout.after (Time_ns.Span.of_sec 5.) |> ok)
             ()
           |> ok)
        ~style:
          (style
             [ Background (Background.solid (Palette.surface p))
             ; Foreground (Palette.foreground p)
             ; Border_width 1.
             ; Border_color (Palette.border p)
             ; Radius 12.
             ; Padding (px 18.)
             ])
        ~on_dismiss:(fun _ -> inject (Dismiss serial))
        [ Palette.text p "Your preview is ready to share."
        ; Palette.text p ~muted:true "A demonstration confirmation."
        ])
  in
  V.command_scope
    ~commands
    ~style:(style [ Gap (px 20.) ])
    [ Palette.card
        p
        ~title:"One action, many ways to reach it"
        [ V.menu_bar ~platform:false [ menu ] |> ok
        ; V.row
            ~style:(style [ Gap (px 12.); Wrap Wrap ])
            [ V.command_button ~style:button_style ~command:advance ()
            ; V.menu_button ~style:button_style ~menu ()
            ; V.command_button ~style:button_style ~command:choose ()
            ]
        ; Palette.text
            p
            ~muted:true
            "Use ⌘K on macOS or Ctrl+K on Linux to advance. Right-click the draft for \
             its menu."
        ; V.context_menu ~menu (Gpuio_eio.Text_input.view input)
        ; V.checkbox
            ~state:(if State.is_enabled model then Checked else Unchecked)
            ~on_toggle:(inject Toggle_enabled)
            "Enable preview command"
        ]
    ; Palette.card
        p
        ~title:"Make waiting understandable"
        [ Palette.text p (State.Stage.label (State.stage model))
        ; V.progress
            ~config:(Progress.Config.create ~label:"Preview progress" ~value |> ok)
            ~style:(style [ Height (px 10.); Foreground (Palette.accent p) ])
            ()
        ; Palette.text
            p
            ~muted:true
            "Advance through ready, working, halfway and complete."
        ]
    ; Palette.card
        p
        ~title:"A quiet confirmation"
        [ V.command_button ~style:button_style ~command:notify ()
        ; Palette.text
            p
            ~muted:true
            "A new save replaces the previous notification. Hover or focus pauses its \
             native expiry."
        ; Palette.text
            p
            (if Option.is_some (State.notification model)
             then "Notification visible"
             else "No pending notification")
        ]
    ; V.toast_stack notifications |> ok
    ; (if chooser
       then
         V.command_palette
           ~config:
             (Command_palette.Config.create
                ~label:"Preview commands"
                ~commands:[ advance; notify; copy ]
                ~placeholder:"Find a preview action…"
                ()
              |> ok)
           ~on_dismiss:(fun _ -> set_chooser false)
           ()
       else V.column [])
    ]
;;
