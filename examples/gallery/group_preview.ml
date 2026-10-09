open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module P = Presentation

let style = Style.create_exn
let px = Length.px_exn

let named name view =
  V.with_accessibility
    view
    (Accessibility.create ~role:Group ~label:name () |> Or_error.ok_exn)
  |> Or_error.ok_exn
;;

let component palette graph =
  let variant, set_variant = B.state P.Group_variant.Card graph in
  let header, toggle_header = B.toggle ~default_model:true graph in
  let footer, toggle_footer = B.toggle ~default_model:true graph in
  let refined, toggle_refined = B.toggle ~default_model:false graph in
  let checked, toggle_checked = B.toggle ~default_model:false graph in
  let clicks, click =
    B.state_machine0 ~default_model:0 ~apply_action:(fun _ count () -> count + 1) graph
  in
  let open B.Let_syntax in
  let%arr p = palette
  and variant = variant
  and set_variant = set_variant
  and header = header
  and toggle_header = toggle_header
  and footer = footer
  and toggle_footer = toggle_footer
  and refined = refined
  and toggle_refined = toggle_refined
  and checked = checked
  and toggle_checked = toggle_checked
  and clicks = clicks
  and click = click in
  let control label =
    V.button
      ~style:
        (style
           [ Width (px 210.)
           ; Padding (px 8.)
           ; Radius 8.
           ; Foreground (Palette.foreground p)
           ; Background (Background.solid (Palette.background p))
           ; Border_width 1.
           ; Border_color (Palette.border p)
           ])
      label
      ~on_click:(click ())
  in
  let checkbox label enabled toggle =
    V.checkbox ~state:(if enabled then Checked else Unchecked) ~on_toggle:toggle label
  in
  let variant_name =
    match variant with
    | P.Group_variant.Card -> "Card"
    | Plain -> "Plain"
    | Filled -> "Filled"
    | Outline -> "Outline"
  in
  V.column
    ~style:(style [ Gap (px 12.) ])
    [ Palette.text p "Change the frame. Keep your place."
    ; V.row
        ~style:(style [ Gap (px 8.); Wrap Wrap ])
        (List.map
           [ P.Group_variant.Card, "Card"
           ; Plain, "Plain"
           ; Filled, "Filled"
           ; Outline, "Outline"
           ]
           ~f:(fun (choice, label) ->
             Palette.button
               p
               ~selected:(P.Group_variant.equal variant choice)
               ("Group: " ^ label)
               (set_variant choice)))
    ; V.row
        ~style:(style [ Gap (px 12.); Wrap Wrap ])
        [ checkbox "Group header" header toggle_header
        ; checkbox "Group footer" footer toggle_footer
        ; checkbox "Refine group slots" refined toggle_refined
        ]
    ; P.group_box
        (Palette.appearance p)
        ~key:(Key.of_string_exn "group-preview")
        ~style:(style [ Width (px 440.) ])
        ~variant
        ~header_style:(if refined then style [ Padding_left (px 10.) ] else Style.empty)
        ~body_style:(if refined then style [ Padding (px 24.) ] else Style.empty)
        ~footer_style:(if refined then style [ Padding_left (px 20.) ] else Style.empty)
        ?header:(Option.some_if header (control "Group heading action"))
        ?footer:(Option.some_if footer (control "Group footer action"))
        [ V.column
            ~style:(style [ Gap (px 10.) ])
            [ checkbox "Keep group updates" checked toggle_checked
            ; control "Run group action"
            ]
          |> named "Group content"
        ]
      |> named "Configurable group"
    ; Palette.text p ~muted:true (sprintf "Group actions: %d" clicks)
    ; Palette.text
        p
        ~muted:true
        (sprintf
           "Group layout: %s · %s · %s · %s"
           variant_name
           (if refined then "refined" else "default")
           (if header then "header" else "no header")
           (if footer then "footer" else "no footer"))
    ]
;;
