open Core
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module S = Gpuio.Sidebar

let ok = Or_error.ok_exn
let px = Gpuio.Length.px_exn
let style = Gpuio.Style.create_exn

module Destination = struct
  type t =
    | Overview
    | Diagram
    | Review
    | Feedback
    | Tour
    | Sources
    | Results

  let all = [ Overview; Diagram; Review; Feedback; Tour; Sources; Results ]

  let name = function
    | Overview -> "Workspace overview"
    | Diagram -> "Run diagram"
    | Review -> "Checkpoints"
    | Feedback -> "Run feedback"
    | Tour -> "Guided tour"
    | Sources -> "Source collection"
    | Results -> "Run findings"
  ;;

  let id t = S.Id.of_string (name t) |> ok
  let find requested = List.find all ~f:(fun t -> S.Id.equal requested (id t))
end

let initial =
  let item ?(children = []) compact_label destination =
    S.Item.create
      ~id:(Destination.id destination)
      ~label:(Destination.name destination)
      ~compact_label
      ~children
      ()
    |> ok
  in
  let group label items =
    S.Group.create ~id:(S.Id.of_string label |> ok) ~label items |> ok
  in
  S.create
    ~selected:None
    ~collapse:Icon
    ~groups:
      [ group
          "ARTIFACTS"
          [ item "W" Overview
          ; item "R" Diagram ~children:[ item "C" Review; item "F" Feedback ]
          ; item "T" Tour
          ]
      ; group "CONTEXT" [ item "S" Sources ~children:[ item "F" Results ] ]
      ]
    ()
  |> ok
;;

module Action = struct
  type t =
    | Request of S.Request.t
    | Mode of S.Collapse.t
end

let component ~current ~icons ~dark ~on_select graph =
  let model, inject =
    B.state_machine0
      ~default_model:initial
      ~apply_action:(fun context model action ->
        match action with
        | Action.Mode mode -> S.with_collapse model mode
        | Request request ->
          let next = S.apply_request model request in
          (match request with
           | Select id when S.is_visible model id && not (S.is_disabled model) ->
             Option.iter (Destination.find id) ~f:(fun destination ->
               Bonsai.Apply_action_context.schedule_event context (on_select destination))
           | Select _ | Toggle _ | Toggle_collapsed -> ());
          next)
      graph
  in
  let open B.Let_syntax in
  let%arr model = model
  and inject = inject
  and icons = icons
  and current = current
  and dark = dark in
  let p = Palette.of_dark dark in
  let model = S.select model (Some (Destination.id current)) |> ok in
  let on_request request = inject (Action.Request request) in
  let button_style =
    style
      [ Foreground p.muted
      ; Background (Gpuio.Background.solid p.sidebar)
      ; Border_width 0.
      ; Padding (px 5.)
      ; Radius 6.
      ; Font_size 11.
      ]
  in
  let icon_mode = S.Collapse.equal (S.collapse model) Icon in
  let labels =
    S.Labels.create
      ~navigation:"Workspace destinations"
      ~current:"Current workspace page"
      ~expand_sidebar:"Show destinations"
      ~collapse_sidebar:"Collapse destinations"
      ~toggle_item:(fun ~label ~expanded ->
        (if expanded then "Collapse " else "Expand ") ^ label)
    |> ok
  in
  V.column
    ~style:(style [ Grow 1.; Basis (px 0.); Min_height (px 110.); Gap (px 5.) ])
    [ V.row
        ~style:(style [ Gap (px 4.); Wrap Wrap ])
        [ S.toggle model ~labels ~style:button_style ~on_request ()
        ; V.button
            ~style:button_style
            ~on_click:(inject (Mode (if icon_mode then Offcanvas else Icon)))
            (if icon_mode then "Hide mode" else "Icon mode")
        ]
    ; V.column
        ~style:(style [ Grow 1.; Basis (px 0.); Min_height (px 0.) ])
        [ S.view
            model
            ~labels
            ~hidden:Retain
            ~on_request
            ~decorate:(fun item ->
              let icon =
                match Destination.find (S.Item.id item) with
                | Some Overview -> Icons.Name.Command
                | Some Diagram -> Code
                | Some Review -> Check
                | Some Feedback -> Message
                | Some Tour -> Spark
                | Some Sources -> Paperclip
                | Some Results -> Search
                | None -> Command
              in
              S.Decoration.create ?icon:(Icons.decoration icons icon) ())
            ~appearance:
              (S.Appearance.create
                 ~width:174.
                 ~compact_width:44.
                 ~style:
                   (style
                      [ Background (Gpuio.Background.solid p.sidebar)
                      ; Border_width 0.
                      ; Padding (px 0.)
                      ])
                 ~item_style:
                   (style
                      [ Foreground p.muted
                      ; Background (Gpuio.Background.solid p.sidebar)
                      ; Font_size 12.
                      ; Padding (px 6.)
                      ])
                 ~current_style:
                   (style
                      [ Foreground p.accent
                      ; Background (Gpuio.Background.solid p.accent_surface)
                      ; Border_color p.accent
                      ])
                 ()
               |> ok)
            ()
          |> ok
        ]
    ]
;;
