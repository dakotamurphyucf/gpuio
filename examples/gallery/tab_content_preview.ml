open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View

let ok = Or_error.ok_exn
let id s = Choice.Id.of_string s |> ok

let definitions =
  [ "plan", "A thoughtful plan for a new workspace"
  ; "draft", "Draft response and implementation notes"
  ; "review", "Review"
  ; "research", "Research sources and references"
  ; "tools", "Tool execution and diagnostics"
  ; "archive", "Archived conversations"
  ]
;;

module Model = struct
  type t =
    { open_tabs : string list
    ; selected : string option
    ; reveal_serial : int64
    ; reveal : Tab_bar.Reveal_request.t option
    ; last_action : string
    }

  let initial =
    { open_tabs = List.map definitions ~f:fst
    ; selected = Some "plan"
    ; reveal_serial = 0L
    ; reveal = None
    ; last_action = "Close buttons have independent actions."
    }
  ;;
end

module Action = struct
  type t =
    | Select of string
    | Close of string
    | Reveal of string
    | Reverse
    | Restore
end

let apply (model : Model.t) = function
  | Action.Select name ->
    { model with selected = Some name; last_action = "Selected " ^ name }
  | Close name ->
    let open_tabs =
      List.filter model.open_tabs ~f:(fun tab -> not (String.equal tab name))
    in
    let selected =
      if Option.equal String.equal model.selected (Some name)
      then List.hd open_tabs
      else model.selected
    in
    { model with open_tabs; selected; last_action = "Closed " ^ name }
  | Reverse -> { model with open_tabs = List.rev model.open_tabs }
  | Restore -> { Model.initial with reveal_serial = model.reveal_serial }
  | Reveal name ->
    if Int64.equal model.reveal_serial Int64.max_value
    then { model with last_action = "Reveal request limit reached" }
    else (
      let reveal_serial = Int64.succ model.reveal_serial in
      { model with
        reveal_serial
      ; reveal = Some (Tab_bar.Reveal_request.create (id name) ~serial:reveal_serial |> ok)
      ; last_action = "Reveal requested for " ^ name
      })
;;

let register_icon app scope =
  let source =
    Asset.Source.of_bytes
      ~format:Svg
      {|<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16"><path d="M4 1.5h5l3 3V14H4zM9 1.5v3h3M6 8h4M6 10.5h4" fill="none" stroke="white" stroke-width="1.25" stroke-linejoin="round"/></svg>|}
    |> ok
  in
  Bonsai.Effect.map (Gpuio_eio.Asset.register app ~scope source) ~f:(function
    | Ok asset -> Icon.Decoration.create ~asset:(Gpuio_eio.Asset.handle asset) ()
    | Error error -> Error (Error.create_s [%sexp (error : Gpuio_eio.Asset.Error.t)]))
;;

let component app window palette graph =
  let menu_icon =
    Preview_scope.acquire
      window
      ~name:"gallery-tab-menu-icons"
      ~create:(register_icon app)
      graph
  in
  let animate, toggle_animate = B.toggle ~default_model:true graph in
  let show_icons, toggle_icons = B.toggle ~default_model:true graph in
  let model, inject =
    B.state_machine0
      ~default_model:Model.initial
      ~apply_action:(fun _ model action -> apply model action)
      graph
  in
  let narrow, toggle_narrow = B.toggle ~default_model:true graph in
  let open B.Let_syntax in
  let%arr p = palette
  and model = model
  and inject = inject
  and menu_icon = menu_icon
  and animate = animate
  and toggle_animate = toggle_animate
  and show_icons = show_icons
  and toggle_icons = toggle_icons
  and narrow = narrow
  and toggle_narrow = toggle_narrow in
  let style = Style.create_exn in
  let px = Length.px_exn in
  let options =
    List.map model.open_tabs ~f:(fun name ->
      Choice.create
        ~id:(id name)
        ~label:(List.Assoc.find_exn definitions name ~equal:String.equal)
        ()
      |> ok)
    |> Choice.Collection.create
    |> ok
  in
  V.column
    ~style:(style [ Gap (px 12.); Padding (px 12.) ])
    [ Palette.text p "Independent tab controls"
    ; V.row
        ~style:(style [ Gap (px 8.); Wrap Wrap; Align_items Center ])
        [ V.switch ~checked:narrow ~on_toggle:toggle_narrow "Truncate long tab names"
        ; Palette.button p "Reverse tabs" (inject Reverse)
        ; Palette.button p "Restore tabs" (inject Restore)
        ; Palette.button
            p
            "Select last tab"
            (Option.value_map
               (List.last model.open_tabs)
               ~default:B.Effect.Ignore
               ~f:(fun name -> inject (Select name)))
        ; Palette.button
            p
            "Reveal last tab"
            (Option.value_map
               (List.last model.open_tabs)
               ~default:B.Effect.Ignore
               ~f:(fun name -> inject (Reveal name)))
        ]
    ; V.tab_bar_with_content
        ~key:(Key.of_string_exn "closable-tabs")
        ?max_width:(if narrow then Some 190. else None)
        ~viewport:(Tab_bar.Viewport.create ?reveal:model.reveal ())
        ?motion:(if animate then Some Tab_bar.Motion.default else None)
        ~appearance:
          (Tab_bar.Appearance.create
             ~variant:Pill
             ~tab_style:
               (Style.with_state_exn
                  Style.empty
                  Selected
                  [ Foreground (Palette.accent p) ])
             ()
           |> ok)
        ~style:(style [ Foreground (Palette.foreground p) ])
        ~config:
          (Choice.Config.create
             ~label:"Closable workspace tabs"
             ~options
             ~selected:(Option.map model.selected ~f:id)
             ()
           |> ok)
        ~content:
          (List.map model.open_tabs ~f:(fun name ->
             ( id name
             , V.Tab_content.create
                 ~prefix:(V.text ~style:(style [ Foreground (Palette.accent p) ]) "•")
                 ~suffix:
                   (V.button
                      ~key:(Key.of_string_exn "close")
                      ~accessible_name:("Close " ^ name)
                      ~style:(style [ Width (px 22.); Height (px 22.); Padding (px 2.) ])
                      ~on_click:(inject (Close name))
                      "×")
                 ()
               |> ok )))
        ~on_select:(fun choice -> inject (Select (Choice.Id.to_string choice)))
        ()
      |> ok
      |> V.tab_bar_frame
           ~menu:
             (Tab_bar.Menu.create
                ~icons:
                  (match menu_icon, show_icons with
                   | Preview_scope.Ready icon, true ->
                     List.map model.open_tabs ~f:(fun name -> id name, icon)
                   | _ -> [])
                ()
              |> ok)
           ~style:
             (style
                [ Width (px 560.)
                ; Max_width (Length.percent_exn 100.)
                ; Align_items Center
                ; Gap (px 8.)
                ])
           ~prefix:(Palette.text p "Workspace")
           ~suffix:
             (V.button
                ~accessible_name:"Restore workspace tabs"
                ~on_click:(inject Restore)
                "+")
           ~trailing:
             (V.button
                ~accessible_name:"Restore tabs from scroll end"
                ~style:(style [ Shrink 0.; Margin_left (px 8.) ])
                ~on_click:(inject Restore)
                "Restore")
      |> ok
    ; V.switch ~checked:animate ~on_toggle:toggle_animate "Animate tab selection"
    ; V.switch ~checked:show_icons ~on_toggle:toggle_icons "Show tab menu icons"
    ; Palette.text p model.last_action
    ]
;;
