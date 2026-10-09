open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module Editor = Gpuio_eio.Text_input

let ok = Or_error.ok_exn
let px = Length.px_exn
let style = Style.create_exn
let id s = Choice.Id.of_string s |> ok
let key = Key.of_string_exn

let choices values =
  values
  |> List.map ~f:(fun (name, label) -> Choice.create ~id:(id name) ~label () |> ok)
  |> Choice.Collection.create
  |> ok
;;

let tabs = choices [ "notes", "Notes"; "draft", "Draft" ]

let variant_name = function
  | Tab_bar.Variant.Tab -> "Tab"
  | Outline -> "Outline"
  | Pill -> "Pill"
  | Segmented -> "Segmented"
  | Underline -> "Underline"
;;

let breadcrumb_path =
  choices [ "studio", "Studio"; "workspace", "Workspace"; "preview", "Preview" ]
;;

let component app window palette graph =
  let tab_content_preview = Tab_content_preview.component app window palette graph in
  let workflow = Stepper_preview.component window palette graph in
  let split_group = Split_group_preview.component window palette graph in
  let pagination_preview = Pagination_preview.component window palette graph in
  let disclosure_preview = Disclosure_preview.component window palette graph in
  let current_tab, set_tab = B.state (id "notes") graph in
  let decorated_tabs, toggle_decorated_tabs = B.toggle ~default_model:true graph in
  let styled_tabs, toggle_styled_tabs = B.toggle ~default_model:false graph in
  let tab_variant, next_variant =
    B.state_machine0
      ~default_model:Tab_bar.Variant.Underline
      ~apply_action:(fun _ variant () ->
        match variant with
        | Tab -> Outline
        | Outline -> Pill
        | Pill -> Segmented
        | Segmented -> Underline
        | Underline -> Tab)
      graph
  in
  let route, set_route = B.state (id "preview") graph in
  let passive, toggle_passive = B.toggle ~default_model:false graph in
  let note =
    Editor.create
      window
      ~config:
        (B.return
           (Text_input.Config.create ~mode:Multiline ~label:"Retained notes" () |> ok))
      ~initial_text:"This note stays intact when you change tabs."
      graph
  in
  let draft =
    Editor.create
      window
      ~config:
        (B.return
           (Text_input.Config.create ~mode:Multiline ~label:"Retained draft" () |> ok))
      ~initial_text:"A separate draft with its own native editing history."
      graph
  in
  let open B.Let_syntax in
  let%arr p = palette
  and tab_content_preview = tab_content_preview
  and split_group = split_group
  and workflow = workflow
  and current_tab = current_tab
  and set_tab = set_tab
  and decorated_tabs = decorated_tabs
  and toggle_decorated_tabs = toggle_decorated_tabs
  and styled_tabs = styled_tabs
  and toggle_styled_tabs = toggle_styled_tabs
  and tab_variant = tab_variant
  and next_variant = next_variant
  and note = note
  and draft = draft
  and disclosure_preview = disclosure_preview
  and pagination_preview = pagination_preview
  and route = route
  and set_route = set_route
  and passive = passive
  and toggle_passive = toggle_passive in
  let path =
    let members = Choice.Collection.to_list breadcrumb_path in
    let index =
      List.findi_exn members ~f:(fun _ item -> Choice.Id.equal (Choice.id item) route)
      |> fst
    in
    Choice.Collection.create (List.take members (index + 1)) |> ok
  in
  let input_style = style [ Height (px (Palette.size p 100.)); Padding (px 10.) ] in
  let first =
    V.column
      ~style:(style [ Gap (px 14.); Padding (px 12.) ])
      [ V.row
          ~style:(style [ Gap (px 12.); Align_items Center; Wrap Wrap ])
          [ Palette.button p ("Tab style: " ^ variant_name tab_variant) (next_variant ())
          ; V.switch
              ~checked:styled_tabs
              ~on_toggle:toggle_styled_tabs
              "Customize tab targets"
          ]
      ; V.switch
          ~checked:decorated_tabs
          ~on_toggle:toggle_decorated_tabs
          "Decorated workspace tabs"
      ; V.tab_bar_with_labels
          ~appearance:
            (Tab_bar.Appearance.create
               ~variant:tab_variant
               ~height:(if styled_tabs then 40. else 32.)
               ~tab_style:
                 (if styled_tabs
                  then
                    Style.with_state_exn
                      Style.empty
                      Hovered
                      [ Background (Background.solid (Palette.border p)) ]
                  else Style.empty)
               ~item_styles:
                 (if styled_tabs
                  then
                    [ id "notes", style [ Width (px 160.) ]
                    ; id "draft", style [ Width (px 140.) ]
                    ]
                  else [])
               ()
             |> ok)
          ~style:
            (style [ Foreground (Palette.foreground p); Border_color (Palette.accent p) ])
          ~config:
            (Choice.Config.create
               ~label:"Preview workspace tabs"
               ~options:tabs
               ~selected:(Some current_tab)
               ()
             |> ok)
          ~on_select:set_tab
          ~labels:
            (if decorated_tabs
             then
               List.map
                 [ "notes", "Notes", "3"; "draft", "Draft", "Unsent" ]
                 ~f:(fun (name, label, badge) ->
                   ( id name
                   , V.row
                       ~style:(style [ Gap (px 8.); Align_items Center ])
                       [ Palette.text p label
                       ; V.text
                           ~style:
                             (style
                                [ Padding (px 4.)
                                ; Radius 6.
                                ; Font_size (Palette.size p 11.)
                                ; Foreground (Palette.accent p)
                                ; Background (Background.solid (Palette.border p))
                                ])
                           badge
                       ] ))
             else [])
          ()
        |> ok
      ; tab_content_preview
      ; V.tab_panel
          ~key:(key "notes-panel")
          ~label:"Notes panel"
          ~active:(Choice.Id.equal current_tab (id "notes"))
          [ Editor.view ~style:input_style note ]
      ; V.tab_panel
          ~key:(key "draft-panel")
          ~label:"Draft panel"
          ~active:(Choice.Id.equal current_tab (id "draft"))
          [ Editor.view ~style:input_style draft ]
      ]
  in
  V.column
    ~style:(style [ Gap (px 20.) ])
    [ Palette.card
        p
        ~title:"A workspace that keeps your place"
        [ Navigation.breadcrumbs
            path
            ~label:"Preview breadcrumb"
            ~current_description:"Current preview"
            ~is_navigable:(fun item ->
              not (passive && Choice.Id.equal (Choice.id item) (id "workspace")))
            ~item_style:(fun item ->
              if Choice.Id.equal (Choice.id item) (id "studio")
              then style [ Foreground (Palette.accent p); Font_weight 700 ]
              else Style.empty)
            ~on_navigate:set_route
            ()
          |> ok
        ; V.row
            ~style:(style [ Gap (px 8.); Wrap Wrap ])
            [ Palette.button p "Open Preview" (set_route (id "preview"))
            ; Palette.button p ~selected:passive "Passive workspace label" toggle_passive
            ]
        ; Palette.text p ~muted:true ("Current location: " ^ Choice.Id.to_string route)
        ; V.split_pane
            ~style:(style [ Height (px (Palette.size p 220.)) ])
            ~config:
              (Split_pane.Config.create
                 ~label:"Preview workspace split"
                 ~initial_first:400.
                 ~minimum_first:200.
                 ~minimum_second:110.
                 ()
               |> ok)
            ~first
            ~second:
              (V.column
                 ~style:(style [ Padding (px 16.); Gap (px 12.) ])
                 [ Palette.text p ~size:18. "Inspector"
                 ; Palette.text
                     p
                     ~muted:true
                     "Resize this divider with the pointer or keyboard."
                 ])
            ()
        ]
    ; split_group
    ; workflow
    ; disclosure_preview
    ; pagination_preview
    ]
;;
