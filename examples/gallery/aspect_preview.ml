open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View

let style = Style.create_exn
let px = Length.px_exn
let ok = Or_error.ok_exn

let component palette graph =
  let ratio, next_ratio =
    B.state_machine0
      ~default_model:0
      ~apply_action:(fun _ value () -> (value + 1) % 3)
      graph
  in
  let wide, toggle_width = B.toggle ~default_model:false graph in
  let fixed, toggle_height = B.toggle ~default_model:false graph in
  let count, increment =
    B.state_machine0 ~default_model:0 ~apply_action:(fun _ value () -> value + 1) graph
  in
  let open B.Let_syntax in
  let%arr p = palette
  and ratio = ratio
  and next_ratio = next_ratio
  and wide = wide
  and toggle_width = toggle_width
  and fixed = fixed
  and toggle_height = toggle_height
  and count = count
  and increment = increment in
  let ratio_label, ratio = [| "Square", 1.; "Landscape", 2.; "Portrait", 0.5 |].(ratio) in
  let label name view =
    V.with_accessibility view (Accessibility.create ~role:Group ~label:name () |> ok)
    |> ok
  in
  Palette.card
    p
    ~title:"Proportions that follow your layout"
    [ Palette.text
        p
        ~muted:true
        "Resize the frame. The preview keeps its proportions and its state."
    ; V.row
        ~style:(style [ Gap (px 8.); Wrap Wrap ])
        [ Palette.button p ("Ratio: " ^ ratio_label) (next_ratio ())
        ; Palette.button p (if wide then "Frame: wide" else "Frame: compact") toggle_width
        ; Palette.button
            p
            (if fixed then "Height: fixed" else "Height: automatic")
            toggle_height
        ]
    ; V.column
        ~key:(Key.of_string_exn "aspect-frame")
        ~style:
          (style
             [ Width (px (if wide then 200. else 140.))
             ; Padding (px 12.)
             ; Border_width 1.
             ; Border_color (Palette.border p)
             ; Radius 14.
             ; Align_items Start
             ; Background (Background.solid (Palette.background p))
             ])
        [ V.column
            ~key:(Key.of_string_exn "aspect-preview")
            ~style:
              (style
                 [ Width (Length.percent_exn 100.)
                 ; Aspect_ratio ratio
                 ; Height (if fixed then px 80. else Length.auto)
                 ; Shrink 0.
                 ; Justify_content Center
                 ; Align_items Center
                 ; Radius 8.
                 ; Background (Background.solid (Palette.surface p))
                 ])
            [ Palette.button p (sprintf "Kept %d" count) (increment ()) ]
          |> label "Aspect preview surface"
        ]
      |> label "Aspect preview frame"
    ]
;;
