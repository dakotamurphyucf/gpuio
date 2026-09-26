open Core
open Gpuio
module B = Bonsai.Cont
module E = Bonsai.Effect
module UI = Gpuio_bonsai.View
module Input = Gpuio_eio.Text_input

let ok = Or_error.ok_exn
let px = Length.px_exn
let full = Length.percent_exn 100.
let style = Style.create_exn
let initial_draft = "A small idea, ready to become something useful."

module Page = struct
  type t =
    | Draft
    | Review
    | Deliver

  let id t =
    Carousel.Id.of_string
      (match t with
       | Draft -> "draft"
       | Review -> "review"
       | Deliver -> "deliver")
    |> ok
  ;;

  let label = function
    | Draft -> "Draft"
    | Review -> "Review"
    | Deliver -> "Deliver"
  ;;

  let all =
    List.map [ Draft; Review; Deliver ] ~f:(fun t ->
      Carousel.Item.create ~id:(id t) ~label:(label t) t |> ok)
  ;;
end

module Action = struct
  type t =
    | Navigate of Carousel.Request.t
    | Toggle_axis
    | Toggle_looping
    | Toggle_autoplay
    | Toggle_lifetime
    | Toggle_disabled
    | Trim
    | Restore
    | Increment
end

module Model = struct
  type t =
    { carousel : Page.t Carousel.t
    ; axis : Carousel.Axis.t
    ; hidden : Content_policy.t
    ; counter : int
    ; requests : int
    }

  let initial =
    { carousel = Carousel.create ~looping:true Page.all |> ok
    ; axis = Horizontal
    ; hidden = Retain
    ; counter = 0
    ; requests = 0
    }
  ;;

  let selected_label t =
    Carousel.selected t.carousel
    |> Option.value_map ~default:"Empty" ~f:Carousel.Item.label
  ;;

  let axis t = t.axis
  let hidden t = t.hidden
  let counter t = t.counter
  let requests t = t.requests

  let apply t = function
    | Action.Navigate request ->
      { t with
        carousel = Carousel.apply_request t.carousel request |> ok
      ; requests = t.requests + 1
      }
    | Toggle_axis ->
      { t with
        axis =
          (match t.axis with
           | Horizontal -> Vertical
           | Vertical -> Horizontal)
      }
    | Toggle_looping ->
      { t with
        carousel =
          Carousel.with_looping t.carousel (not (Carousel.is_looping t.carousel)) |> ok
      }
    | Toggle_autoplay ->
      let policy =
        if Option.is_some (Carousel.auto_advance t.carousel)
        then None
        else
          Some (Carousel.Auto_advance.create ~interval:(Time_ns.Span.of_sec 4.) () |> ok)
      in
      { t with carousel = Carousel.with_auto_advance t.carousel policy |> ok }
    | Toggle_lifetime ->
      { t with
        hidden =
          (match t.hidden with
           | Retain -> Unmount
           | Unmount -> Retain)
      }
    | Toggle_disabled ->
      { t with
        carousel =
          Carousel.with_disabled t.carousel (not (Carousel.is_disabled t.carousel)) |> ok
      }
    | Trim ->
      { t with carousel = Carousel.with_items t.carousel (List.take Page.all 2) |> ok }
    | Restore -> { t with carousel = Carousel.with_items t.carousel Page.all |> ok }
    | Increment -> { t with counter = t.counter + 1 }
  ;;
end

module Observation = struct
  type t =
    { model : Model.t
    ; inject : Action.t -> unit E.t
    ; editor : Input.t
    }
end

let component ~observed window graph =
  let model, inject =
    B.state_machine0
      ~default_model:Model.initial
      ~apply_action:(fun _ model action -> Model.apply model action)
      graph
  in
  let editor =
    Input.create
      window
      ~initial_text:initial_draft
      ~config:
        (B.return
           (Text_input.Config.create ~mode:Single_line ~label:"Gallery draft" () |> ok))
      graph
  in
  let open B.Let_syntax in
  B.Edge.after_display
    (let%arr model = model
     and inject = inject
     and editor = editor in
     E.of_thunk (fun () -> observed := Some { Observation.model; inject; editor }))
    graph;
  let%arr model = model
  and inject = inject
  and editor = editor in
  let button action label = UI.button ~on_click:(inject action) label in
  let toggle label enabled action =
    UI.switch ~checked:enabled ~on_toggle:(inject action) label
  in
  let gallery =
    UI.carousel
      model.carousel
      ~key:(Key.of_string_exn "project-gallery")
      ~label:"Project gallery"
      ~axis:model.axis
      ~hidden:model.hidden
      ~on_request:(fun request -> inject (Navigate request))
      ~style:(style [ Width full; Height (px 296.); Gap (px 12.); Shrink 0. ])
      ~viewport_style:
        (style
           [ Width full
           ; Height (px 248.)
           ; Grow 0.
           ; Shrink 0.
           ; Radius 14.
           ; Background (Background.solid (Color.rgb_exn 0x142135))
           ])
      ~page_style:(style [ Width full; Height full ])
      ~controls_style:(style [ Justify_content Center; Gap (px 8.) ])
      ~control_style:(style [ Radius 8.; Padding (px 8.); Font_size 13. ])
      ~content:(fun item ->
        let page = Carousel.Item.data item in
        let number, title, subtitle, accent, background =
          match page with
          | Page.Draft ->
            ( "01 / CREATE"
            , "Make room for your next idea."
            , "A native draft, with room to explore."
            , 0x70e0c4
            , 0x142f35 )
          | Review ->
            ( "02 / REFINE"
            , "Good work gets better together."
            , "Move between pages without losing your place."
            , 0xb9afff
            , 0x292542 )
          | Deliver ->
            ( "03 / SHARE"
            , "From a first thought to something real."
            , "The same state, ready for a different view."
            , 0xf4c887
            , 0x383027 )
        in
        [ UI.column
            ~style:
              (style
                 [ Width full
                 ; Height full
                 ; Padding (px 22.)
                 ; Gap (px 12.)
                 ; Background (Background.solid (Color.rgb_exn background))
                 ])
            [ UI.text
                ~style:
                  (style
                     [ Font_size 11.; Font_weight 700; Foreground (Color.rgb_exn accent) ])
                number
            ; UI.text ~style:(style [ Font_size 23.; Font_weight 700 ]) title
            ; UI.text
                ~style:(style [ Font_size 14.; Foreground (Color.rgb_exn 0xc5d2e3) ])
                subtitle
            ; (match page with
               | Draft -> Input.view ~style:(style [ Width full; Height (px 38.) ]) editor
               | Review ->
                 UI.text "Drag the open area, use the arrows, or choose a page below."
               | Deliver ->
                 UI.text
                   "The application owns selection. Native motion follows your \
                    interaction.")
            ; UI.row
                ~style:(style [ Gap (px 10.); Align_items Center ])
                [ button Increment "Add a star"
                ; UI.text
                    (sprintf
                       "%d %s · shared Bonsai state"
                       model.counter
                       (if model.counter = 1 then "star" else "stars"))
                ]
            ]
        ])
      ()
  in
  UI.column
    ~style:
      (style
         [ Width full
         ; Shrink 0.
         ; Gap (px 14.)
         ; Padding (px 20.)
         ; Radius 18.
         ; Background (Background.solid (Color.rgb_exn 0x1b263b))
         ])
    [ UI.row
        ~style:
          (style
             [ Justify_content Space_between
             ; Align_items Center
             ; Gap (px 12.)
             ; Wrap Wrap
             ])
        [ UI.text
            ~style:(style [ Font_size 19.; Font_weight 600 ])
            "A gallery that feels at home."
        ; UI.text ("Gallery selection: " ^ Model.selected_label model)
        ]
    ; gallery
    ; UI.row
        ~style:(style [ Gap (px 14.); Wrap Wrap; Align_items Center ])
        [ button
            Toggle_axis
            (match model.axis with
             | Horizontal -> "Direction: horizontal"
             | Vertical -> "Direction: vertical")
        ; toggle "Loop pages" (Carousel.is_looping model.carousel) Toggle_looping
        ; toggle
            "Auto-advance"
            (Option.is_some (Carousel.auto_advance model.carousel))
            Toggle_autoplay
        ; toggle "Disable gallery" (Carousel.is_disabled model.carousel) Toggle_disabled
        ]
    ; UI.row
        ~style:(style [ Gap (px 10.); Wrap Wrap ])
        [ button
            Toggle_lifetime
            (match model.hidden with
             | Retain -> "Content: retain"
             | Unmount -> "Content: unmount")
        ; button Trim "Remove delivery page"
        ; button Restore "Restore all pages"
        ]
    ; UI.text
        ~style:(style [ Font_size 12.; Foreground (Color.rgb_exn 0xafbfd5) ])
        (match model.hidden with
         | Retain ->
           "Retain keeps the native draft. Auto-advance pauses for hover, focus and \
            reduced motion."
         | Unmount ->
           "Unmount resets the native draft on return. Stars and workspace data tasks \
            stay alive.")
    ]
;;
