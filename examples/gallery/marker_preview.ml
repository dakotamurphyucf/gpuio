open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module M = Presentation.Marker

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn
let key = Key.of_string_exn

let component palette graph =
  let variant, next_variant =
    B.state_machine0
      ~default_model:M.Variant.Plain
      ~apply_action:(fun _ v () ->
        match v with
        | Plain -> Separator
        | Separator -> Border
        | Border -> Plain)
      graph
  in
  let loading_style, toggle_loading_style =
    B.state_machine0
      ~default_model:M.Loading_style.Spinner
      ~apply_action:(fun _ v () ->
        match v with
        | Spinner -> Shimmer
        | Shimmer -> Spinner)
      graph
  in
  let icon, next_icon =
    B.state_machine0 ~default_model:0 ~apply_action:(fun _ i () -> (i + 1) % 3) graph
  in
  let loading, toggle_loading = B.toggle ~default_model:false graph in
  let typed, toggle_typed = B.toggle ~default_model:true graph in
  let rich, toggle_rich = B.toggle ~default_model:true graph in
  let empty, toggle_empty = B.toggle ~default_model:false graph in
  let refined, toggle_refined = B.toggle ~default_model:false graph in
  let compact, toggle_compact = B.toggle ~default_model:false graph in
  let actions, act =
    B.state_machine0 ~default_model:0 ~apply_action:(fun _ n () -> n + 1) graph
  in
  let open B.Let_syntax in
  let%arr p = palette
  and variant = variant
  and next_variant = next_variant
  and loading_style = loading_style
  and toggle_loading_style = toggle_loading_style
  and icon = icon
  and next_icon = next_icon
  and loading = loading
  and toggle_loading = toggle_loading
  and typed = typed
  and toggle_typed = toggle_typed
  and rich = rich
  and toggle_rich = toggle_rich
  and empty = empty
  and toggle_empty = toggle_empty
  and refined = refined
  and toggle_refined = toggle_refined
  and compact = compact
  and toggle_compact = toggle_compact
  and actions = actions
  and act = act in
  let variant_name =
    match variant with
    | Plain -> "Plain"
    | Separator -> "Separator"
    | Border -> "Border"
  in
  let loading_name =
    match loading_style with
    | Spinner -> "Spinner"
    | Shimmer -> "Shimmer"
  in
  let icon_name =
    match icon with
    | 0 -> "None"
    | 1 -> "Custom"
    | _ -> "Empty"
  in
  let checkbox label checked on_toggle =
    V.checkbox ~state:(if checked then Checked else Unchecked) ~on_toggle label
  in
  let action =
    V.button
      ~key:(key "action")
      ~style:
        (style
           [ Font_size 13.
           ; Padding (px 7.)
           ; Radius 7.
           ; Background (Background.solid (Palette.background p))
           ; Foreground (Palette.foreground p)
           ])
      ~on_click:(act ())
      "Marker action"
  in
  let content_style =
    Style.merge
      [ style [ Gap (px 10.); Align_items Center ]
      ; (if refined
         then
           Style.create_exn
             [ Opacity 0.65
             ; Padding (px 6.)
             ; Radius 8.
             ; Background (Background.solid (Palette.background p))
             ]
           |> fun s -> Style.with_state_exn s Hovered [ Opacity 0.9 ]
         else Style.empty)
      ]
  in
  let content =
    M.Content.create
      ~key:(key "content")
      ~style:content_style
      ((if typed
        then
          [ M.Content.Item.text
              ~key:(key "text")
              ~style:(style [ User_select true ])
              (if empty then "" else "Thinking · 京都")
            |> ok
          ]
        else [])
       @ if rich then [ M.Content.Item.element ~key:(key "action") action ] else [])
    |> ok
    |> M.Item.content
  in
  let marker =
    M.create
      (Palette.appearance p)
      ~key:(key "marker")
      ~variant
      ~loading
      ~loading_style
      ~spinner:(M.Spinner.create ~label:"Marker activity" () |> ok)
      ~style:(style [ Min_height (px 42.); Font_size 14. ])
      ~separator_style:
        (if refined
         then style [ Height (px 2.); Background (Background.solid (Palette.accent p)) ]
         else Style.empty)
      ((if icon = 0
        then []
        else
          [ M.Icon.create ~key:(key "icon") (if icon = 1 then [ V.text "◇" ] else [])
            |> M.Item.icon
          ])
       @ [ content
         ; M.Item.element
             ~key:(key "steady")
             (V.text
                ~style:(style [ Font_size 11.; Foreground (Palette.muted p) ])
                "Steady")
         ])
    |> ok
  in
  let marker =
    V.with_accessibility
      marker
      (Accessibility.create ~role:Group ~label:"Marker preview" () |> ok)
    |> ok
  in
  V.column
    ~style:(style [ Gap (px 12.); Width (Length.percent_exn 100.) ])
    [ Palette.text
        p
        ~muted:true
        "A quiet signal of work in progress. Your controls stay in place."
    ; V.column
        ~style:
          (style
             [ Width (px (if compact then 380. else 560.))
             ; Max_width (Length.percent_exn 100.)
             ; Padding (px 14.)
             ; Radius 12.
             ; Background (Background.solid (Palette.surface p))
             ])
        [ marker ]
    ; V.row
        ~style:(style [ Gap (px 8.); Wrap Wrap ])
        [ Palette.button p ("Marker variant: " ^ variant_name) (next_variant ())
        ; Palette.button p ("Marker loading: " ^ loading_name) (toggle_loading_style ())
        ; Palette.button p ("Marker icon: " ^ icon_name) (next_icon ())
        ]
    ; V.row
        ~style:(style [ Gap (px 14.); Wrap Wrap ])
        [ checkbox "Marker busy" loading toggle_loading
        ; checkbox "Marker typed text" typed toggle_typed
        ; checkbox "Marker rich content" rich toggle_rich
        ; checkbox "Empty marker text" empty toggle_empty
        ]
    ; V.row
        ~style:(style [ Gap (px 14.); Wrap Wrap ])
        [ checkbox "Refine marker styles" refined toggle_refined
        ; checkbox "Compact marker" compact toggle_compact
        ]
    ; Palette.text p ~muted:true (sprintf "Marker actions: %d" actions)
    ]
;;
