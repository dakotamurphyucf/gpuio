open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn

module Variant = struct
  type t =
    | Default
    | Primary
    | Secondary
    | Danger
    | Info
    | Success
    | Warning
    | Ghost
    | Link
    | Text
    | Custom
  [@@deriving equal]

  let all =
    [ Default
    ; Primary
    ; Secondary
    ; Danger
    ; Info
    ; Success
    ; Warning
    ; Ghost
    ; Link
    ; Text
    ; Custom
    ]
  ;;

  let label = function
    | Default -> "Default"
    | Primary -> "Primary"
    | Secondary -> "Secondary"
    | Danger -> "Danger"
    | Info -> "Info"
    | Success -> "Success"
    | Warning -> "Warning"
    | Ghost -> "Ghost"
    | Link -> "Link"
    | Text -> "Text"
    | Custom -> "Custom"
  ;;

  let accent t p =
    let themed light dark =
      Color.rgb_exn
        (match Palette.document_appearance p with
         | Document.Appearance.Light -> light
         | Dark -> dark)
    in
    match t with
    | Default | Secondary | Ghost | Text -> Palette.foreground p
    | Primary | Link -> Palette.accent p
    | Danger -> themed 0xb4233a 0xff9ba9
    | Info -> themed 0x175cd3 0x9bc1ff
    | Success -> themed 0x087443 0x83dfac
    | Warning -> themed 0x884a0e 0xf5cf80
    | Custom -> themed 0x6941c6 0xd0b5ff
  ;;
end

let appearance variant p ~outline ~compact ~large ~rounded ~selected =
  let accent = Variant.accent variant p in
  let alpha color opacity = Color.with_opacity color opacity |> ok in
  let transparent = alpha accent 0. in
  let filled =
    match variant with
    | Variant.Primary | Danger | Info | Success | Warning | Custom -> not outline
    | Default | Secondary | Ghost | Link | Text -> false
  in
  let background, foreground =
    if selected
    then alpha accent 0.18, accent
    else if filled
    then accent, Palette.background p
    else (
      let background =
        match variant with
        | Variant.Default -> Palette.surface p
        | Secondary -> Palette.border p
        | Primary | Danger | Info | Success | Warning | Ghost | Link | Text | Custom ->
          transparent
      in
      background, accent)
  in
  let border = outline || Variant.equal variant Default in
  let text_only = Variant.equal variant Text || Variant.equal variant Link in
  let height = Palette.size p (if large then 44. else 32.) in
  let padding = if text_only then 0. else Palette.size p (if compact then 8. else 16.) in
  let shadow =
    if Variant.equal variant Custom && not outline
    then
      [ Shadow.create
          ~color:(alpha accent 0.25)
          ~offset_x:0.
          ~offset_y:3.
          ~blur:10.
          ~spread:0.
          ()
        |> ok
      ]
    else []
  in
  let base =
    style
      ([ Style.Property.Background (Background.solid background)
       ; Foreground foreground
       ; Padding_left (px padding)
       ; Padding_right (px padding)
       ; Padding_top (px 0.)
       ; Padding_bottom (px 0.)
       ; Font_size (Palette.size p (if large then 15. else 13.))
       ; Font_weight 500
       ; Border_width (if border then 1. else 0.)
       ; Border_color (if selected || outline then accent else Palette.border p)
       ; Radius (if rounded then height /. 2. else 7.)
       ; Shadows shadow
       ; Text_decoration (if Variant.equal variant Link then Underline else None)
       ]
       @ if text_only then [] else [ Height (px height) ])
    |> fun base ->
    Style.with_state_exn
      base
      Focused
      [ Shadows
          [ Shadow.create ~color:accent ~offset_x:0. ~offset_y:0. ~blur:0. ~spread:2. ()
            |> ok
          ]
      ]
    |> fun focused -> Style.with_state_exn focused Disabled [ Opacity 0.45; Shadows [] ]
  in
  if selected
  then base
  else
    base
    |> fun base ->
    Style.with_state_exn
      base
      Hovered
      [ Background
          (Background.solid (if filled then alpha accent 0.88 else alpha accent 0.12))
      ]
    |> fun hovered ->
    Style.with_state_exn
      hovered
      Pressed
      [ Background
          (Background.solid (if filled then alpha accent 0.72 else alpha accent 0.22))
      ]
;;

let component palette graph =
  let outline, toggle_outline = B.toggle ~default_model:false graph in
  let compact, toggle_compact = B.toggle ~default_model:false graph in
  let large, toggle_large = B.toggle ~default_model:false graph in
  let rounded, toggle_rounded = B.toggle ~default_model:false graph in
  let selected, toggle_selected = B.toggle ~default_model:false graph in
  let loading, toggle_loading = B.toggle ~default_model:false graph in
  let disabled, toggle_disabled = B.toggle ~default_model:false graph in
  let hovered, observe_hover =
    B.state_machine0
      ~default_model:None
      ~apply_action:(fun _ current (variant, hovered) ->
        if hovered
        then Some variant
        else if Option.exists current ~f:(Variant.equal variant)
        then None
        else current)
      graph
  in
  let requests, invoke =
    B.state_machine0
      ~default_model:(0, None)
      ~apply_action:(fun _ (count, _) variant -> count + 1, Some variant)
      graph
  in
  let open B.Let_syntax in
  let%arr p = palette
  and outline = outline
  and toggle_outline = toggle_outline
  and compact = compact
  and toggle_compact = toggle_compact
  and large = large
  and toggle_large = toggle_large
  and rounded = rounded
  and toggle_rounded = toggle_rounded
  and selected = selected
  and toggle_selected = toggle_selected
  and loading = loading
  and toggle_loading = toggle_loading
  and disabled = disabled
  and toggle_disabled = toggle_disabled
  and hovered = hovered
  and observe_hover = observe_hover
  and requests = requests
  and invoke = invoke in
  let buttons =
    List.map Variant.all ~f:(fun variant ->
      let label = Variant.label variant in
      let key = Key.of_string_exn ("appearance-" ^ label) in
      let style = appearance variant p ~outline ~compact ~large ~rounded ~selected in
      let anchor =
        if Variant.equal variant Link
        then
          V.link
            ~key
            ~style
            (Link.Config.create
               ~label:(label ^ " appearance action")
               ~disabled
               ~loading
               ()
             |> ok)
            ~on_click:(invoke variant)
            [ V.text (if loading then "Loading guide…" else label) ]
          |> ok
        else
          V.button
            ~key
            ~style
            ~disabled
            ~accessible_name:(label ^ " appearance action")
            ~on_click:(invoke variant)
            label
      in
      let anchor =
        V.with_hover anchor ~on_change:(fun hovered -> observe_hover (variant, hovered))
        |> ok
      in
      let rich = Variant.equal variant Primary in
      let description = if rich then "A clear next step" else label ^ " action details" in
      V.tooltip
        ~key:(Key.of_string_exn ("hint-" ^ label))
        ~config:
          (Tooltip.Config.create
             ~label:description
             ~placement:
               (Placement.create ~side:(if rich then Right else Top) ~offset:8. () |> ok)
             ()
           |> ok)
        ~anchor
        ~content:
          (if rich
           then
             V.column
               [ Palette.text p "A clear next step"
               ; Palette.text
                   p
                   ~muted:true
                   "Use Enter or Space when this action is focused."
               ]
           else Palette.text p description)
        ())
  in
  let count, last = requests in
  Palette.card
    p
    ~title:"Actions, in context"
    [ Palette.text
        p
        ~muted:true
        "Shape the hierarchy with color, borders and spacing. Hover or focus an action \
         for help; every action here just records a preview request."
    ; V.row
        ~style:(style [ Gap (px 16.); Wrap Wrap ])
        (List.map
           [ "Outline action styles", outline, toggle_outline
           ; "Compact action spacing", compact, toggle_compact
           ; "Large action sizes", large, toggle_large
           ; "Rounded action corners", rounded, toggle_rounded
           ; "Selected action appearance", selected, toggle_selected
           ; "Disable appearance actions", disabled, toggle_disabled
           ; "Load link preview", loading, toggle_loading
           ]
           ~f:(fun (label, checked, on_toggle) ->
             V.checkbox ~state:(Check_state.of_bool checked) ~on_toggle label))
    ; V.row ~style:(style [ Gap (px 16.); Align_items Center; Wrap Wrap ]) buttons
    ; Palette.text
        p
        ~muted:true
        ("Hovered action: " ^ Option.value_map hovered ~default:"None" ~f:Variant.label)
    ; Palette.text
        p
        (sprintf
           "Style requests: %d · Last: %s"
           count
           (Option.value_map last ~default:"None" ~f:Variant.label))
    ; Palette.text
        p
        ~muted:true
        "Selected appearance changes emphasis; it does not turn an action into a toggle. \
         Link keeps its focus while loading and leaves routing to the application."
    ]
;;
