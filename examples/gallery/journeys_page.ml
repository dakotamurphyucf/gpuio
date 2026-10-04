open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module Editor = Gpuio_eio.Text_input

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn
let full = Length.percent_exn 100.

module Chapter = struct
  type t =
    | Imagine
    | Shape
    | Share

  let all = [ Imagine; Shape; Share ]

  let name = function
    | Imagine -> "Imagine"
    | Shape -> "Shape"
    | Share -> "Share"
  ;;

  let detail = function
    | Imagine -> "Every useful thing begins with a small idea."
    | Shape -> "Give the idea form. Make the next step clear."
    | Share -> "Bring others into the conversation."
  ;;

  let carousel_id t = Carousel.Id.of_string (name t) |> ok
  let route_id t = Navigation_stack.Id.of_string (name t) |> ok
end

module Slides = struct
  type t =
    { carousel : Chapter.t Carousel.t
    ; axis : Carousel.Axis.t
    }

  module Action = struct
    type t =
      | Request of Carousel.Request.t
      | Toggle_axis
      | Toggle_auto
  end

  let initial =
    { carousel =
        Carousel.create
          ~looping:true
          (List.map Chapter.all ~f:(fun chapter ->
             Carousel.Item.create
               ~id:(Chapter.carousel_id chapter)
               ~label:(Chapter.name chapter)
               chapter
             |> ok))
        |> ok
    ; axis = Horizontal
    }
  ;;

  let apply t = function
    | Action.Request request ->
      { t with carousel = Carousel.apply_request t.carousel request |> ok }
    | Toggle_axis ->
      { t with
        axis =
          (match t.axis with
           | Horizontal -> Vertical
           | Vertical -> Horizontal)
      }
    | Toggle_auto ->
      let auto =
        if Option.is_some (Carousel.auto_advance t.carousel)
        then None
        else
          Some (Carousel.Auto_advance.create ~interval:(Time_ns.Span.of_sec 4.) () |> ok)
      in
      { t with carousel = Carousel.with_auto_advance t.carousel auto |> ok }
  ;;
end

module Route_action = struct
  type t =
    | Back
    | Forward
end

module Rail = struct
  module Action = struct
    type t =
      | Request of Sidebar.Request.t
      | Toggle_mode
      | Toggle_activation
  end

  let id s = Sidebar.Id.of_string s |> ok

  let item name ?activation ?children () =
    Sidebar.Item.create
      ~id:(id name)
      ~label:name
      ~compact_label:(String.prefix name 1)
      ?activation
      ?children
      ()
    |> ok
  ;;

  let groups activation =
    [ Sidebar.Group.create
        ~id:(id "workspace")
        ~label:"Workspace"
        [ item
            "Projects"
            ~activation
            ~children:[ item "Orchard" (); item "Observatory" () ]
            ()
        ; item "Archive" ()
        ]
      |> ok
    ]
  ;;

  let activation t =
    Sidebar.find t (id "Projects") |> Option.value_exn |> Sidebar.Item.activation
  ;;

  let activation_label t =
    match activation t with
    | Select_only -> "Branch selection: navigate"
    | Expand -> "Branch selection: expand"
    | Toggle -> "Branch selection: toggle"
  ;;

  let initial =
    Sidebar.create
      ~groups:(groups Sidebar.Item.Activation.Select_only)
      ~selected:(Some (id "Orchard"))
      ~expanded:[ id "Projects" ]
      ~collapse:Icon
      ()
    |> ok
  ;;

  let apply t = function
    | Action.Request request -> Sidebar.apply_request t request
    | Toggle_activation ->
      let activation =
        match activation t with
        | Select_only -> Sidebar.Item.Activation.Expand
        | Expand -> Sidebar.Item.Activation.Toggle
        | Toggle -> Sidebar.Item.Activation.Select_only
      in
      Sidebar.with_groups t (groups activation) |> ok
    | Toggle_mode ->
      Sidebar.with_collapse
        t
        (match Sidebar.collapse t with
         | Icon -> Offcanvas
         | Offcanvas | Never -> Icon)
  ;;
end

let component window palette graph =
  let measured_cards = Carousel_track_preview.component window palette graph in
  let slides, slide =
    B.state_machine0
      ~default_model:Slides.initial
      ~apply_action:(fun _ model action -> Slides.apply model action)
      graph
  in
  let history, navigate =
    B.state_machine0
      ~default_model:
        (Navigation_stack.create
           ~current:(Chapter.route_id Imagine)
           (List.map Chapter.all ~f:(fun chapter ->
              Navigation_stack.Entry.create
                ~id:(Chapter.route_id chapter)
                ~label:(Chapter.name chapter)
                chapter
              |> ok))
         |> ok)
      ~apply_action:(fun _ history -> function
         | Route_action.Back -> Navigation_stack.pop history
         | Forward -> Navigation_stack.forward history)
      graph
  in
  let rail, request =
    B.state_machine0
      ~default_model:Rail.initial
      ~apply_action:(fun _ model action -> Rail.apply model action)
      graph
  in
  let styled_labels, set_styled_labels = B.state true graph in
  let editor label initial_text =
    Editor.create
      window
      ~initial_text
      ~config:(B.return (Text_input.Config.create ~mode:Single_line ~label () |> ok))
      graph
  in
  let draft = editor "Carousel idea" "Keep this idea as the slides move." in
  let note = editor "Journey note" "Return here and pick up where you left off." in
  let open B.Let_syntax in
  let%arr p = palette
  and slides = slides
  and slide = slide
  and history = history
  and navigate = navigate
  and rail = rail
  and request = request
  and styled_labels = styled_labels
  and set_styled_labels = set_styled_labels
  and draft = draft
  and measured_cards = measured_cards
  and note = note in
  let panel_style = style [ Padding (px 18.); Gap (px 14.); Width full; Height full ] in
  let button_style =
    style
      [ Padding (px 8.)
      ; Radius 8.
      ; Foreground (Palette.foreground p)
      ; Background (Background.solid (Palette.border p))
      ]
  in
  let current = Navigation_stack.current history |> Option.value_exn in
  V.column
    ~style:(style [ Gap (px 20.) ])
    [ Palette.card
        p
        ~title:"Let an idea unfold"
        [ V.row
            ~style:(style [ Gap (px 12.); Align_items Center ])
            [ Palette.button
                p
                (match slides.axis with
                 | Horizontal -> "Use vertical slides"
                 | Vertical -> "Use horizontal slides")
                (slide Toggle_axis)
            ; V.switch
                ~checked:(Option.is_some (Carousel.auto_advance slides.carousel))
                ~on_toggle:(slide Toggle_auto)
                "Auto-advance slides"
            ]
        ; V.carousel
            slides.carousel
            ~label:"Idea carousel"
            ~hidden:Retain
            ~on_request:(fun r -> slide (Request r))
            ~axis:slides.axis
            ~style:(style [ Height (px 245.); Gap (px 12.); Shrink 0. ])
            ~viewport_style:
              (style
                 [ Height (px 190.)
                 ; Width full
                 ; Grow 0.
                 ; Shrink 0.
                 ; Radius 12.
                 ; Background (Background.solid (Palette.background p))
                 ])
            ~page_style:panel_style
            ~control_style:button_style
            ~controls_style:(style [ Gap (px 8.); Justify_content Center ])
            ~content:(fun item ->
              let chapter = Carousel.Item.data item in
              [ Palette.text p ~size:22. (Chapter.name chapter)
              ; Palette.text p ~muted:true (Chapter.detail chapter)
              ; (match chapter with
                 | Imagine -> Editor.view draft
                 | Shape | Share -> V.column [])
              ])
            ()
        ; Palette.text
            p
            ("Current idea: "
             ^ (Carousel.selected slides.carousel
                |> Option.value_exn
                |> Carousel.Item.label))
        ]
    ; measured_cards
    ; Palette.card
        p
        ~title:"Move through a workspace"
        [ V.row
            ~style:(style [ Gap (px 10.); Wrap Wrap ])
            [ Sidebar.toggle
                rail
                ~style:button_style
                ~on_request:(fun r -> request (Request r))
                ()
            ; Palette.button
                p
                (match Sidebar.collapse rail with
                 | Icon -> "Use offcanvas sidebar"
                 | Offcanvas | Never -> "Use icon sidebar")
                (request Toggle_mode)
            ; Palette.button p (Rail.activation_label rail) (request Toggle_activation)
            ; V.switch
                ~checked:styled_labels
                ~on_toggle:(set_styled_labels (not styled_labels))
                "Style sidebar labels"
            ; V.button
                ~style:button_style
                ~disabled:(not (Navigation_stack.can_pop history))
                ~on_click:(navigate Back)
                "Go back"
            ; V.button
                ~style:button_style
                ~disabled:(not (Navigation_stack.can_forward history))
                ~on_click:(navigate Forward)
                "Continue journey"
            ]
        ; V.row
            ~style:(style [ Height (px 220.); Gap (px 14.); Overflow_y Hidden ])
            [ Sidebar.view
                rail
                ~hidden:Retain
                ~appearance:
                  (Sidebar.Appearance.create
                     ~width:180.
                     ~compact_width:48.
                     ~style:
                       (style
                          [ Background (Background.solid (Palette.background p))
                          ; Foreground (Palette.foreground p)
                          ])
                     ~item_style:(style [ Foreground (Palette.foreground p) ])
                     ~current_style:
                       (style
                          [ Foreground (Palette.accent p)
                          ; Background (Background.solid (Palette.border p))
                          ])
                     ()
                   |> ok)
                ~on_request:(fun r -> request (Request r))
                ~decorate:(fun item ->
                  let featured =
                    Sidebar.Id.equal (Sidebar.Item.id item) (Rail.id "Projects")
                  in
                  Sidebar.Decoration.create
                    ~style:(if styled_labels then style [ Radius 8. ] else Style.empty)
                    ~label_style:
                      (if styled_labels && featured
                       then style [ Font_weight 700; Foreground (Palette.accent p) ]
                       else Style.empty)
                    ?suffix:
                      (if featured
                       then Some (Palette.text p ~muted:true ~size:11. "02")
                       else None)
                    ())
                ()
              |> ok
            ; V.navigation_stack
                history
                ~label:"Idea journey"
                ~hidden:Retain
                ~style:
                  (style
                     [ Grow 1.
                     ; Min_width (px 0.)
                     ; Height full
                     ; Background (Background.solid (Palette.background p))
                     ; Radius 12.
                     ])
                ~page_style:panel_style
                ~content:(fun entry ->
                  let chapter = Navigation_stack.Entry.data entry in
                  [ Palette.text p ~size:22. ("Journey: " ^ Chapter.name chapter)
                  ; Palette.text p ~muted:true (Chapter.detail chapter)
                  ; (match chapter with
                     | Imagine -> Editor.view note
                     | Shape | Share -> V.column [])
                  ])
                ()
            ]
        ; Palette.text p ("Current journey: " ^ Navigation_stack.Entry.label current)
        ; Palette.text
            p
            ("Destination: "
             ^ (Sidebar.selected rail
                |> Option.value_map ~default:"None" ~f:Sidebar.Id.to_string))
        ]
    ]
;;
