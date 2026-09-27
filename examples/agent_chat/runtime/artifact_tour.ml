open Core
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module C = Gpuio.Carousel
module P = Gpuio.Presentation

let ok = Or_error.ok_exn
let px = Gpuio.Length.px_exn
let style = Gpuio.Style.create_exn
let full = Gpuio.Length.percent_exn 100.

module Page = struct
  type t =
    | Sources
    | Results
    | Diagram
    | Feedback

  let name = function
    | Sources -> "Sources"
    | Results -> "Results"
    | Diagram -> "Diagram"
    | Feedback -> "Feedback"
  ;;

  let all = [ Sources; Results; Diagram; Feedback ]

  let items =
    List.map all ~f:(fun page ->
      C.Item.create ~id:(C.Id.of_string (name page) |> ok) ~label:(name page) page |> ok)
  ;;

  let title = function
    | Sources -> "Start with the context."
    | Results -> "Find the signal."
    | Diagram -> "See how it connects."
    | Feedback -> "Give it a second look."
  ;;

  let description = function
    | Sources ->
      "Browse a lazy source tree, retry a failed collection, or explore a large local \
       fixture."
    | Results ->
      "Sort the complete sample findings, refine score filters, and inspect a row \
       without losing your place."
    | Diagram ->
      "Move stages, follow a connector, and inspect the details of a simulated run."
    | Feedback ->
      "Record a usefulness rating and private review note, then check the sources behind \
       the result."
  ;;
end

module Action = struct
  type t =
    | Navigate of C.Request.t
    | Toggle_auto
    | Deactivate
end

let apply carousel = function
  | Action.Navigate request -> C.apply_request carousel request |> ok
  | Toggle_auto ->
    let policy =
      if Option.is_some (C.auto_advance carousel)
      then None
      else Some (C.Auto_advance.create ~interval:(Time_ns.Span.of_sec 4.) () |> ok)
    in
    C.with_auto_advance carousel policy |> ok
  | Deactivate -> C.restart_auto_advance carousel |> ok
;;

let component ~active ~dark ~on_open graph =
  let carousel, inject =
    B.state_machine0
      ~default_model:(C.create ~looping:true Page.items |> ok)
      ~apply_action:(fun _ carousel action -> apply carousel action)
      graph
  in
  let open B.Let_syntax in
  B.Edge.on_change
    active
    ~equal:Bool.equal
    ~callback:
      (B.map inject ~f:(fun inject active ->
         if active then Bonsai.Effect.Ignore else inject Deactivate))
    graph;
  let%arr carousel = carousel
  and inject = inject
  and dark = dark in
  let p = Palette.of_dark dark in
  let appearance = if dark then P.Appearance.dark else P.Appearance.light in
  let button_style =
    style
      [ Foreground p.text
      ; Background (Gpuio.Background.solid p.raised)
      ; Border_width 1.
      ; Border_color p.line
      ; Padding (px 9.)
      ; Radius 8.
      ]
  in
  let selected =
    C.selected carousel |> Option.value_map ~default:"No artifacts" ~f:C.Item.label
  in
  V.column
    ~style:(style [ Gap (px 16.); Min_width (px 0.) ])
    [ V.row [ P.badge appearance ~tone:Accent ~size:Small "WORKSPACE TOUR" ]
    ; V.text
        ~style:(style [ Font_size 23.; Font_weight 600 ])
        "From context to confidence."
    ; V.text
        ~style:(style [ Foreground p.muted; Line_height (px 20.) ])
        "Four ways to explore the local agent workspace. Every card opens a working view."
    ; V.carousel
        carousel
        ~key:(Gpuio.Key.of_string_exn "artifact-tour")
        ~label:"Workspace tour"
        ~hidden:Unmount
        ~style:(style [ Width full; Height (px 360.); Shrink 0.; Gap (px 12.) ])
        ~viewport_style:
          (style
             [ Width full
             ; Height (px 308.)
             ; Grow 0.
             ; Shrink 0.
             ; Radius 14.
             ; Background (Gpuio.Background.solid p.surface)
             ])
        ~page_style:(style [ Width full; Height full ])
        ~controls_style:(style [ Justify_content Center; Gap (px 6.) ])
        ~control_style:
          (Gpuio.Style.merge [ button_style; style [ Padding (px 6.); Font_size 12. ] ])
        ~on_request:(fun request -> inject (Navigate request))
        ~content:(fun item ->
          let page = C.Item.data item in
          [ V.column
              ~style:
                (style
                   [ Width full
                   ; Height full
                   ; Padding (px 18.)
                   ; Gap (px 16.)
                   ; Background (Gpuio.Background.solid p.surface)
                   ; Radius 14.
                   ; Border_color p.line
                   ; Border_width 1.
                   ])
              [ P.marker
                  appearance
                  ~tone:Accent
                  ("EXPLORE / " ^ String.uppercase (Page.name page))
              ; V.text ~style:(style [ Font_size 24.; Font_weight 600 ]) (Page.title page)
              ; V.text
                  ~style:(style [ Foreground p.muted; Line_height (px 21.) ])
                  (Page.description page)
              ; V.column ~style:(style [ Grow 1. ]) []
              ; P.attachment
                  appearance
                  ~name:(Page.name page ^ " workspace")
                  ~detail:"Local sample · no external service"
                  ~style:
                    (style
                       [ Padding (px 10.)
                       ; Radius 10.
                       ; Background (Gpuio.Background.solid p.raised)
                       ])
                  ~actions:
                    (V.button
                       ~style:button_style
                       ~on_click:(on_open page)
                       ("Open " ^ Page.name page))
                  ()
              ]
          ])
        ()
    ; P.status_bar appearance ~leading:(P.marker appearance ("Tour stop: " ^ selected)) ()
    ; V.button
        ~style:button_style
        ~on_click:(inject Toggle_auto)
        (if Option.is_some (C.auto_advance carousel)
         then "Pause guided tour"
         else "Start guided tour")
    ; V.text
        ~style:(style [ Foreground p.muted; Font_size 12.; Line_height (px 18.) ])
        "Use arrows or swipe to explore. The optional tour pauses while you hover, focus \
         a card, hide the view, or use reduced motion."
    ]
;;
