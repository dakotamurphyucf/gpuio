open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module A = Presentation.Alert

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn
let key = Key.of_string_exn

module Icon_choice = struct
  type t =
    | Default
    | Custom
    | Hidden
end

let component palette graph =
  let variant, next_variant =
    B.state_machine0
      ~default_model:A.Variant.Default
      ~apply_action:(fun _ v () ->
        match v with
        | Default -> Info
        | Info -> Success
        | Success -> Warning
        | Warning -> Error
        | Error -> Default)
      graph
  in
  let size, next_size =
    B.state_machine0
      ~default_model:A.Size.Medium
      ~apply_action:(fun _ v () ->
        match v with
        | XSmall -> Small
        | Small -> Medium
        | Medium -> Large
        | Large -> XSmall)
      graph
  in
  let icon, next_icon =
    B.state_machine0
      ~default_model:Icon_choice.Default
      ~apply_action:(fun _ v () ->
        match v with
        | Default -> Icon_choice.Custom
        | Custom -> Hidden
        | Hidden -> Default)
      graph
  in
  let banner, toggle_banner = B.toggle ~default_model:false graph in
  let title, toggle_title = B.toggle ~default_model:true graph in
  let close, toggle_close = B.toggle ~default_model:true graph in
  let disabled, toggle_disabled = B.toggle ~default_model:false graph in
  let refined, toggle_refined = B.toggle ~default_model:false graph in
  let compact, toggle_compact = B.toggle ~default_model:false graph in
  let visible, set_visible = B.state true graph in
  let actions, act =
    B.state_machine0 ~default_model:0 ~apply_action:(fun _ n () -> n + 1) graph
  in
  let dismissals, dismiss =
    B.state_machine0 ~default_model:0 ~apply_action:(fun _ n () -> n + 1) graph
  in
  let open B.Let_syntax in
  let%arr p = palette
  and variant = variant
  and next_variant = next_variant
  and size = size
  and next_size = next_size
  and icon = icon
  and next_icon = next_icon
  and banner = banner
  and toggle_banner = toggle_banner
  and title = title
  and toggle_title = toggle_title
  and close = close
  and toggle_close = toggle_close
  and disabled = disabled
  and toggle_disabled = toggle_disabled
  and refined = refined
  and toggle_refined = toggle_refined
  and compact = compact
  and toggle_compact = toggle_compact
  and visible = visible
  and set_visible = set_visible
  and actions = actions
  and act = act
  and dismissals = dismissals
  and dismiss = dismiss in
  let variant_name =
    match variant with
    | Default -> "Default"
    | Info -> "Info"
    | Success -> "Success"
    | Warning -> "Warning"
    | Error -> "Error"
  in
  let size_name =
    match size with
    | XSmall -> "XS"
    | Small -> "S"
    | Medium -> "M"
    | Large -> "L"
  in
  let icon_name, icon =
    match icon with
    | Icon_choice.Default -> "Default", A.Icon.Default
    | Hidden -> "Hidden", A.Icon.Hidden
    | Custom -> "Custom", A.Icon.Custom (V.text "◇")
  in
  let checkbox label checked on_toggle =
    V.checkbox ~state:(if checked then Checked else Unchecked) ~on_toggle label
  in
  let alert =
    A.create
      (Palette.appearance p)
      ~key:(key "alert")
      ~variant
      ~size
      ~icon
      ~layout:(if banner then Banner else Card)
      ~visible
      ?title:(Option.some_if title (A.title "A small interruption, a clear next step"))
      ?close:
        (Option.some_if
           close
           (A.Close.create
              ~label:"Dismiss alert"
              ~disabled
              ~on_click:(fun () -> Bonsai.Effect.Many [ dismiss (); set_visible false ])
              ()
            |> ok))
      ~style:
        (if refined
         then style [ Radius 3.; Border_width 2.; Border_color (Palette.accent p) ]
         else Style.empty)
      ~body_style:(if refined then style [ Gap (px 9.) ] else Style.empty)
      [ V.text
          ~key:(key "message")
          ~style:(style [ User_select true ])
          "Your workspace is safe. Review the details, then continue where you left off. \
           京都"
      ; V.row ~key:(key "actions") [ Palette.button p "Alert action" (act ()) ]
      ]
  in
  let alert =
    if visible
    then
      V.with_accessibility
        alert
        (Accessibility.create ~role:Alert ~live:Off ~label:"Alert preview" () |> ok)
      |> ok
    else alert
  in
  V.column
    ~style:(style [ Gap (px 12.); Width (Length.percent_exn 100.) ])
    [ Palette.text p ~muted:true "Clear feedback, with the next step close at hand."
    ; V.column
        ~style:
          (style
             [ Width (px (if compact then 350. else 560.))
             ; Max_width (Length.percent_exn 100.)
             ])
        [ alert ]
    ; V.row
        ~style:(style [ Gap (px 8.); Wrap Wrap ])
        [ Palette.button p ("Alert variant: " ^ variant_name) (next_variant ())
        ; Palette.button p ("Alert size: " ^ size_name) (next_size ())
        ; Palette.button p ("Alert icon: " ^ icon_name) (next_icon ())
        ]
    ; V.row
        ~style:(style [ Gap (px 14.); Wrap Wrap ])
        [ checkbox "Alert banner" banner toggle_banner
        ; checkbox "Alert title" title toggle_title
        ; checkbox "Alert close control" close toggle_close
        ; checkbox "Disable alert close" disabled toggle_disabled
        ]
    ; V.row
        ~style:(style [ Gap (px 14.); Wrap Wrap ])
        [ checkbox "Refine alert styles" refined toggle_refined
        ; checkbox "Compact alert" compact toggle_compact
        ; Palette.button p "Restore alert" (set_visible true)
        ]
    ; Palette.text p ~muted:true (sprintf "Alert actions: %d" actions)
    ; Palette.text p ~muted:true (sprintf "Alert dismissals: %d" dismissals)
    ]
;;
