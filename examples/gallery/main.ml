open Core
open Gpuio
module App = Gpuio_eio.App
module Scope = Gpuio_eio.Scope
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module Page = Gpuio_gallery_model.Page
module Appearance = Gpuio_gallery_model.Appearance

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn
let full = Length.percent_exn 100.

let component ~app ~open_window ~page ~appearance ~scale window graph =
  let open B.Let_syntax in
  B.Edge.on_change
    (B.Expert.Var.value appearance)
    ~equal:Appearance.equal
    ~callback:
      (B.return (fun appearance ->
         E.of_thunk (fun () ->
           App.Window.set_theme
             window
             (Palette.theme (Palette.create appearance Appearance.Scale.Comfortable)))))
    graph;
  let page_value = B.Expert.Var.value page in
  let palette =
    let%arr a = B.Expert.Var.value appearance
    and s = B.Expert.Var.value scale in
    Palette.create a s
  in
  let content = Pages.component ~app window ~page:page_value ~palette graph in
  let%arr page_value = page_value
  and p = palette
  and content = content
  and appearance_value = B.Expert.Var.value appearance
  and scale_value = B.Expert.Var.value scale in
  let action f = E.of_thunk f in
  let navigation =
    List.map Page.all ~f:(fun candidate ->
      Palette.button
        p
        ~selected:(Page.equal candidate page_value)
        (Page.title candidate)
        (action (fun () -> B.Expert.Var.set page candidate)))
  in
  V.row
    ~style:
      (style
         [ Width full
         ; Height full
         ; Background (Background.solid (Palette.background p))
         ; Foreground (Palette.foreground p)
         ])
    [ V.column
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
        ; V.column
            ~style:(style [ Grow 1.; Min_height (px 0.); Overflow_y Scroll; Gap (px 6.) ])
            navigation
        ; Palette.text p ~muted:true "Built with OCaml.\nRendered natively."
        ]
    ; V.column
        ~style:
          (style
             [ Grow 1.; Min_width (px 0.); Height full; Padding (px 30.); Gap (px 22.) ])
        [ V.row
            ~style:(style [ Align_items Center; Gap (px 10.) ])
            [ V.column
                ~style:(style [ Grow 1.; Gap (px 8.) ])
                [ Palette.text p ~size:30. (Page.title page_value)
                ; Palette.text p ~muted:true (Page.description page_value)
                ]
            ; Palette.button
                p
                (Appearance.label appearance_value)
                (action (fun () ->
                   B.Expert.Var.set appearance (Appearance.toggle appearance_value)))
            ; Palette.button
                p
                (Appearance.Scale.label scale_value)
                (action (fun () ->
                   B.Expert.Var.set scale (Appearance.Scale.next scale_value)))
            ; Palette.button p "New window" (action open_window)
            ]
        ; V.column
            ~key:(Key.of_string ("preview-" ^ Page.key page_value) |> ok)
            ~style:
              (style
                 [ Grow 1.; Min_height (px 0.); Overflow_y Scroll; Padding_right (px 8.) ])
            [ content ]
        ]
    ]
;;

let () =
  let background = Array.exists (Sys.get_argv ()) ~f:(String.equal "--background") in
  App.run (fun _env app ->
    let windows = ref [] in
    let serial = ref 0 in
    let rec open_window () =
      windows := List.filter !windows ~f:(fun w -> not (App.Window.is_closed w));
      if List.length !windows < 4
      then (
        incr serial;
        let page = B.Expert.Var.create Page.Presentation in
        let appearance = B.Expert.Var.create Appearance.Dark in
        let scale = B.Expert.Var.create Appearance.Scale.Comfortable in
        let window =
          App.open_window
            app
            ~theme:
              (Palette.theme
                 (Palette.create Appearance.Dark Appearance.Scale.Comfortable))
            ~focus:(not background)
            ~title:(sprintf "GPUIO · Component Studio %d" !serial)
            ~width:1120.
            ~height:820.
            (component ~app ~open_window ~page ~appearance ~scale)
          |> ok
        in
        windows := window :: !windows)
    in
    open_window ())
;;
