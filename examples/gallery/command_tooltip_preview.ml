open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module Binding = Command_binding

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn
let key = Key.of_string_exn
let command_id = Command.Id.of_string "gallery.hinted-action" |> ok

let hint observation =
  match observation with
  | None -> "Looking up shortcut…", []
  | Some observation ->
    (match Binding.Observation.state observation with
     | Ready entries ->
       (match
          List.find_map entries ~f:(fun (target, entry) ->
            match target with
            | Binding.Target.Command id when Command.Id.equal id command_id -> Some entry
            | Command _ | Native_action _ -> None)
        with
        | Some (Registry { enabled; candidates }) ->
          let shortcuts =
            List.map candidates ~f:(fun c -> c.Binding.Candidate.shortcut)
          in
          ( (if not enabled
             then "Action disabled"
             else if List.is_empty shortcuts
             then "No shortcut assigned"
             else "Assigned shortcut")
          , shortcuts )
        | Some Missing_command -> "Action not registered", []
        | None | Some (Native_unbound | Native_binding _ | Native_unsupported _) ->
          "Shortcut information unavailable", [])
     | Suspended | Context_gone | Invalid_context | Epoch_exhausted | Capacity ->
       "Shortcut information unavailable", [])
;;

let component palette graph =
  let alternate, toggle_alternate = B.toggle ~default_model:false graph in
  let assigned, toggle_assigned = B.toggle ~default_model:true graph in
  let enabled, toggle_enabled = B.toggle ~default_model:true graph in
  let linux, toggle_linux = B.toggle ~default_model:false graph in
  let count, invoke =
    B.state_machine0 ~default_model:0 ~apply_action:(fun _ count () -> count + 1) graph
  in
  let open B.Let_syntax in
  let observed_button =
    Gpuio_bonsai.Command_binding.component
      ~key:(key "command-hint-observer")
      ~config:
        (B.return
           (Binding.Config.create ~context:Binding.Context.here [ Command command_id ]
            |> ok))
      ~f:(fun observation _ ->
        let%arr p = palette
        and observation = observation
        and linux = linux in
        let description, shortcuts = hint observation in
        let platform = if linux then Shortcut.Platform.Linux else Macos in
        let content =
          V.column
            ~style:(style [ Gap (px 8.); Padding (px 12.) ])
            [ Palette.text p "Run a local preview action"
            ; Palette.text p ~muted:true description
            ; V.row
                ~style:(style [ Gap (px 6.); Wrap Wrap ])
                (List.mapi shortcuts ~f:(fun i shortcut ->
                   Presentation.Kbd.create
                     (Palette.appearance p)
                     ~platform
                     ~key:(key (Int.to_string i))
                     shortcut
                   |> ok))
            ]
        in
        [ V.tooltip
            ~key:(key "command-hint-tooltip")
            ~config:
              (Tooltip.Config.create ~label:"Command shortcut hint" ~width:280. () |> ok)
            ~anchor:
              (V.command_button
                 ~key:(key "hinted-action")
                 ~style:
                   (style
                      [ Padding (px 12.)
                      ; Radius 8.
                      ; Background (Background.solid (Palette.surface p))
                      ; Foreground (Palette.foreground p)
                      ])
                 ~command:command_id
                 ())
            ~content
            ()
        ])
      graph
  in
  let%arr p = palette
  and observed_button = observed_button
  and alternate = alternate
  and toggle_alternate = toggle_alternate
  and assigned = assigned
  and toggle_assigned = toggle_assigned
  and enabled = enabled
  and toggle_enabled = toggle_enabled
  and linux = linux
  and toggle_linux = toggle_linux
  and count = count
  and invoke = invoke in
  let shortcut =
    Shortcut.create ~key:(if alternate then "j" else "h") ~modifiers:[ Primary; Shift ] ()
    |> ok
  in
  let command =
    Command.create
      ~id:command_id
      ~label:"Run hinted action"
      ~enabled
      ~shortcuts:(if assigned then [ shortcut ] else [])
      ~on_invoke:(fun () -> invoke ())
      ()
    |> ok
  in
  let checkbox label checked on_toggle =
    V.checkbox ~state:(Check_state.of_bool checked) ~on_toggle label
  in
  V.command_scope
    ~commands:(Command.Registry.create [ command ] |> ok)
    [ Palette.card
        p
        ~title:"Hints that follow the action"
        [ Palette.text
            p
            ~muted:true
            "Hover or focus the action to see its assigned shortcut. Hints follow the \
             local command registry; the operating system may reserve a shortcut."
        ; V.row
            ~style:(style [ Gap (px 16.); Wrap Wrap ])
            [ checkbox "Assign hinted shortcut" assigned toggle_assigned
            ; checkbox "Alternate hinted shortcut" alternate toggle_alternate
            ; checkbox "Enable hinted action" enabled toggle_enabled
            ; checkbox "Linux hint labels" linux toggle_linux
            ]
        ; observed_button
        ; Palette.text p (sprintf "Hinted action requests: %d" count)
        ]
    ]
;;
