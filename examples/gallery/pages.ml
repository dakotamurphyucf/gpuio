open Core
open Gpuio
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module P = Presentation
module Editor = Gpuio_eio.Text_input
module Page = Gpuio_gallery_model.Page

let ok = Or_error.ok_exn
let px = Length.px_exn
let style = Style.create_exn
let group children = V.column ~style:(style [ Gap (px 20.) ]) children

let status_regions palette graph =
  let leading, toggle_leading = B.toggle ~default_model:true graph in
  let trailing, toggle_trailing = B.toggle ~default_model:true graph in
  let center, toggle_center = B.toggle ~default_model:true graph in
  let clicks, click =
    B.state_machine0 ~default_model:0 ~apply_action:(fun _ count () -> count + 1) graph
  in
  let open B.Let_syntax in
  let%arr p = palette
  and leading = leading
  and toggle_leading = toggle_leading
  and trailing = trailing
  and toggle_trailing = toggle_trailing
  and center = center
  and toggle_center = toggle_center
  and clicks = clicks
  and click = click in
  V.column
    ~style:(style [ Gap (px 8.) ])
    [ V.row
        ~style:(style [ Gap (px 16.); Wrap Wrap ])
        [ V.checkbox
            ~state:(if leading then Checked else Unchecked)
            ~on_toggle:toggle_leading
            "Leading status"
        ; V.checkbox
            ~state:(if center then Checked else Unchecked)
            ~on_toggle:toggle_center
            "Center action"
        ; V.checkbox
            ~state:(if trailing then Checked else Unchecked)
            ~on_toggle:toggle_trailing
            "Trailing status"
        ]
    ; (P.status_bar
         (Palette.appearance p)
         ~style:(style [ Height (px 56.) ])
         ?leading:(Option.some_if leading (Palette.text p "Workspace ready"))
         ?center:(Option.some_if center (Palette.button p "Sync workspace" (click ())))
         ?trailing:(Option.some_if trailing (Palette.text p "UTF-8"))
         ()
       |> fun view ->
       V.with_accessibility
         view
         (Accessibility.create ~role:Group ~label:"Workspace status bar" () |> ok)
       |> ok)
    ; Palette.text p ~muted:true (sprintf "Status actions: %d" clicks)
    ]
;;

let presentation app window palette graph =
  let attachment_preview = Attachment_preview.component app window palette graph in
  let badge_preview = Badge_preview.component app window palette graph in
  let label_preview = Label_preview.component palette graph in
  let shimmer_preview = Shimmer_preview.component palette graph in
  let marker_preview = Marker_preview.component palette graph in
  let alert_preview = Alert_preview.component palette graph in
  let tag_preview = Tag_preview.component palette graph in
  let chat_composition_preview =
    Chat_composition_preview.component app window palette graph
  in
  let chat_list_preview = Chat_list_preview.component app window palette graph in
  let group_preview = Group_preview.component palette graph in
  let empty_preview = Empty_preview.component app window palette graph in
  let separator_preview = Separator_preview.component palette graph in
  let link_preview = Link_preview.component app window palette graph in
  let status_regions = status_regions palette graph in
  let notice, set_notice = B.state "Ready when you are" graph in
  let animate_loading, toggle_loading = B.toggle ~default_model:false graph in
  let open B.Let_syntax in
  let%arr p = palette
  and notice = notice
  and animate_loading = animate_loading
  and toggle_loading = toggle_loading
  and status_regions = status_regions
  and attachment_preview = attachment_preview
  and badge_preview = badge_preview
  and shimmer_preview = shimmer_preview
  and marker_preview = marker_preview
  and alert_preview = alert_preview
  and tag_preview = tag_preview
  and chat_composition_preview = chat_composition_preview
  and chat_list_preview = chat_list_preview
  and label_preview = label_preview
  and group_preview = group_preview
  and empty_preview = empty_preview
  and separator_preview = separator_preview
  and link_preview = link_preview
  and set_notice = set_notice in
  let a = Palette.appearance p in
  group
    [ Palette.card
        p
        ~title:"A little context goes a long way"
        [ V.row
            ~style:(style [ Gap (px 10.); Wrap Wrap ])
            [ P.badge a ~tone:Accent "New release"
            ; P.tag a ~tone:Success "Connected"
            ; P.marker a ~tone:Warning "Review requested"
            ; P.shortcut_label a [ "⌘"; "K" ]
            ]
        ; P.separator a ()
        ; P.description_list
            a
            [ P.Description.create
                ~key:(Key.of_string "runtime" |> ok)
                ~term:"Runtime"
                ~definition:(Palette.text p "OCaml + native GPUI")
            ; P.Description.create
                ~key:(Key.of_string "appearance" |> ok)
                ~term:"Appearance"
                ~definition:(Palette.text p "Your colors, your components")
            ]
        ; status_regions
        ; badge_preview
        ]
    ; Palette.card p ~title:"Attachments with a little more to say" [ attachment_preview ]
    ; Palette.card p ~title:"Text with context" [ label_preview ]
    ; Palette.card p ~title:"A little light, in motion" [ shimmer_preview ]
    ; Palette.card p ~title:"Signals that stay out of the way" [ marker_preview ]
    ; Palette.card p ~title:"A clear next step" [ alert_preview ]
    ; Palette.card p ~title:"Small details, useful actions" [ tag_preview ]
    ; Palette.card p ~title:"Room for a conversation" [ chat_composition_preview ]
    ; Palette.card p ~title:"A conversation in motion" [ chat_list_preview ]
    ; Palette.card p ~title:"Structure with flexibility" [ group_preview ]
    ; Palette.card p ~title:"Space with intention" [ separator_preview ]
    ; Palette.card p ~title:"Follow your curiosity" [ link_preview ]
    ; Palette.card
        p
        ~title:"Feedback with a purpose"
        [ P.alert
            a
            ~tone:Success
            ~title:"Everything is in sync"
            [ Palette.text
                p
                ~muted:true
                "Your changes are available across this workspace."
            ]
        ; P.banner
            a
            ~tone:Accent
            ~title:"Make room for something new"
            ~actions:
              (Palette.button p "Try it" (set_notice "You activated the banner action"))
            [ Palette.text p "Actions remain ordinary keyboard-accessible controls." ]
        ; P.status_bar a ~leading:(P.marker a ~tone:Success notice) ()
        ]
    ; Palette.card
        p
        ~title:"Messages & attachments"
        [ P.message
            a
            ~author:"Aster"
            ~detail:"Just now"
            ~avatar:
              (V.avatar
                 ~style:
                   (style
                      [ Foreground (Palette.background p)
                      ; Background (Background.solid (Palette.accent p))
                      ])
                 (Avatar.Config.create
                    ~fallback:(Avatar.Fallback.create "AS" |> ok)
                    ~description:(Image.Description.label "Aster avatar" |> ok)
                    ()))
            (P.bubble
               a
               (Palette.text p "Native interfaces can be expressive and approachable."))
        ; P.attachment
            a
            ~name:"design-notes.md"
            ~detail:"Markdown · 2.4 KB"
            ~actions:
              (Palette.button p "Inspect attachment" (set_notice "Attachment selected"))
            ()
        ; P.tool_result
            a
            ~title:"Workspace scan"
            ~status:(P.badge a ~tone:Success "Complete")
            (Palette.text p "12 files reviewed. No changes needed.")
        ]
    ; Palette.card
        p
        ~title:"A useful empty state"
        [ P.empty_state
            a
            ~title:"A fresh start"
            ~description:"Create your first collection."
            ~actions:
              (Palette.button p "Create collection" (set_notice "Collection created"))
            ()
        ; empty_preview
        ]
    ; Palette.card
        p
        ~title:"Waiting can feel considered"
        [ V.switch
            ~checked:animate_loading
            ~on_toggle:toggle_loading
            "Animate loading previews"
        ; V.row
            ~style:(style [ Gap (px 20.); Align_items Center ])
            (List.map
               [ Loading.Kind.Skeleton, "Loading skeleton"
               ; Shimmer, "Loading shimmer"
               ; Spinner, "Loading spinner"
               ]
               ~f:(fun (kind, label) ->
                 V.loading
                   ~style:(style [ Foreground (Palette.accent p) ])
                   ~config:
                     (Loading.Config.create ~kind ~label ~animated:animate_loading ()
                      |> ok)
                   ()))
        ; Palette.text
            p
            ~muted:true
            "Static by default. Animation follows the application's motion policy."
        ]
    ]
;;

let choice_id s = Choice.Id.of_string s |> ok

let choices =
  [ "balanced", "Balanced"; "focused", "Focused"; "explore", "Explore" ]
  |> List.map ~f:(fun (id, label) -> Choice.create ~id:(choice_id id) ~label () |> ok)
  |> Choice.Collection.create
  |> ok
;;

let controls window palette graph =
  let enabled, toggle_enabled = B.toggle ~default_model:true graph in
  let checked, toggle_checked = B.toggle ~default_model:true graph in
  let count, update_count =
    B.state_machine0 ~default_model:0 ~apply_action:(fun _ count () -> count + 1) graph
  in
  let selected, set_selected = B.state (Some (choice_id "balanced")) graph in
  let open B.Let_syntax in
  let config =
    let%arr enabled = enabled
    and selected = selected in
    Combobox.Config.create
      ~label:"Find a mode"
      ~options:choices
      ~selected
      ~disabled:(not enabled)
      ~placeholder:"Search modes"
      ()
    |> ok
  in
  let on_select =
    let%arr set_selected = set_selected in
    fun selection -> set_selected (Some (Combobox.Selection.id selection))
  in
  let combo = Gpuio_eio.Combobox.create window ~config ~on_select graph in
  let%arr p = palette
  and enabled = enabled
  and toggle_enabled = toggle_enabled
  and checked = checked
  and toggle_checked = toggle_checked
  and count = count
  and update_count = update_count
  and selected = selected
  and set_selected = set_selected
  and combo = combo in
  let config label =
    Choice.Config.create ~label ~options:choices ~selected ~disabled:(not enabled) ()
    |> ok
  in
  group
    [ Palette.card
        p
        ~title:"Intentional actions"
        [ Palette.button p (sprintf "Pressed %d times" count) (update_count ())
        ; V.switch ~checked:enabled ~on_toggle:toggle_enabled "Enable selection controls"
        ; V.checkbox
            ~disabled:(not enabled)
            ~state:(Check_state.of_bool checked)
            ~on_toggle:toggle_checked
            "Receive updates"
        ; Palette.text
            p
            ~muted:true
            (if checked then "Updates are enabled." else "Updates are paused.")
        ]
    ; Palette.card
        p
        ~title:"Choose your mode"
        [ V.radio_group
            ~config:(config "Mode radio group")
            ~on_select:(fun id -> set_selected (Some id))
            ()
        ; V.select
            ~config:(config "Mode dropdown")
            ~on_select:(fun id -> set_selected (Some id))
            ()
        ; Gpuio_eio.Combobox.view
            ~style:(style [ Height (px (Palette.size p 40.)) ])
            combo
        ; Palette.text
            p
            ~muted:true
            ("Selected: "
             ^ Option.value_map selected ~default:"None" ~f:Choice.Id.to_string)
        ]
    ]
;;

let editors window palette graph =
  let read_only, toggle_read_only = B.toggle ~default_model:false graph in
  let invalid, toggle_invalid = B.toggle ~default_model:false graph in
  let submitted, set_submitted = B.state "Nothing submitted yet" graph in
  let open B.Let_syntax in
  let config mode label =
    let%arr read_only = read_only in
    Text_input.Config.create ~mode ~label ~read_only ~placeholder:"Write something…" ()
    |> ok
  in
  let on_submit =
    let%arr set_submitted = set_submitted in
    fun submission -> set_submitted (Text_input.Submission.text submission)
  in
  let title =
    Editor.create
      window
      ~config:(config Single_line "Document title")
      ~initial_text:"A place for good ideas"
      ~on_submit
      graph
  in
  let body =
    Editor.create
      window
      ~config:(config Multiline "Document body")
      ~initial_text:
        "Hello, 世界. 👨‍👩‍👧‍👦\n\
         Select text, undo an edit, or compose with your preferred input method."
      graph
  in
  let%arr p = palette
  and read_only = read_only
  and invalid = invalid
  and toggle_invalid = toggle_invalid
  and toggle_read_only = toggle_read_only
  and title = title
  and body = body
  and submitted = submitted in
  group
    [ Palette.card
        p
        ~title:"An editor that belongs on your desktop"
        [ V.switch ~checked:read_only ~on_toggle:toggle_read_only "Read-only preview"
        ; V.switch ~checked:invalid ~on_toggle:toggle_invalid "Show validation error"
        ; Form.field
            (Form.Field.create
               ~label:"Document title"
               ~help:"A name to find this document again."
               ~required:true
               ?error:
                 (if invalid
                  then Some "Choose a different title for this example."
                  else None)
               ()
             |> ok)
            ~label_style:(style [ Foreground (Palette.foreground p) ])
            ~help_style:(style [ Foreground (Palette.muted p) ])
            ~control:
              (Editor.view ~style:(style [ Height (px (Palette.size p 42.)) ]) title)
            ()
          |> ok
        ; Editor.view ~style:(style [ Height (px (Palette.size p 180.)) ]) body
        ; Palette.text
            p
            ~muted:true
            "Press Enter in the title to submit. Multiline text keeps its own editing \
             session."
        ; Palette.text p ("Submitted: " ^ submitted)
        ]
    ]
;;

let component ~app ~desktop ~motion window ~page ~palette graph =
  let open B.Let_syntax in
  match%sub page with
  | Page.Presentation -> presentation app window palette graph
  | Styles -> Styles_page.component palette graph
  | Controls -> controls window palette graph
  | Text_inputs -> editors window palette graph
  | Numeric_inputs -> Numeric_page.component window palette graph
  | Pickers -> Pickers_page.component window palette graph
  | Overlays -> Overlays_page.component palette graph
  | Navigation -> Navigation_page.component window palette graph
  | Feedback -> Feedback_page.component window palette graph
  | Journeys -> Journeys_page.component window palette graph
  | Collections -> Collections_page.component palette graph
  | Documents -> Documents_page.component app window palette graph
  | Highlighting -> Highlight_page.component app window palette graph
  | Canvas -> Canvas_page.component app window palette graph
  | Assets -> Assets_page.component app window palette graph
  | Charts -> Charts_page.component app window palette graph
  | Motion -> Motion_page.component app ~motion palette graph
  | Extensions -> Extensions_page.component palette graph
  | Input -> Input_page.component palette graph
  | Observations -> Observations_page.component window palette graph
  | Responsive -> Responsive_page.component window palette graph
  | Desktop -> Desktop_page.component app desktop window palette graph
  | Runtime -> Runtime_page.component app window palette graph
;;
