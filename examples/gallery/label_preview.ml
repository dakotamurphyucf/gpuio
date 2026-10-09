open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module P = Presentation

type mode =
  | Plain
  | Prefix
  | All
[@@deriving equal]

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn
let primary = "İstanbul · Élan · agent"
let secondary = "agent notes · 世界"

let component palette graph =
  let mode, set_mode = B.state All graph in
  let secondary_enabled, toggle_secondary = B.toggle ~default_model:true graph in
  let masked, toggle_masked = B.toggle ~default_model:false graph in
  let expanded, toggle_width = B.toggle ~default_model:false graph in
  let open B.Let_syntax in
  let%arr p = palette
  and mode = mode
  and set_mode = set_mode
  and secondary_enabled = secondary_enabled
  and toggle_secondary = toggle_secondary
  and masked = masked
  and toggle_masked = toggle_masked
  and expanded = expanded
  and toggle_width = toggle_width in
  let highlight =
    match mode with
    | Plain -> None
    | Prefix -> Some (Label.Match.prefix "i" |> ok)
    | All -> Some (Label.Match.all "AGENT" |> ok)
  in
  let config =
    Label.create
      ?secondary:(Option.some_if secondary_enabled secondary)
      ?highlight
      ~masked
      primary
    |> ok
  in
  let label =
    P.styled_label
      (Palette.appearance p)
      ~key:(Key.of_string_exn "styled-label")
      ~style:
        (style
           [ User_select true
           ; Font_size 20.
           ; Line_height (px 28.)
           ; Width (px (if expanded then 540. else 300.))
           ; Min_width (px 0.)
           ])
      config
  in
  let choices =
    List.map
      [ Plain, "Label: plain"; Prefix, "Label: prefix i"; All, "Label: all agent" ]
      ~f:(fun (choice, title) ->
        Palette.button p ~selected:(equal_mode mode choice) title (set_mode choice))
  in
  V.column
    ~style:(style [ Gap (px 12.); Width (Length.percent_exn 100.) ])
    [ Palette.text p ~size:16. "Labels that read naturally"
    ; Palette.text
        p
        "One selectable text flow, with secondary text and Unicode match coloring."
    ; V.row ~style:(style [ Gap (px 8.); Wrap Wrap ]) choices
    ; V.row
        ~style:(style [ Gap (px 12.); Wrap Wrap ])
        [ V.checkbox
            ~state:(if secondary_enabled then Checked else Unchecked)
            ~on_toggle:toggle_secondary
            "Label secondary text"
        ; V.checkbox
            ~state:(if masked then Checked else Unchecked)
            ~on_toggle:toggle_masked
            "Mask label"
        ; Palette.button
            p
            (if expanded then "Label width: wide" else "Label width: compact")
            toggle_width
        ]
    ; V.column
        ~style:
          (style
             [ Padding (px 16.)
             ; Radius 10.
             ; Background (Background.solid (Palette.background p))
             ; Min_width (px 0.)
             ])
        [ label ]
    ]
;;
