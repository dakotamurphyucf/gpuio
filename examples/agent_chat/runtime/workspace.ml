open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module App = Gpuio_eio.App
module Scope = Gpuio_eio.Scope
module Editor = Gpuio_eio.Text_input
module Input = Gpuio.Text_input
module View = Gpuio_bonsai.View
module List_view = Gpuio_bonsai.Virtual_list
module Pager = Gpuio_eio.List_paging
module Tabs = Gpuio.Workspace
module Command = Gpuio.Command
module Backend = Conversation.Backend

module Panel = struct
  type t =
    { editor : Editor.t
    ; list : int List_view.Output.t
    }
end

type t =
  { icons : Icons.t
  ; conversations : Conversation.t list
  ; tabs : int Tabs.t B.Expert.Var.t
  ; dark : bool B.Expert.Var.t
  ; notice : string B.Expert.Var.t
  ; backend : Backend.Config.t B.Expert.Var.t
  ; palette : bool B.Expert.Var.t
  ; demo_controls : bool B.Expert.Var.t
  ; close_pending : bool B.Expert.Var.t
  ; mutable close_answer : (Gpuio.Window.Close_decision.t -> unit) option
  ; panels : (int, Panel.t) Hashtbl.t
  }

let px = Gpuio.Length.px_exn
let full = Gpuio.Length.percent_exn 100.
let style = Gpuio.Style.create_exn
let key = Gpuio.Key.of_string_exn
let tab_id id = Tabs.Id.of_string (Int.to_string id) |> Or_error.ok_exn

let tab conversation =
  Tabs.Tab.create
    ~id:(tab_id (Conversation.id conversation))
    ~label:(Conversation.title conversation)
    (Conversation.id conversation)
  |> Or_error.ok_exn
;;

let find t id = List.find_exn t.conversations ~f:(fun c -> Conversation.id c = id)

let create ~icons conversations ~selected =
  let chosen = List.find_exn conversations ~f:(fun c -> Conversation.id c = selected) in
  { icons
  ; conversations
  ; tabs = B.Expert.Var.create (Tabs.create [ tab chosen ] |> Or_error.ok_exn)
  ; dark = B.Expert.Var.create true
  ; notice = B.Expert.Var.create "Local demo · no network or credentials"
  ; backend = B.Expert.Var.create (Backend.Config.create () |> Or_error.ok_exn)
  ; palette = B.Expert.Var.create false
  ; demo_controls = B.Expert.Var.create false
  ; close_pending = B.Expert.Var.create false
  ; close_answer = None
  ; panels = Int.Table.create ()
  }
;;

let select t id =
  let previous = B.Expert.Var.get t.tabs in
  let next =
    match Tabs.find previous (tab_id id) with
    | Some _ -> Tabs.select previous (tab_id id)
    | None -> Tabs.add previous (tab (find t id))
  in
  B.Expert.Var.set t.tabs (Or_error.ok_exn next)
;;

let close_tab t id =
  let remaining, _ =
    Tabs.remove (B.Expert.Var.get t.tabs) (tab_id id) |> Or_error.ok_exn
  in
  B.Expert.Var.set t.tabs remaining
;;

let panel t id = Hashtbl.find t.panels id
let dark t = B.Expert.Var.get t.dark
let toggle_theme t = B.Expert.Var.set t.dark (not (dark t))
let set_backend t value = B.Expert.Var.set t.backend value
let notice t = B.Expert.Var.get t.notice
let notify t text = B.Expert.Var.set t.notice text

let report t result =
  Result.iter_error result ~f:(fun error -> notify t (Error.to_string_hum error))
;;

let color dark light_value dark_value =
  Gpuio.Color.rgb_exn (if dark then dark_value else light_value)
;;

let background dark = (Palette.of_dark dark).canvas |> Gpuio.Background.solid
let surface dark = (Palette.of_dark dark).surface |> Gpuio.Background.solid
let foreground dark = (Palette.of_dark dark).text
let muted dark = (Palette.of_dark dark).muted
let solid = Gpuio.Background.solid
let compose a b = Gpuio.Style.merge [ a; b ]

let button_style dark =
  let p = Palette.of_dark dark in
  style
    [ Padding_top (px 7.)
    ; Padding_bottom (px 7.)
    ; Padding_left (px 10.)
    ; Padding_right (px 10.)
    ; Radius 7.
    ; Border_width 0.
    ; Background (solid p.sidebar)
    ; Foreground p.muted
    ; Font_size 12.
    ; Font_weight 500
    ; Gap (px 7.)
    ; Shrink 0.
    ]
  |> fun t ->
  Gpuio.Style.with_state_exn t Hovered [ Background (solid p.raised); Foreground p.text ]
  |> fun t ->
  Gpuio.Style.with_state_exn t Focused [ Border_width 1.; Border_color p.accent ]
;;

let button dark ?disabled ?(icons = []) ?icon label on_click =
  View.button
    ?disabled
    ~style:(button_style dark)
    ?leading_icon:(Option.bind icon ~f:(Icons.decoration icons))
    ~on_click
    label
;;

let icon_button dark icons icon ~label on_click =
  let p = Palette.of_dark dark in
  let anchor =
    match Icons.decoration icons icon with
    | None -> button dark label on_click
    | Some decoration ->
      View.icon_button ~style:(button_style dark) ~label ~on_click decoration
  in
  View.tooltip
    ~style:
      (style
         [ Background (solid p.raised)
         ; Foreground p.text
         ; Font_size 11.
         ; Padding (px 7.)
         ; Radius 6.
         ; Border_width 1.
         ; Border_color p.line
         ])
    ~config:
      (Gpuio.Tooltip.Config.create ~label ~width:150. ~hoverable:false ()
       |> Or_error.ok_exn)
    ~anchor
    ~content:(View.text label)
    ()
;;

let caption p text =
  View.text ~style:(style [ Font_size 11.; Foreground p.Palette.muted ]) text
;;

let dot p =
  View.column
    ~style:
      (style
         [ Width (px 6.)
         ; Height (px 6.)
         ; Radius 3.
         ; Background (solid p.Palette.success)
         ; Shrink 0.
         ])
    []
;;

let spacer = View.column ~style:(style [ Grow 1. ]) []
let action f = E.of_thunk f

let config label mode =
  Input.Config.create
    ~mode
    ~label
    ~placeholder:"Ask anything, or describe what you want to build…"
    ~min_rows:2
    ~max_rows:4
    ()
  |> Or_error.ok_exn
;;

let list_config =
  List_view.Config.create
    ~height:(Estimated 160.)
    ~overscan:240.
    ~max_active:32
    ~scroll:Follow_tail_when_at_end
    ()
  |> Or_error.ok_exn
;;

let submit t window conversation submission =
  action (fun () ->
    match panel t (Conversation.id conversation) with
    | None -> notify t "Composer is not ready."
    | Some panel ->
      let accepted =
        let open E.Let_syntax in
        let%bind result = Editor.clear_if_unchanged panel.editor submission in
        action (fun () ->
          match result with
          | Ok _ -> notify t "Sent"
          | Error (Stale_revision | Stale_editor | Closed) ->
            notify t "Sent · your newer draft was kept"
          | Error error ->
            notify t (Sexp.to_string_hum (Input.Command_error.sexp_of_t error)))
      in
      Conversation.submit
        conversation
        ~window_scope:(App.Window.scope window)
        ~config:(B.Expert.Var.get t.backend)
        ~prompt:(Input.Submission.text submission)
        ~on_accept:accepted
      |> report t)
;;

let send t editor =
  let open E.Let_syntax in
  let%bind result = Editor.submit editor in
  action (fun () ->
    Result.iter_error result ~f:(fun error ->
      notify t (Sexp.to_string_hum (Input.Command_error.sexp_of_t error))))
;;

let attach t ~read_file ~attachment_directory window conversation =
  let open E.Let_syntax in
  let%bind selected =
    Gpuio_eio.File_dialog.open_
      window
      ~config:
        (Gpuio_eio.File_dialog.Open.create
           ?directory:attachment_directory
           ~title:"Attach a text file"
           ()
         |> Or_error.ok_exn)
  in
  match selected with
  | Ok None -> E.Ignore
  | Error error ->
    action (fun () ->
      notify t (Sexp.to_string_hum (Gpuio.File_dialog.Error.sexp_of_t error)))
  | Ok (Some [ path ]) ->
    action (fun () ->
      Scope.start
        (App.Window.scope window)
        ~f:(fun () -> read_file path)
        ~on_result:(fun result ->
          match result with
          | Error error -> action (fun () -> notify t (Error.to_string_hum error))
          | Ok text ->
            let bytes = Gpuio.File_path.to_string path in
            let name =
              if Stdlib.String.is_valid_utf_8 bytes
              then Filename.basename bytes
              else "Text attachment"
            in
            let%bind result = Conversation.attach conversation ~name ~text in
            action (fun () ->
              match result with
              | Ok () -> notify t ("Attached " ^ name)
              | Error error -> notify t (Error.to_string_hum error)))
      |> Result.map ~f:(fun (_ : Scope.Task.t) -> ())
      |> report t)
  | Ok (Some ([] | _ :: _ :: _)) -> action (fun () -> notify t "Select one text file.")
;;

let document_view t dark message =
  match message.Conversation.Message.body with
  | Plain text -> View.text ~style:(style [ White_space Normal; User_select true ]) text
  | Rich (document, mode) ->
    let config =
      Gpuio.Document.Config.create
        ~source:(Gpuio_eio.Document.handle document)
        ~mode
        ~appearance:(if dark then Dark else Light)
        ~label:message.detail
        ~initially_collapsed:
          (match mode with
           | Markdown -> false
           | Code _ | Diff -> true)
        ~layout:Flow
        ()
      |> Or_error.ok_exn
    in
    View.document
      ~style:(style [ Font_size 14.; Line_height (px 23.) ])
      ~on_navigate:(fun navigation ->
        action (fun () ->
          notify
            t
            (match navigation with
             | Link url -> "Link: " ^ url
             | Line { path; line; _ } ->
               sprintf "%s:%d" (Option.value path ~default:"Document") line)))
      config
;;

let message_view t dark icons message =
  let p = Palette.of_dark dark in
  let user = String.equal message.Conversation.Message.author "You" in
  let artifact =
    match message.body with
    | Rich (_, (Code _ | Diff)) -> true
    | Rich (_, Markdown) | Plain _ -> false
  in
  let avatar =
    View.column
      ~style:
        (style
           [ Width (px (if artifact then 22. else 27.))
           ; Height (px (if artifact then 22. else 27.))
           ; Shrink 0.
           ; Radius 8.
           ; Align_items Center
           ; Justify_content Center
           ; Background (solid (if user then p.raised else p.accent_surface))
           ; Foreground p.accent
           ; Font_size 11.
           ; Font_weight 600
           ])
      [ (if user
         then View.text "Y"
         else Icons.view icons (if artifact then Code else Spark))
      ]
  in
  View.column
    ~style:
      (style
         [ Width full
         ; Align_items Center
         ; Padding_left (px 28.)
         ; Padding_right (px 28.)
         ; Padding_top (px 8.)
         ; Padding_bottom (px 8.)
         ])
    [ View.column
        ~style:
          (style
             [ Width full
             ; Max_width (px 760.)
             ; Gap (px (if artifact then 6. else 12.))
             ; Padding (px (if artifact then 12. else 6.))
             ; Background (solid (if artifact then p.surface else p.canvas))
             ; Radius 12.
             ; Border_width (if artifact then 1. else 0.)
             ; Border_color p.line
             ; Foreground p.text
             ])
        [ View.row
            ~style:(style [ Align_items Center; Gap (px 9.) ])
            [ avatar
            ; View.text
                ~style:(style [ Font_size 12.; Font_weight 600 ])
                (if user then "You" else if artifact then "Workspace" else "GPUIO")
            ; caption
                p
                (if artifact
                 then "Artifact · ready to review"
                 else if user
                 then "Just now"
                 else "Local assistant")
            ; spacer
            ; (if artifact
               then
                 View.row
                   ~style:
                     (style
                        [ Gap (px 4.)
                        ; Align_items Center
                        ; Foreground p.success
                        ; Font_size 10.
                        ])
                   [ Icons.view icons Check; View.text "Ready" ]
               else View.text "")
            ]
        ; document_view t dark message
        ]
    ]
;;

let conversation_panel t ~read_file ~attachment_directory window conversation graph =
  let dark = B.Expert.Var.value t.dark in
  let icons = Icons.value t.icons in
  let phase = Conversation.phase_value conversation in
  let snapshot = Pager.value (Conversation.pager conversation) in
  let editor =
    Editor.create
      window
      ~config:
        (B.return (config ("Message · " ^ Conversation.title conversation) Multiline))
      ~on_submit:(B.return (submit t window conversation))
      graph
  in
  let list =
    List_view.paged
      (module Int)
      snapshot
      ~paging:(B.return (Pager.controls (Conversation.pager conversation)))
      ~row_key:(fun id -> key (Int.to_string id))
      ~config:list_config
      ~style:(B.return (style [ Grow 1.; Basis (px 0.); Min_height (px 0.); Width full ]))
      ~render_row:(fun ~key:_ ~data ~lifetime:_ _graph ->
        let open B.Let_syntax in
        let%arr message = data
        and dark = dark
        and icons = icons in
        message_view t dark icons message)
      graph
  in
  let open B.Let_syntax in
  B.Edge.after_display
    (let%arr editor = editor
     and list = list in
     action (fun () ->
       Hashtbl.set
         t.panels
         ~key:(Conversation.id conversation)
         ~data:{ Panel.editor; list = Or_error.ok_exn list }))
    graph;
  let%arr editor = editor
  and list = list
  and phase = phase
  and snapshot = snapshot
  and dark = dark
  and icons = icons in
  let list = Or_error.ok_exn list in
  let busy =
    match phase with
    | Conversation.Phase.Accepting | Streaming -> true
    | Idle | Complete | Cancelled | Failed _ -> false
  in
  let status =
    match phase with
    | Idle -> "Ready"
    | Accepting -> "Sending…"
    | Streaming -> "Responding…"
    | Complete -> "Complete"
    | Cancelled -> "Cancelled"
    | Failed _ -> "Interrupted"
  in
  let p = Palette.of_dark dark in
  let send_style =
    compose
      (button_style dark)
      (style
         [ Background (solid p.accent)
         ; Foreground p.accent_ink
         ; Font_weight 600
         ; Padding_left (px 14.)
         ; Padding_right (px 12.)
         ; Radius 8.
         ])
  in
  View.column
    ~style:
      (style
         [ Height full
         ; Width full
         ; Min_width (px 0.)
         ; Min_height (px 0.)
         ; Foreground p.text
         ])
    [ View.row
        ~style:
          (style
             [ Padding_left (px 32.)
             ; Padding_right (px 24.)
             ; Padding_top (px 23.)
             ; Padding_bottom (px 20.)
             ; Align_items Center
             ; Gap (px 10.)
             ; Shrink 0.
             ; Border_bottom_width 1.
             ; Border_color p.line
             ])
        [ View.column
            ~style:(style [ Grow 1.; Gap (px 6.) ])
            [ View.text
                ~style:(style [ Font_size 22.; Font_weight 600 ])
                (Conversation.title conversation)
            ; View.row
                ~style:(style [ Gap (px 7.); Align_items Center ])
                [ (if busy
                   then
                     View.progress
                       ~style:
                         (style
                            [ Width (px 28.)
                            ; Height (px 3.)
                            ; Foreground p.accent
                            ; Background (solid p.raised)
                            ])
                       ~config:
                         (Gpuio.Progress.Config.create
                            ~label:"Generating response"
                            ~value:Gpuio.Progress.Value.indeterminate
                          |> Or_error.ok_exn)
                       ()
                   else dot p)
                ; caption
                    p
                    (sprintf
                       "%d messages · %s"
                       (Gpuio.List_collection.length snapshot.items)
                       status)
                ]
            ]
        ; icon_button
            dark
            icons
            History
            ~label:"Load older"
            (action (fun () ->
               Pager.request (Conversation.pager conversation) Before |> report t))
        ; icon_button
            dark
            icons
            Arrow_down
            ~label:"Latest"
            (List_view.Controller.jump_to_latest (List_view.Output.controller list))
        ]
    ; List_view.Output.view list
    ; (match phase with
       | Failed error ->
         View.row
           ~style:
             (style
                [ Margin_left (px 28.)
                ; Margin_right (px 28.)
                ; Margin_top (px 8.)
                ; Padding (px 10.)
                ; Radius 8.
                ; Shrink 0.
                ; Background (solid p.accent_surface)
                ; Foreground p.accent
                ; Font_size 12.
                ])
           [ View.text ~style:(style [ White_space Normal ]) error ]
       | Idle | Accepting | Streaming | Complete | Cancelled -> View.column [])
    ; View.column
        ~style:
          (style
             [ Width full
             ; Align_items Center
             ; Padding_left (px 28.)
             ; Padding_right (px 28.)
             ; Padding_top (px 12.)
             ; Padding_bottom (px 15.)
             ; Shrink 0.
             ; Gap (px 10.)
             ])
        [ View.column
            ~style:
              (style
                 [ Width full
                 ; Max_width (px 760.)
                 ; Radius 16.
                 ; Background (solid p.surface)
                 ; Border_width 1.
                 ; Border_color p.line
                 ; Padding (px 10.)
                 ; Gap (px 8.)
                 ])
            [ Editor.view
                ~style:
                  (style
                     [ Width full
                     ; Height (px 70.)
                     ; Shrink 0.
                     ; Padding (px 8.)
                     ; Border_width 0.
                     ; Font_size 14.
                     ; Background (solid p.surface)
                     ; Foreground p.text
                     ])
                editor
            ; View.row
                ~style:(style [ Gap (px 6.); Align_items Center; Width full ])
                [ icon_button
                    dark
                    icons
                    Paperclip
                    ~label:"Attach text…"
                    (attach t ~read_file ~attachment_directory window conversation)
                ; View.row
                    ~style:
                      (style [ Gap (px 6.); Align_items Center; Padding_left (px 4.) ])
                    [ dot p; caption p "Local assistant" ]
                ; spacer
                ; (if busy
                   then
                     button
                       dark
                       ~icons
                       ~icon:Stop
                       "Cancel"
                       (action (fun () -> Conversation.cancel conversation))
                   else if Option.is_some (Conversation.last_document conversation)
                   then
                     icon_button
                       dark
                       icons
                       Retry
                       ~label:"Retry response"
                       (action (fun () ->
                          Conversation.retry
                            conversation
                            ~config:(Backend.Config.create () |> Or_error.ok_exn)
                          |> report t))
                   else View.text "")
                ; View.button
                    ~disabled:busy
                    ~style:send_style
                    ?trailing_icon:(Icons.decoration icons Arrow_up)
                    ~on_click:(send t editor)
                    "Send"
                ]
            ]
        ; caption p "Enter to send · Shift-Enter for a new line"
        ]
    ]
;;

let answer_close t answer =
  let callback = t.close_answer in
  t.close_answer <- None;
  B.Expert.Var.set t.close_pending false;
  Option.iter callback ~f:(fun callback -> callback answer)
;;

let install_close_handler t window =
  App.Window.set_close_handler window (fun _reason ->
    let open E.Let_syntax in
    let%bind snapshots =
      E.all
        (List.map (Hashtbl.data t.panels) ~f:(fun panel ->
           Editor.read_snapshot panel.editor))
    in
    let drafts =
      List.exists snapshots ~f:(function
        | Ok snapshot ->
          (not (String.is_empty (Input.Snapshot.text snapshot)))
          || Option.is_some (Input.Snapshot.composition snapshot)
        | Error (Closed | Stale_editor | Not_mounted) -> false
        | Error
            ( Stale_revision
            | Composing
            | Invalid_selection
            | Limit_exceeded
            | Busy
            | Native_failure
            | Invalid_text
            | Focus_blocked ) -> true)
    in
    if not drafts
    then E.return Gpuio.Window.Close_decision.Allow
    else
      E.Expert.of_fun ~f:(fun ~callback ->
        t.close_answer <- Some callback;
        B.Expert.Var.set t.close_pending true));
  Scope.Expert.on_cancel (App.Window.scope window) (fun () ->
    t.close_answer <- None;
    Hashtbl.clear t.panels)
  |> Or_error.ok_exn
  |> fun (_ : unit -> unit) -> ()
;;

let command_id text = Command.Id.of_string text |> Or_error.ok_exn
let shortcut key modifiers = Gpuio.Shortcut.create ~key ~modifiers () |> Or_error.ok_exn

let component t ~open_window ~read_file ~attachment_directory window graph =
  let search =
    Editor.create
      window
      ~config:
        (B.return
           (Input.Config.create
              ~mode:Single_line
              ~label:"Find conversations"
              ~placeholder:"Find conversations…"
              ()
            |> Or_error.ok_exn))
      graph
  in
  let panels =
    List.map t.conversations ~f:(fun conversation ->
      let view =
        conversation_panel t ~read_file ~attachment_directory window conversation graph
      in
      B.map view ~f:(fun view -> Conversation.id conversation, view))
    |> B.all
  in
  let open B.Let_syntax in
  let%arr tabs = B.Expert.Var.value t.tabs
  and dark = B.Expert.Var.value t.dark
  and notice = B.Expert.Var.value t.notice
  and palette = B.Expert.Var.value t.palette
  and demo_controls = B.Expert.Var.value t.demo_controls
  and icons = Icons.value t.icons
  and pending_close = B.Expert.Var.value t.close_pending
  and search = search
  and panels = panels in
  let p = Palette.of_dark dark in
  let active = Option.map (Tabs.active tabs) ~f:Tabs.Tab.data in
  let selected =
    Option.value active ~default:(Conversation.id (List.hd_exn t.conversations))
  in
  let make_command ?(shortcuts = []) name label f =
    Command.create
      ~id:(command_id name)
      ~label
      ~shortcuts
      ~on_invoke:(fun () -> action f)
      ()
    |> Or_error.ok_exn
  in
  let commands =
    Command.Registry.create
      [ make_command
          ~shortcuts:[ shortcut "p" [ Primary; Shift ] ]
          "commands"
          "Open commands"
          (fun () -> B.Expert.Var.set t.palette true)
      ; make_command
          ~shortcuts:[ shortcut "n" [ Primary ] ]
          "new-window"
          "Open conversation in another window"
          (fun () -> open_window selected)
      ; make_command
          ~shortcuts:[ shortcut "]" [ Primary; Shift ] ]
          "next-tab"
          "Next conversation tab"
          (fun () -> B.Expert.Var.set t.tabs (Tabs.next (B.Expert.Var.get t.tabs)))
      ; make_command
          ~shortcuts:[ shortcut "[" [ Primary; Shift ] ]
          "previous-tab"
          "Previous conversation tab"
          (fun () -> B.Expert.Var.set t.tabs (Tabs.previous (B.Expert.Var.get t.tabs)))
      ; make_command "theme" "Toggle light/dark theme" (fun () -> toggle_theme t)
      ; make_command
          ~shortcuts:[ shortcut "w" [ Primary ] ]
          "close-tab"
          "Close conversation tab"
          (fun () -> Option.iter active ~f:(close_tab t))
      ; make_command "close-window" "Close window" (fun () ->
          App.Window.request_close window)
      ; Command.native ~id:(command_id "copy") ~label:"Copy selection" Copy
        |> Or_error.ok_exn
      ]
    |> Or_error.ok_exn
  in
  let menu =
    Gpuio.Menu.create
      ~label:"Workspace"
      [ Command (command_id "new-window")
      ; Command (command_id "next-tab")
      ; Command (command_id "previous-tab")
      ; Separator
      ; Command (command_id "theme")
      ; Command (command_id "copy")
      ; Separator
      ; Command (command_id "close-tab")
      ; Command (command_id "close-window")
      ]
    |> Or_error.ok_exn
  in
  let query =
    Editor.snapshot search
    |> Option.value_map ~default:"" ~f:Input.Snapshot.text
    |> String.lowercase
  in
  let sidebar =
    View.column
      ~style:
        (style
           [ Width full
           ; Height full
           ; Padding (px 18.)
           ; Gap (px 20.)
           ; Background (solid p.sidebar)
           ; Foreground p.text
           ])
      ([ View.row
           ~style:
             (style
                [ Gap (px 10.)
                ; Align_items Center
                ; Padding_top (px 6.)
                ; Padding_bottom (px 8.)
                ])
           [ View.column
               ~style:
                 (style
                    [ Width (px 34.)
                    ; Height (px 34.)
                    ; Radius 10.
                    ; Background (solid p.accent_surface)
                    ; Foreground p.accent
                    ; Align_items Center
                    ; Justify_content Center
                    ])
               [ Icons.view icons Spark ]
           ; View.text ~style:(style [ Font_size 18.; Font_weight 650 ]) "GPUIO"
           ; caption p "STUDIO"
           ]
       ; View.row
           ~style:
             (style
                [ Width full
                ; Height (px 34.)
                ; Padding_left (px 9.)
                ; Gap (px 7.)
                ; Align_items Center
                ; Background (solid p.canvas)
                ; Border_width 1.
                ; Border_color p.line
                ; Radius 7.
                ; Foreground p.muted
                ])
           [ Icons.view icons Search
           ; Editor.view
               ~style:
                 (style
                    [ Grow 1.
                    ; Basis (px 0.)
                    ; Min_width (px 0.)
                    ; Height (px 30.)
                    ; Font_size 12.
                    ; Foreground p.text
                    ])
               search
           ]
       ; View.row
           ~style:(style [ Align_items Center; Padding_top (px 6.) ])
           [ caption p "YOUR CONVERSATIONS"; spacer; Icons.view icons Message ]
       ; View.column
           ~style:(style [ Gap (px 5.); Width full ])
           (List.filter_map t.conversations ~f:(fun conversation ->
              if
                String.is_substring
                  (String.lowercase (Conversation.title conversation))
                  ~substring:query
              then (
                let chosen =
                  Option.exists active ~f:(Int.equal (Conversation.id conversation))
                in
                Some
                  (View.button
                     ~style:
                       (compose
                          (button_style dark)
                          (style
                             [ Width full
                             ; Padding (px 10.)
                             ; Font_size 12.
                             ; Gap (px 9.)
                             ; Justify_content Start
                             ; Background
                                 (solid (if chosen then p.accent_surface else p.sidebar))
                             ; Foreground (if chosen then p.accent else p.muted)
                             ]))
                     ?leading_icon:(Icons.decoration icons Message)
                     ~on_click:
                       (action (fun () -> select t (Conversation.id conversation)))
                     (Conversation.title conversation)))
              else None))
       ; spacer
       ; View.column
           ~style:
             (style
                [ Gap (px 10.)
                ; Padding (px 12.)
                ; Radius 10.
                ; Border_width 1.
                ; Border_color p.line
                ])
           [ View.row
               ~style:(style [ Gap (px 7.); Align_items Center ])
               [ dot p
               ; View.text
                   ~style:(style [ Font_size 12.; Font_weight 500 ])
                   "A little room to explore"
               ]
           ; View.text
               ~style:
                 (style
                    [ Font_size 11.
                    ; Foreground p.muted
                    ; White_space Normal
                    ; Line_height (px 17.)
                    ])
               "A native workspace, powered by OCaml. Everything here runs locally."
           ; button
               dark
               ~icons
               ~icon:Sliders
               "Demo controls"
               (action (fun () -> B.Expert.Var.set t.demo_controls (not demo_controls)))
           ]
       ]
       @ (if demo_controls
          then
            [ View.column
                ~style:
                  (style
                     [ Gap (px 5.)
                     ; Padding (px 8.)
                     ; Radius 9.
                     ; Background (solid p.canvas)
                     ])
                [ caption p "SIMULATED RESPONSES"
                ; button
                    dark
                    "Normal stream"
                    (action (fun () ->
                       set_backend t (Backend.Config.create () |> Or_error.ok_exn);
                       notify t "Normal streaming selected"))
                ; button
                    dark
                    "Slow stream"
                    (action (fun () ->
                       set_backend
                         t
                         (Backend.Config.create ~chunk_bytes:7 ~delay_seconds:0.06 ()
                          |> Or_error.ok_exn);
                       notify t "Slow streaming selected"))
                ; button
                    dark
                    "Simulate error"
                    (action (fun () ->
                       set_backend
                         t
                         (Backend.Config.create ~fail_after_chunks:8 () |> Or_error.ok_exn);
                       notify t "Next response will stop after eight chunks"))
                ]
            ]
          else [])
       @ [ View.row
             ~style:
               (style
                  [ Align_items Center
                  ; Padding_top (px 8.)
                  ; Border_top_width 1.
                  ; Border_color p.line
                  ])
             [ caption p "Local workspace"
             ; spacer
             ; icon_button
                 dark
                 icons
                 (if dark then Sun else Moon)
                 ~label:(if dark then "Light theme" else "Dark theme")
                 (action (fun () -> toggle_theme t))
             ]
         ])
  in
  let content =
    View.column
      ~style:
        (style
           [ Width full
           ; Height full
           ; Min_width (px 0.)
           ; Min_height (px 0.)
           ; Background (background dark)
           ; Foreground (foreground dark)
           ])
      ([ View.row
           ~style:
             (style
                [ Width full
                ; Height (px 45.)
                ; Shrink 0.
                ; Gap (px 4.)
                ; Align_items Center
                ; Padding_left (px 14.)
                ; Padding_right (px 12.)
                ; Border_bottom_width 1.
                ; Border_color p.line
                ; Background (solid p.sidebar)
                ])
           [ (if List.is_empty (Tabs.tabs tabs)
              then caption p "No open tabs"
              else
                View.tab_bar
                  ~style:
                    (Gpuio.Style.with_state_exn
                       (style [ Font_size 12.; Foreground p.muted ])
                       Selected
                       [ Border_color p.accent; Foreground p.text ])
                  ~config:(Tabs.choices tabs ~label:"Conversation tabs" |> Or_error.ok_exn)
                  ~on_select:(fun id ->
                    action (fun () -> select t (Int.of_string (Tabs.Id.to_string id))))
                  ())
           ; icon_button
               dark
               icons
               Close
               ~label:"Close tab"
               (action (fun () -> Option.iter active ~f:(close_tab t)))
           ; spacer
           ; icon_button
               dark
               icons
               External
               ~label:"New window"
               (action (fun () -> open_window selected))
           ; icon_button
               dark
               icons
               Command
               ~label:"Commands"
               (action (fun () -> B.Expert.Var.set t.palette true))
           ]
       ]
       @ List.map panels ~f:(fun (id, view) ->
         View.tab_panel
           ~key:(key ("panel-" ^ Int.to_string id))
           ~style:(style [ Grow 1.; Basis (px 0.); Min_height (px 0.); Width full ])
           ~label:(Conversation.title (find t id))
           ~active:(Option.equal Int.equal active (Some id))
           [ view ]))
  in
  let split =
    View.split_pane
      ~key:(key "workspace-split")
      ~style:
        (style
           [ Width full; Grow 1.; Basis (px 0.); Min_height (px 0.); Foreground p.line ])
      ~config:
        (Gpuio.Split_pane.Config.create
           ~label:"Conversation sidebar width"
           ~initial_first:250.
           ~minimum_first:210.
           ~maximum_first:380.
           ~minimum_second:500.
           ()
         |> Or_error.ok_exn)
      ~first:sidebar
      ~second:content
      ()
  in
  let close_dialog =
    View.dialog
      ~style:
        (style
           [ Background (solid p.surface)
           ; Border_width 1.
           ; Border_color p.line
           ; Radius 16.
           ; Padding (px 8.)
           ; Foreground p.text
           ])
      ~key:(key "close-decision")
      ~config:
        (Gpuio.Overlay.Config.create ~label:"Unsaved drafts" ~width:420. ()
         |> Or_error.ok_exn)
      ~on_dismiss:(fun _ -> action (fun () -> answer_close t Keep_open))
      (if pending_close
       then
         Some
           (View.column
              ~style:
                (style
                   [ Gap (px 12.)
                   ; Padding (px 16.)
                   ; Background (surface dark)
                   ; Foreground (foreground dark)
                   ])
              [ View.text
                  ~style:(style [ Font_size 18.; Font_weight 600; White_space Normal ])
                  "Close this window and discard its drafts?"
              ; View.text
                  ~style:
                    (style
                       [ Font_size 13.
                       ; Foreground p.muted
                       ; White_space Normal
                       ; Line_height (px 21.)
                       ])
                  "Other windows keep their own drafts. Closing the last window ends the \
                   demo."
              ; View.row
                  ~style:(style [ Gap (px 10.) ])
                  [ button
                      dark
                      "Keep editing"
                      (action (fun () -> answer_close t Keep_open))
                  ; button
                      dark
                      "Discard drafts and close"
                      (action (fun () -> answer_close t Allow))
                  ]
              ])
       else None)
  in
  View.command_scope
    ~commands
    ~style:
      (style
         [ Width full
         ; Height full
         ; Display Flex
         ; Direction Column
         ; Background (background dark)
         ; Foreground (foreground dark)
         ; Font_size 13.
         ])
    ([ View.menu_bar [ menu ] |> Or_error.ok_exn
     ; split
     ; View.text
         ~style:
           (style
              [ Padding_left (px 16.)
              ; Padding_top (px 5.)
              ; Padding_bottom (px 5.)
              ; Font_size 10.
              ; Shrink 0.
              ; Foreground p.faint
              ; Border_top_width 1.
              ; Border_color p.line
              ; Background (solid p.sidebar)
              ])
         notice
     ; close_dialog
     ]
     @
     if palette
     then
       [ View.command_palette
           ~appearance:
             (Gpuio.Command_palette.Appearance.create
                ~popup_width:540.
                ~row_height:42.
                ~popup_style:
                  (style
                     [ Background (solid p.surface)
                     ; Foreground p.text
                     ; Border_color p.line
                     ; Radius 12.
                     ; Font_size 13.
                     ])
                ~option_style:
                  (Gpuio.Style.with_state_exn
                     (style [ Foreground p.text; Background (solid p.surface) ])
                     Focused
                     [ Background (solid p.accent_surface); Foreground p.accent ])
                ~empty_style:(style [ Foreground p.muted ])
                ()
              |> Or_error.ok_exn)
           ~config:
             (Gpuio.Command_palette.Config.create
                ~label:"Workspace commands"
                ~commands:
                  (List.map
                     [ "new-window"
                     ; "next-tab"
                     ; "previous-tab"
                     ; "theme"
                     ; "copy"
                     ; "close-tab"
                     ; "close-window"
                     ]
                     ~f:command_id)
                ()
              |> Or_error.ok_exn)
           ~on_dismiss:(fun _ -> action (fun () -> B.Expert.Var.set t.palette false))
           ()
       ]
     else [])
;;
