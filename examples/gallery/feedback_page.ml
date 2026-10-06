open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module State = Gpuio_gallery_model.Feedback_state
module Palette_controller = Gpuio_eio.Palette_controller

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
    [ Label "Preview workflow"
    ; Command advance
    ; Command notify
    ; Separator
    ; Submenu
        (Menu.create ~label:"Editing" [ Label "Selection actions"; Command copy ] |> ok)
    ]
  |> ok
;;

let register_menu_icon app scope =
  let source =
    Asset.Source.of_bytes
      ~format:Svg
      {|<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16"><path d="M3 8l3 3 7-7" fill="none" stroke="white" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>|}
    |> ok
  in
  Bonsai.Effect.map (Gpuio_eio.Asset.register app ~scope source) ~f:(function
    | Ok asset ->
      Icon.Config.create
        ~asset:(Gpuio_eio.Asset.handle asset)
        ~description:Image.Description.decorative
        ()
    | Error error -> Error (Error.create_s [%sexp (error : Gpuio_eio.Asset.Error.t)]))
;;

let component ~search_palette app window palette graph =
  let external_palette =
    External_palette_preview.component ~search:search_palette window palette graph
  in
  let menu_icon =
    Preview_scope.acquire
      window
      ~name:"gallery-feedback-menu-artwork"
      ~create:(register_menu_icon app)
      graph
  in
  let show_content, toggle_content = B.toggle ~default_model:true graph in
  let progress_preview = Progress_preview.component window palette graph in
  let model, inject =
    B.state_machine0
      ~default_model:State.initial
      ~apply_action:(fun _ model action -> State.apply model action)
      graph
  in
  let chooser, set_chooser = B.state false graph in
  let palette_controller = Palette_controller.create window graph in
  let palette_command_status, set_palette_command_status = B.state "" graph in
  let palette_help, toggle_palette_help = B.toggle ~default_model:false graph in
  let palette_note =
    Gpuio_eio.Text_input.create
      window
      ~initial_text:"A private note"
      ~config:
        (B.return
           (Text_input.Config.create ~mode:Single_line ~label:"Palette note" () |> ok))
      graph
  in
  let palette_search, next_palette_search =
    B.state_machine0
      ~default_model:0
      ~apply_action:(fun _ index () -> (index + 1) mod 3)
      graph
  in
  let palette_searchable, toggle_palette_searchable =
    B.toggle ~default_model:true graph
  in
  let palette_clear, toggle_palette_clear = B.toggle ~default_model:false graph in
  let toast_anchor, next_toast_anchor =
    B.state_machine0
      ~default_model:0
      ~apply_action:(fun _ index () -> (index + 1) mod 8)
      graph
  in
  let toast_insets, toggle_toast_insets = B.toggle ~default_model:false graph in
  let layered, toggle_layered = B.toggle ~default_model:false graph in
  let toast_motion, toggle_toast_motion = B.toggle ~default_model:false graph in
  let sample_toasts, update_sample_toasts =
    B.state_machine0
      ~default_model:[ 1; 2; 3 ]
      ~apply_action:(fun _ model -> function
         | `Show -> [ 1; 2; 3 ]
         | `Dismiss id -> List.filter model ~f:(fun current -> current <> id))
      graph
  in
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
  and external_palette = external_palette
  and menu_icon = menu_icon
  and show_content = show_content
  and toggle_content = toggle_content
  and model = model
  and inject = inject
  and palette_help = palette_help
  and toggle_palette_help = toggle_palette_help
  and palette_note = palette_note
  and chooser = chooser
  and set_chooser = set_chooser
  and palette_controller = palette_controller
  and palette_command_status = palette_command_status
  and set_palette_command_status = set_palette_command_status
  and palette_search = palette_search
  and next_palette_search = next_palette_search
  and palette_searchable = palette_searchable
  and toggle_palette_searchable = toggle_palette_searchable
  and palette_clear = palette_clear
  and toggle_palette_clear = toggle_palette_clear
  and input = input
  and progress_preview = progress_preview
  and toast_anchor = toast_anchor
  and next_toast_anchor = next_toast_anchor
  and toast_insets = toast_insets
  and toggle_toast_insets = toggle_toast_insets
  and toast_motion = toast_motion
  and toggle_toast_motion = toggle_toast_motion
  and layered = layered
  and toggle_layered = toggle_layered
  and sample_toasts = sample_toasts
  and update_sample_toasts = update_sample_toasts in
  let ((anchor, anchor_label) : Toast.Placement.Anchor.t * string) =
    match toast_anchor with
    | 0 -> Bottom_right, "Bottom right"
    | 1 -> Bottom_center, "Bottom center"
    | 2 -> Bottom_left, "Bottom left"
    | 3 -> Left_center, "Left center"
    | 4 -> Top_left, "Top left"
    | 5 -> Top_center, "Top center"
    | 6 -> Top_right, "Top right"
    | _ -> Right_center, "Right center"
  in
  let placement =
    if toast_insets
    then Toast.Placement.create ~anchor ~top:80. ~right:40. ~bottom:32. ~left:24. ()
    else Toast.Placement.create ~anchor ()
  in
  let toast_config =
    Toast.Stack.create
      ~placement:(placement |> ok)
      ?layering:(Option.some_if layered Toast.Stack.Layering.default)
      ?motion:(Option.some_if toast_motion Toast.Stack.Motion.default)
      ()
    |> ok
  in
  let search, search_label =
    match palette_search with
    | 0 -> Command_palette.Search.All_terms, "All terms"
    | 1 -> Substring, "Substring"
    | _ -> Unfiltered, "Unfiltered"
  in
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
  let notifications =
    if layered
    then
      List.map sample_toasts ~f:(fun serial ->
        let title, text =
          match serial with
          | 1 ->
            "Workspace saved", "Your workspace and open tabs are ready for next time."
          | 2 ->
            ( "Research complete"
            , "Three sources are ready to review. Hover over this stack or Tab to \
               Notifications to expand every message." )
          | _ -> "A new idea is ready", "Open the stack to explore the details."
        in
        V.toast
          ~key:(Key.of_string_exn ("layered-sample-" ^ Int.to_string serial))
          ~config:
            (Toast.Config.create ~label:title ~timeout:Toast.Timeout.persistent () |> ok)
          ~style:
            (style
               [ Background (Background.solid (Palette.surface p))
               ; Foreground (Palette.foreground p)
               ; Border_width 1.
               ; Border_color (Palette.border p)
               ; Radius 12.
               ; Padding (px 18.)
               ])
          ~on_dismiss:(fun _ -> update_sample_toasts (`Dismiss serial))
          [ Palette.text p title; Palette.text p ~muted:true text ])
    else notifications
  in
  let decorate view =
    match menu_icon, show_content with
    | Preview_scope.Ready icon, true ->
      let item text =
        V.row
          ~style:(style [ Gap (px 8.); Align_items Center ])
          [ V.icon
              ~style:
                (style [ Width (px 16.); Height (px 16.); Foreground (Palette.accent p) ])
              icon
          ; V.text text
          ]
      in
      V.with_menu_item_content
        view
        ~items:
          (List.map
             [ [ 0; 1 ], "Advance preview"
             ; [ 0; 2 ], "Save preview"
             ; [ 0; 4 ], "Editing"
             ; [ 0; 4; 1 ], "Copy preview selection"
             ]
             ~f:(fun (path, label) -> Menu.Item_path.of_list path |> ok, item label))
      |> ok
    | _ -> view
  in
  let palette_status =
    match Palette_controller.snapshot palette_controller with
    | None -> "Finding commands…"
    | Some snapshot ->
      let selected =
        match Command_palette.Snapshot.selected snapshot with
        | None -> "No command selected"
        | Some command when Command.Id.equal command advance -> "Advance preview selected"
        | Some command when Command.Id.equal command notify -> "Save preview selected"
        | Some _ -> "Copy selection selected"
      in
      sprintf
        "%d matching commands · %s%s"
        (Command_palette.Snapshot.matched_count snapshot)
        selected
        (if Command_palette.Snapshot.loading snapshot then " · Loading" else "")
  in
  let palette_command command =
    let open Bonsai.Effect.Let_syntax in
    let%bind result = Palette_controller.command palette_controller command in
    set_palette_command_status
      (match result with
       | Ok _ -> "Native command applied"
       | Error error ->
         Sexp.to_string_hum ([%sexp_of: Command_palette.Command_error.t] error))
  in
  let decorate_palette view =
    let items =
      match menu_icon, show_content with
      | Preview_scope.Ready icon, true ->
        let item title detail =
          V.row
            ~style:(style [ Gap (px 10.); Align_items Center; Padding (px 6.) ])
            [ V.icon
                ~style:
                  (style
                     [ Width (px 18.); Height (px 18.); Foreground (Palette.accent p) ])
                icon
            ; V.column
                ~style:(style [ Gap (px 4.) ])
                [ V.text title; V.text ~style:(style [ Font_size 12. ]) detail ]
            ]
        in
        [ advance, item "Advance preview" "Continue to the next step"
        ; notify, item "Save preview" "Keep this idea for later"
        ]
      | _ -> []
    in
    V.with_palette_content
      view
      ~header:
        (V.column
           ~style:(style [ Gap (px 8.) ])
           [ V.text "Workspace commands"; Gpuio_eio.Text_input.view palette_note ])
      ~footer:
        (V.column
           ~style:(style [ Gap (px 8.) ])
           [ V.text ~style:(style [ Font_size 12. ]) palette_status
           ; V.button ~on_click:toggle_palette_help "Palette help"
           ; V.row
               ~style:(style [ Gap (px 8.); Wrap Wrap ])
               [ V.button ~on_click:(palette_command (Set_query "store")) "Find saves"
               ; V.button ~on_click:(palette_command (Set_query "")) "Clear search"
               ; V.button
                   ~on_click:(palette_command (Highlight (Some notify)))
                   "Highlight save"
               ; V.button ~on_click:(palette_command (Highlight None)) "Clear highlight"
               ; V.button ~on_click:(palette_command Focus) "Focus search"
               ; V.button ~on_click:(palette_command (Set_loading true)) "Start loading"
               ; V.button ~on_click:(palette_command (Set_loading false)) "Finish loading"
               ]
           ; V.text palette_command_status
           ; V.text
               (if palette_help
                then "Use keywords to narrow the commands"
                else "Arrow keys to choose · Enter to run")
           ])
      ~empty:
        (V.column
           ~style:(style [ Gap (px 8.); Padding (px 8.) ])
           [ V.text "No matching commands"
           ; V.button ~on_click:toggle_palette_help "Help finding a command"
           ])
      ~items
      ()
    |> ok
  in
  V.command_scope
    ~commands
    ~style:(style [ Gap (px 20.) ])
    [ Palette.card
        p
        ~title:"One action, many ways to reach it"
        [ V.menu_bar ~platform:false [ menu ] |> ok |> decorate
        ; V.row
            ~style:(style [ Gap (px 12.); Wrap Wrap ])
            [ V.command_button ~style:button_style ~command:advance ()
            ; V.menu_button ~style:button_style ~menu () |> decorate
            ; V.command_button ~style:button_style ~command:choose ()
            ]
        ; Palette.text
            p
            ~muted:true
            "Use ⌘K on macOS or Ctrl+K on Linux to advance. Right-click the draft for \
             its menu."
        ; V.context_menu ~menu (Gpuio_eio.Text_input.view input) |> decorate
        ; Palette.button p ("Palette search: " ^ search_label) (next_palette_search ())
        ; V.switch
            ~checked:palette_searchable
            ~on_toggle:toggle_palette_searchable
            "Show command search"
        ; V.switch
            ~checked:palette_clear
            ~on_toggle:toggle_palette_clear
            "Clear command query before closing"
        ; Palette.text
            p
            ~muted:true
            "Try ‘next step’ to find Advance, or ‘store’ to find Save by keyword."
        ; V.switch ~checked:show_content ~on_toggle:toggle_content "Detailed menu items"
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
    ; progress_preview
    ; Palette.card
        p
        ~title:"A quiet confirmation"
        [ V.command_button ~style:button_style ~command:notify ()
        ; Palette.button p ("Placement: " ^ anchor_label) (next_toast_anchor ())
        ; V.switch
            ~checked:toast_insets
            ~on_toggle:toggle_toast_insets
            "Reserve window margins"
        ; V.switch ~checked:layered ~on_toggle:toggle_layered "Layer notification cards"
        ; V.switch
            ~checked:toast_motion
            ~on_toggle:toggle_toast_motion
            "Animate notifications"
        ; (if layered
           then
             Palette.button
               p
               "Show three sample notifications"
               (update_sample_toasts `Show)
           else V.column [])
        ; Palette.text
            p
            ~muted:true
            (if layered
             then
               "Hover or Tab to Notifications to expand. Page Up/Down and Home/End \
                scroll while the stack itself is focused. Sample cards stay until \
                dismissed."
             else
               "A new save replaces the previous notification. Hover or focus pauses its \
                native expiry.")
        ; Palette.text
            p
            (if Option.is_some (State.notification model)
             then "Notification visible"
             else "No pending notification")
        ]
    ; external_palette
    ; V.toast_stack ~config:toast_config notifications |> ok
    ; (if chooser
       then
         V.command_palette
           ~key:(Palette_controller.key palette_controller)
           ~config:
             (Command_palette.Config.create_entries
                ~label:"Preview commands"
                ~search
                ~searchable:palette_searchable
                ~escape:(if palette_clear then Clear_query_first else Dismiss)
                ~keywords:
                  [ advance, [ "next step" ]
                  ; notify, [ "store"; "persist" ]
                  ; copy, [ "clipboard" ]
                  ]
                ~entries:
                  [ Group
                      (Command_palette.Group.create
                         ~id:(Command_palette.Group.Id.of_string "preview" |> ok)
                         ~label:"Preview actions"
                         ~commands:[ advance; notify ]
                         ()
                       |> ok)
                  ; Separator
                  ; Group
                      (Command_palette.Group.create
                         ~id:(Command_palette.Group.Id.of_string "editing" |> ok)
                         ~label:"Editing"
                         ~commands:[ copy ]
                         ()
                       |> ok)
                  ]
                ~placeholder:"Find a preview action…"
                ()
              |> ok)
           ~on_change:(Palette_controller.observe palette_controller)
           ~on_dismiss:(fun _ ->
             Bonsai.Effect.Many
               [ Palette_controller.reset palette_controller
               ; set_palette_command_status ""
               ; set_chooser false
               ])
           ()
         |> decorate_palette
       else V.column [])
    ]
;;
