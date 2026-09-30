open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module P = Presentation

let style = Style.create_exn
let px = Length.px_exn
let ok = Or_error.ok_exn

let component palette graph =
  let vertical, toggle_vertical = B.toggle ~default_model:false graph in
  let dashed, toggle_dashed = B.toggle ~default_model:false graph in
  let labelled, toggle_labelled = B.toggle ~default_model:true graph in
  let narrow, toggle_narrow = B.toggle ~default_model:false graph in
  let custom, toggle_custom = B.toggle ~default_model:false graph in
  let long_label, toggle_long_label = B.toggle ~default_model:false graph in
  let clipped, toggle_clipped = B.toggle ~default_model:false graph in
  let checked, toggle_checked = B.toggle ~default_model:false graph in
  let open B.Let_syntax in
  let%arr p = palette
  and vertical = vertical
  and toggle_vertical = toggle_vertical
  and dashed = dashed
  and toggle_dashed = toggle_dashed
  and labelled = labelled
  and toggle_labelled = toggle_labelled
  and narrow = narrow
  and toggle_narrow = toggle_narrow
  and custom = custom
  and toggle_custom = toggle_custom
  and long_label = long_label
  and toggle_long_label = toggle_long_label
  and clipped = clipped
  and toggle_clipped = toggle_clipped
  and checked = checked
  and toggle_checked = toggle_checked in
  let checkbox label state action =
    V.checkbox ~state:(if state then Checked else Unchecked) ~on_toggle:action label
  in
  V.column
    ~style:(style [ Gap (px 12.) ])
    [ Palette.text p "Room between ideas."
    ; V.row
        ~style:(style [ Gap (px 12.); Wrap Wrap ])
        [ checkbox "Vertical separator" vertical toggle_vertical
        ; checkbox "Dashed separator" dashed toggle_dashed
        ; checkbox "Label separator" labelled toggle_labelled
        ; checkbox "Narrow separator" narrow toggle_narrow
        ; checkbox "Custom separator colors" custom toggle_custom
        ]
    ; V.row
        ~style:(style [ Gap (px 12.); Wrap Wrap ])
        [ checkbox "Long separator label" long_label toggle_long_label
        ; checkbox "Bound separator label" clipped toggle_clipped
        ]
    ; (V.column
         ~style:
           (style
              [ Width (px (if narrow then 200. else 360.))
              ; Height (px 180.)
              ; Align_items Center
              ; Justify_content Center
              ; Background (Background.solid (Palette.surface p))
              ])
         [ (P.Separator.create
              (Palette.appearance p)
              ~key:(Key.of_string_exn "separator")
              ~axis:(if vertical then Vertical else Horizontal)
              ~pattern:(if dashed then Dashed else Solid)
              ~style:
                (if not clipped
                 then Style.empty
                 else if vertical
                 then style [ Max_width (px 96.) ]
                 else style [ Max_height (px 32.) ])
              ?color:(Option.some_if custom (Palette.accent p))
              ~label_style:
                (if custom
                 then style [ Foreground (Palette.accent p); Font_size 16. ]
                 else Style.empty)
              ?label:
                (Option.some_if
                   labelled
                   (if long_label
                    then
                      "Continue with another account · 保存した内容は保持されます · Your progress \
                       stays here"
                    else "Continue · 世界"))
              ()
            |> fun view ->
            V.with_accessibility
              view
              (Accessibility.create ~role:Separator ~label:"Rich separator preview" ()
               |> ok)
            |> ok)
         ]
       |> fun view ->
       V.with_accessibility
         view
         (Accessibility.create ~role:Group ~label:"Separator frame" () |> ok)
       |> ok)
    ; checkbox "Keep separator updates" checked toggle_checked
    ; Palette.text
        p
        ~muted:true
        (sprintf
           "Separator: %s · %s · %s · %s · %s"
           (if vertical then "vertical" else "horizontal")
           (if dashed then "dashed" else "solid")
           (if labelled then "labelled" else "plain")
           (if narrow then "narrow" else "wide")
           (if custom then "custom" else "default"))
    ; Palette.text
        p
        ~muted:true
        (sprintf
           "Separator label: %s · %s"
           (if long_label then "long" else "short")
           (if clipped then "bounded" else "natural"))
    ]
;;
