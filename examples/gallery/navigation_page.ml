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

let sections =
  choices [ "identity", "Identity"; "behavior", "Behavior"; "lifetime", "Lifetime" ]
;;

let breadcrumb_path =
  choices [ "studio", "Studio"; "workspace", "Workspace"; "preview", "Preview" ]
;;

let component window palette graph =
  let current_tab, set_tab = B.state (id "notes") graph in
  let disclosure, inject_disclosure =
    B.state_machine0
      ~default_model:
        (Disclosure.create
           ~items:sections
           ~mode:(Single { allow_empty = true })
           ~expanded:[]
           ()
         |> ok)
      ~apply_action:(fun _ state request -> Disclosure.apply_request state request)
      graph
  in
  let pagination, inject_page =
    B.state_machine0
      ~default_model:(Pagination.create ~total_pages:12 () |> ok)
      ~apply_action:(fun _ state request -> Pagination.apply_request state request)
      graph
  in
  let notice, set_notice = B.state "Current location: Preview" graph in
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
  and current_tab = current_tab
  and set_tab = set_tab
  and note = note
  and draft = draft
  and disclosure = disclosure
  and inject_disclosure = inject_disclosure
  and pagination = pagination
  and inject_page = inject_page
  and notice = notice
  and set_notice = set_notice in
  let input_style = style [ Height (px (Palette.size p 100.)); Padding (px 10.) ] in
  let first =
    V.column
      ~style:(style [ Gap (px 14.); Padding (px 12.) ])
      [ V.tab_bar
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
          ()
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
            breadcrumb_path
            ~label:"Preview breadcrumb"
            ~current_description:"Current preview"
            ~on_navigate:(fun target ->
              set_notice ("Requested location: " ^ Choice.Id.to_string target))
            ()
          |> ok
        ; Palette.text p ~muted:true notice
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
    ; Palette.card
        p
        ~title:"Reveal the right amount"
        [ V.accordion
            ~model:disclosure
            ~hidden:Unmount
            ~on_request:inject_disclosure
            ~trigger_style:
              (style
                 [ Padding (px 12.)
                 ; Font_size (Palette.size p 15.)
                 ; Background (Background.solid (Palette.border p))
                 ; Foreground (Palette.foreground p)
                 ; Radius 8.
                 ])
            ~panel_style:(style [ Padding (px 12.) ])
            ~content:(fun target ->
              [ Palette.text
                  p
                  (List.Assoc.find_exn
                     [ ( id "identity"
                       , "Stable keys preserve the identity of each native control." )
                     ; ( id "behavior"
                       , "Native controls handle immediate input; OCaml receives \
                          semantic requests." )
                     ; ( id "lifetime"
                       , "This accordion unmounts collapsed views; the workspace tabs \
                          retain their editors." )
                     ]
                     ~equal:Choice.Id.equal
                     target)
              ])
            ()
        ]
    ; Palette.card
        p
        ~title:"Navigation at any scale"
        [ Navigation.pagination pagination ~on_request:inject_page () |> ok
        ; Palette.text
            p
            (sprintf
               "Preview page %d of 12"
               (Option.value (Pagination.current pagination) ~default:1))
        ]
    ]
;;
