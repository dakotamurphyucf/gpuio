open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module Controller = Gpuio_eio.Pagination

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn

type action =
  | Request of Pagination.Request.t
  | Count of int
  | Toggle_disabled

let component window palette graph =
  let pages, inject =
    B.state_machine0
      ~default_model:(Pagination.create ~total_pages:120 ~current:60 () |> ok)
      ~apply_action:(fun _ model -> function
         | Request request -> Pagination.apply_request model request
         | Count total -> Pagination.with_total_pages model total |> ok
         | Toggle_disabled ->
           Pagination.with_disabled model (not (Pagination.is_disabled model)))
      graph
  in
  let compact, toggle_compact = B.toggle ~default_model:false graph in
  let open B.Let_syntax in
  let layout =
    B.map compact ~f:(fun compact ->
      if compact then Navigation.Pagination_layout.Compact else Full)
  in
  let pager =
    Controller.create
      window
      ~model:pages
      ~layout
      ~on_request:(B.map inject ~f:(fun inject request -> inject (Request request)))
      graph
  in
  let%arr p = palette
  and pages = pages
  and inject = inject
  and compact = compact
  and toggle_compact = toggle_compact
  and pager = pager in
  Palette.card
    p
    ~title:"Navigation at any scale"
    [ V.row
        ~style:(style [ Gap (px 8.); Wrap Wrap ])
        [ Palette.button p ~selected:compact "Compact" toggle_compact
        ; Palette.button
            p
            ~selected:(Pagination.is_disabled pages)
            "Disabled"
            (inject Toggle_disabled)
        ; Palette.button p "120 pages" (inject (Count 120))
        ; Palette.button p "1 billion pages" (inject (Count Pagination.max_pages))
        ; Palette.button p "Shrink to 3" (inject (Count 3))
        ; Palette.button p "Empty" (inject (Count 0))
        ]
    ; Controller.view
        pager
        ~panel_style:
          (style
             [ Background (Background.solid (Palette.surface p))
             ; Foreground (Palette.foreground p)
             ; Radius 12.
             ])
        ~overlay:
          (Overlay.Config.create
             ~label:"Choose a hidden page"
             ~width:420.
             ~dismiss_on_outside_pointer:true
             ()
           |> ok)
      |> ok
    ; Palette.text
        p
        (match Pagination.current pages with
         | None -> "No pages to display"
         | Some current ->
           sprintf "Preview page %d of %d" current (Pagination.total_pages pages))
    ; Palette.text
        p
        ~muted:true
        "Open an ellipsis to jump to a hidden page. Type a number, then choose Go to \
         page."
    ]
;;
