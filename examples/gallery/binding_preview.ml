open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module Binding = Command_binding
module Editor = Gpuio_eio.Text_input

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn
let key = Key.of_string_exn
let command_id = Command.Id.of_string "gallery.live-bindings" |> ok

module Mode = struct
  type t =
    | Focused
    | Editor
    | Here
    | Native
  [@@deriving equal]

  let next = function
    | Focused -> Editor
    | Editor -> Here
    | Here -> Native
    | Native -> Focused
  ;;

  let label = function
    | Focused -> "Focused"
    | Editor -> "Editor"
    | Here -> "Here"
    | Native -> "Native"
  ;;
end

let disposition = function
  | Binding.Disposition.Declared -> "Declared"
  | Override -> "Available (override)"
  | Native_first -> "Available after native handling"
  | Widget -> "Native widget binding"
  | Unavailable reason ->
    (match reason with
     | Disabled -> "Disabled"
     | Scope_blocked -> "Outside the active modal scope"
     | Native_unavailable -> "Native action unavailable"
     | Composition -> "Paused during composition"
     | Text_input -> "Reserved for text input"
     | Native_navigation -> "Reserved for native navigation"
     | Conflict id -> "Shadowed by " ^ Command.Id.to_string id)
;;

let entry p platform (target, value) =
  let name =
    match target with
    | Binding.Target.Command _ -> "Preview action"
    | Native_action _ -> "Copy"
  in
  let description, caps =
    match value with
    | Binding.Entry.Missing_command -> "Not registered", []
    | Native_unbound -> "No binding in this context", []
    | Native_unsupported Sequence_too_long -> "Sequence exceeds the display limit", []
    | Native_unsupported Invalid_stroke -> "Unsupported native key", []
    | Registry { enabled; candidates } ->
      let description =
        if not enabled
        then "Disabled"
        else
          List.map candidates ~f:(fun c -> disposition c.Binding.Candidate.disposition)
          |> String.concat ~sep:" · "
      in
      ( description
      , List.mapi candidates ~f:(fun i candidate ->
          Presentation.Kbd.create
            (Palette.appearance p)
            ~platform
            ~key:(key (Int.to_string i))
            candidate.shortcut
          |> ok) )
    | Native_binding { strokes; disposition = status } ->
      ( disposition status
      , List.mapi strokes ~f:(fun i stroke ->
          Presentation.Kbd.of_native_stroke
            (Palette.appearance p)
            ~platform
            ~key:(key (Int.to_string i))
            stroke
          |> ok) )
  in
  V.row
    ~key:(key name)
    ~style:(style [ Gap (px 12.); Align_items Center; Wrap Wrap ])
    (Palette.text p (name ^ ": " ^ description) :: caps)
  |> fun view ->
  V.with_accessibility
    view
    (Accessibility.create ~role:Group ~label:("Live binding " ^ name) () |> ok)
  |> ok
;;

let observed p platform = function
  | None -> [ Palette.text p ~muted:true "Waiting for native bindings" ]
  | Some observation ->
    let rows =
      match Binding.Observation.state observation with
      | Ready entries -> List.map entries ~f:(entry p platform)
      | Suspended -> [ Palette.text p "Binding observer is suspended" ]
      | Context_gone -> [ Palette.text p "The observed editor was removed" ]
      | Invalid_context -> [ Palette.text p "Invalid native context" ]
      | Epoch_exhausted -> [ Palette.text p "Remount to restart binding observations" ]
      | Capacity -> [ Palette.text p "Binding query reached the native work limit" ]
    in
    rows
    @ [ Palette.text
          p
          ~muted:true
          (sprintf "Binding sample: %Ld" (Binding.Observation.epoch observation))
      ]
;;

let component window palette graph =
  let mode, next_mode =
    B.state_machine0
      ~default_model:Mode.Focused
      ~apply_action:(fun _ mode () -> Mode.next mode)
      graph
  in
  let shown, toggle_shown = B.toggle ~default_model:true graph in
  let registered, toggle_registered = B.toggle ~default_model:true graph in
  let enabled, toggle_enabled = B.toggle ~default_model:true graph in
  let alternate, toggle_alternate = B.toggle ~default_model:false graph in
  let invalid, toggle_invalid = B.toggle ~default_model:false graph in
  let linux, toggle_linux = B.toggle ~default_model:false graph in
  let actions, invoke =
    B.state_machine0 ~default_model:0 ~apply_action:(fun _ n () -> n + 1) graph
  in
  let editor =
    Editor.create
      window
      ~initial_text:"Focus me to discover native Copy."
      ~config:
        (B.return
           (Text_input.Config.create ~mode:Single_line ~label:"Live binding draft" ()
            |> ok))
      graph
  in
  let open B.Let_syntax in
  let config =
    let%arr mode = mode
    and editor = editor
    and invalid = invalid in
    let context =
      match mode with
      | Mode.Focused -> Some Binding.Context.focused
      | Here -> Some Binding.Context.here
      | Editor -> Option.map (Editor.snapshot editor) ~f:Binding.Context.editor
      | Native ->
        Some
          (Binding.Context.native_context (if invalid then "Input !" else "Input") |> ok)
    in
    Option.map context ~f:(fun context ->
      let targets =
        match mode with
        | Here -> [ Binding.Target.Command command_id ]
        | Native -> [ Binding.Target.Native_action Copy ]
        | Focused | Editor -> [ Binding.Target.Command command_id; Native_action Copy ]
      in
      Binding.Config.create ~context targets |> ok)
  in
  let query =
    match%sub shown with
    | false ->
      B.map palette ~f:(fun p -> Palette.text p ~muted:true "Live bindings hidden")
    | true ->
      (match%sub config with
       | None ->
         B.map palette ~f:(fun p -> Palette.text p ~muted:true "Waiting for the editor")
       | Some config ->
         Gpuio_bonsai.Command_binding.component
           ~key:(key "live-query")
           ~config
           ~style:(B.return (style [ Gap (px 10.) ]))
           ~f:(fun observation _ ->
             let%arr observation = observation
             and p = palette
             and linux = linux in
             observed p (if linux then Shortcut.Platform.Linux else Macos) observation)
           graph)
  in
  let%arr p = palette
  and editor = editor
  and query = query
  and mode = mode
  and next_mode = next_mode
  and shown = shown
  and toggle_shown = toggle_shown
  and registered = registered
  and toggle_registered = toggle_registered
  and enabled = enabled
  and toggle_enabled = toggle_enabled
  and alternate = alternate
  and toggle_alternate = toggle_alternate
  and invalid = invalid
  and toggle_invalid = toggle_invalid
  and linux = linux
  and toggle_linux = toggle_linux
  and actions = actions
  and invoke = invoke in
  let shortcut =
    Shortcut.create
      ~key:(if alternate then "l" else "k")
      ~modifiers:[ Primary ]
      ~priority:Override
      ()
    |> ok
  in
  let command =
    Command.create
      ~id:command_id
      ~label:"Live preview action"
      ~shortcuts:[ shortcut ]
      ~enabled
      ~on_invoke:(fun () -> invoke ())
      ()
    |> ok
  in
  let checkbox label checked on_toggle =
    V.checkbox ~state:(if checked then Checked else Unchecked) ~on_toggle label
  in
  V.command_scope
    ~commands:(Command.Registry.create (if registered then [ command ] else []) |> ok)
    [ V.column
        ~style:(style [ Gap (px 14.) ])
        [ Palette.text p ~muted:true "Shortcuts that follow your context."
        ; Editor.view
            ~style:(style [ Height (px 36.); Width (Length.percent_exn 100.) ])
            editor
        ; query
        ; V.row
            ~style:(style [ Gap (px 12.); Wrap Wrap ])
            [ Palette.button p ("Binding context: " ^ Mode.label mode) (next_mode ())
            ; Palette.button p "Run preview action" (invoke ())
            ; checkbox "Show live bindings" shown toggle_shown
            ]
        ; V.row
            ~style:(style [ Gap (px 12.); Wrap Wrap ])
            [ checkbox "Register live shortcut" registered toggle_registered
            ; checkbox "Enable live shortcut" enabled toggle_enabled
            ; checkbox "Use alternate live shortcut" alternate toggle_alternate
            ]
        ; V.row
            ~style:(style [ Gap (px 12.); Wrap Wrap ])
            [ checkbox "Linux live labels" linux toggle_linux
            ; checkbox "Invalid native facts" invalid toggle_invalid
            ]
        ; Palette.text p ~muted:true (sprintf "Live invocations: %d" actions)
        ; Palette.text
            p
            ~muted:true
            "Focus follows the active control. Editor and native contexts describe \
             bindings without moving focus."
        ]
    ]
;;
