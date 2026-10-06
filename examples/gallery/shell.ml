open Core
open Gpuio
module View = Gpuio_bonsai.View
module Page = Gpuio_gallery_model.Page
module Appearance = Gpuio_gallery_model.Appearance
module Theme_selection = Gpuio_gallery_model.Theme_selection

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn
let full = Length.percent_exn 100.

module Snapshot = struct
  type t =
    { page : Gpuio_gallery_model.Page.t
    ; appearance : Gpuio_gallery_model.Appearance.t
    ; preference : Gpuio_gallery_model.Theme_selection.t
    ; scale : Gpuio_gallery_model.Appearance.Scale.t
    ; window_snapshot : Gpuio.Window.Snapshot.t option
    ; capabilities : Gpuio.Window.Capabilities.t option
    ; custom_chrome : bool
    }
end

module Actions = struct
  type t =
    { select_page : Gpuio_gallery_model.Page.t -> unit Bonsai.Effect.t
    ; toggle_appearance : unit Bonsai.Effect.t
    ; follow_system : unit Bonsai.Effect.t
    ; next_scale : unit Bonsai.Effect.t
    ; open_window : unit Bonsai.Effect.t
    ; toggle_fullscreen : unit Bonsai.Effect.t
    ; minimize : unit Bonsai.Effect.t
    ; zoom : unit Bonsai.Effect.t
    ; close : unit Bonsai.Effect.t
    }
end

let view ~palette:p ~(snapshot : Snapshot.t) ~(actions : Actions.t) ~content =
  let { Snapshot.page
      ; appearance
      ; preference
      ; scale
      ; window_snapshot
      ; capabilities
      ; custom_chrome
      }
    =
    snapshot
  in
  let navigation =
    List.map Page.all ~f:(fun candidate ->
      Palette.button
        p
        ~selected:(Page.equal candidate page)
        (Page.title candidate)
        (actions.select_page candidate))
  in
  let workspace =
    View.row
      ~style:
        (style
           [ Width full
           ; Height full
           ; Background (Background.solid (Palette.background p))
           ; Foreground (Palette.foreground p)
           ])
      [ View.column
          ~style:
            (style
               [ Width (px 236.)
               ; Height full
               ; Shrink 0.
               ; Padding (px 22.)
               ; Gap (px 14.)
               ; Background (Background.solid (Palette.surface p))
               ; Border_right_width 1.
               ; Border_color (Palette.border p)
               ])
          [ Palette.text p ~size:24. "GPUIO"
          ; Palette.text p ~muted:true "COMPONENT STUDIO"
          ; Presentation.separator (Palette.appearance p) ()
          ; View.column
              ~style:
                (style [ Grow 1.; Min_height (px 0.); Overflow_y Scroll; Gap (px 6.) ])
              navigation
          ; Palette.text p ~muted:true "Built with OCaml.\nRendered natively."
          ]
      ; View.column
          ~style:
            (style
               [ Grow 1.; Min_width (px 0.); Height full; Padding (px 30.); Gap (px 22.) ])
          [ View.row
              ~style:(style [ Align_items Center; Gap (px 10.) ])
              [ View.column
                  ~style:(style [ Grow 1.; Gap (px 8.) ])
                  [ Palette.text p ~size:30. (Page.title page)
                  ; Palette.text p ~muted:true (Page.description page)
                  ]
              ; Palette.button p (Appearance.label appearance) actions.toggle_appearance
              ; Palette.button
                  p
                  ~selected:
                    (Appearance.Preference.equal
                       (Theme_selection.preference preference)
                       System)
                  "Follow system"
                  actions.follow_system
              ; Palette.button p (Appearance.Scale.label scale) actions.next_scale
              ; Palette.button p "New window" actions.open_window
              ]
          ; (View.column
               ~key:(Key.of_string ("preview-" ^ Page.key page) |> ok)
               ~style:
                 (style
                    [ Grow 1.
                    ; Min_height (px 0.)
                    ; Overflow_y Scroll
                    ; Padding_right (px 8.)
                    ])
               [ content ]
             |> fun view ->
             View.with_accessibility
               view
               (Accessibility.create ~role:Group ~label:"Component preview" () |> ok)
             |> ok)
          ]
      ]
  in
  if not custom_chrome
  then workspace
  else (
    let fullscreen =
      Option.exists window_snapshot ~f:(fun s -> s.Window.Snapshot.fullscreen)
    in
    let title_style =
      style
        [ Min_height (px 48.)
        ; Gap (px 12.)
        ; Padding_right (px 16.)
        ; Background (Background.solid (Palette.surface p))
        ; Border_bottom_width 1.
        ; Border_color (Palette.border p)
        ]
    in
    let title_children =
      [ Palette.text p ~muted:true "GPUIO  /  COMPONENT STUDIO"
      ; View.column ~style:(style [ Grow 1. ]) []
      ; Palette.button
          p
          ~disabled:
            (not
               (Option.exists window_snapshot ~f:(fun s ->
                  s.Window.Snapshot.presentation.controls.fullscreen)))
          (if fullscreen then "Leave fullscreen" else "Fullscreen")
          actions.toggle_fullscreen
      ]
      @
      match capabilities, window_snapshot with
      | Some capabilities, Some snapshot ->
        [ View.window_controls
            ~backend:capabilities.backend
            ~snapshot
            ~style:(style [ Gap (px 6.); Shrink 0. ])
            ~button_style:
              (style
                 [ Padding (px 10.)
                 ; Radius 8.
                 ; Font_size 13.
                 ; Foreground (Palette.foreground p)
                 ; Background (Background.solid (Palette.surface p))
                 ]
               |> fun s ->
               Style.with_state_exn
                 s
                 Hovered
                 [ Background (Background.solid (Palette.border p)) ])
            ~on_minimize:actions.minimize
            ~on_zoom:actions.zoom
            ~on_close:actions.close
            ()
        ]
      | _ -> []
    in
    let title_bar =
      match capabilities with
      | Some capabilities ->
        View.title_bar
          ~backend:capabilities.backend
          ~fullscreen
          ~style:title_style
          title_children
      | None ->
        (* The initial component precedes the native handshake. The first window
           observation supplies a backend before installing native regions. *)
        View.row ~style:title_style title_children
    in
    View.column
      ~style:(style [ Width full; Height full; Border_color (Palette.border p) ])
      [ title_bar
      ; View.column ~style:(style [ Grow 1.; Min_height (px 0.) ]) [ workspace ]
      ])
;;
