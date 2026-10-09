open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn

let component window palette graph =
  let fraction, set_fraction = B.state (Some 0.35) graph in
  let animate, toggle_animate = B.toggle ~default_model:true graph in
  let inert, toggle_inert = B.toggle ~default_model:false graph in
  let draft =
    Gpuio_eio.Text_input.create
      window
      ~initial_text:"Keep typing"
      ~config:
        (B.return
           (Text_input.Config.create ~mode:Single_line ~label:"Progress center draft" ()
            |> ok))
      graph
  in
  let open B.Let_syntax in
  let%arr p = palette
  and fraction = fraction
  and set_fraction = set_fraction
  and animate = animate
  and toggle_animate = toggle_animate
  and inert = inert
  and toggle_inert = toggle_inert
  and draft = draft in
  let value =
    match fraction with
    | None -> Progress.Value.indeterminate
    | Some fraction -> Progress.Value.determinate ~fraction |> ok
  in
  let config label = Progress.Config.create ~label ~value |> ok in
  let transition =
    if animate
    then Progress.Transition.tween (Time_ns.Span.of_ms 600.) |> ok
    else Progress.Transition.immediate
  in
  let button label value = V.button ~on_click:(set_fraction value) label in
  Palette.card
    p
    ~title:"Progress with a living center"
    [ V.row
        ~style:(style [ Gap (px 24.); Align_items Center; Wrap Wrap ])
        [ V.progress_circle
            ~config:(config "Circular transfer progress")
            ~transition
            ~style:
              (style
                 [ Width (px 160.)
                 ; Height (px 160.)
                 ; Foreground (Palette.accent p)
                 ; Inert inert
                 ])
            [ V.column
                ~style:(style [ Width (px 112.); Gap (px 8.) ])
                [ Palette.text
                    p
                    (Option.value_map fraction ~default:"Working…" ~f:(fun v ->
                       sprintf "%.0f%%" (v *. 100.)))
                ; Gpuio_eio.Text_input.view draft
                ]
            ]
        ; V.column
            ~style:(style [ Gap (px 12.); Width (px 240.) ])
            [ V.progress
                ~config:(config "Rounded transfer progress")
                ~transition
                ~style:
                  (style
                     [ Height (px 16.)
                     ; Radius 8.
                     ; Foreground (Palette.accent p)
                     ; Inert inert
                     ])
                ()
            ; Palette.text
                p
                ~muted:true
                "The draft keeps its identity as progress changes."
            ]
        ]
    ; V.row
        ~style:(style [ Gap (px 8.); Wrap Wrap ])
        [ button "Empty" (Some 0.)
        ; button "Tiny" (Some 0.01)
        ; button "Quarter" (Some 0.25)
        ; button "Nearly there" (Some 0.85)
        ; button "Complete" (Some 1.)
        ; button "Unknown" None
        ]
    ; V.checkbox
        ~state:(if animate then Checked else Unchecked)
        ~on_toggle:toggle_animate
        "Animate value changes"
    ; V.checkbox
        ~state:(if inert then Checked else Unchecked)
        ~on_toggle:toggle_inert
        "Inert progress preview"
    ]
;;
