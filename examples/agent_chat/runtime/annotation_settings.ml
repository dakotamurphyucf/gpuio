open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module Value = Gpuio.Color_value.Value
module Rgba = Gpuio.Color_value.Rgba
module Input = Gpuio.Color_input
module Picker = Gpuio_eio.Color_picker
module P = Gpuio.Presentation

type t = Value.t B.Expert.Var.t

let create () = B.Expert.Var.create Value.Empty
let value = B.Expert.Var.value
let ok = Or_error.ok_exn
let px = Gpuio.Length.px_exn
let style = Gpuio.Style.create_exn

let describe = function
  | Value.Empty -> "Theme accent"
  | Color color -> Rgba.to_hex color
;;

let concrete value (p : Palette.t) =
  match value with
  | Value.Empty -> p.accent
  | Color color -> Rgba.to_color color
;;

let component t ~window ~is_current ~dark graph =
  let open B.Let_syntax in
  let config =
    Input.Config.create
      ~labels:(Input.Labels.english ~control:"Diagram annotation" |> ok)
      ~allow_empty:true
      ~alpha_policy:Allow_alpha
      ~palette:
        (List.map
           [ "Iris", "#7C6FF0"; "Rose", "#EF6B9580"; "Mint", "#54C6A2" ]
           ~f:(fun (label, color) ->
             Input.Palette_entry.create ~label ~color:(Rgba.of_hex color |> ok) |> ok))
      ()
    |> ok
  in
  let picker =
    Picker.create
      window
      ~config:(B.return config)
      ~value:(value t)
      ~on_change:
        (B.map is_current ~f:(fun current color ->
           E.of_thunk (fun () -> if current () then B.Expert.Var.set t color)))
      graph
  in
  let%arr confirmed = value t
  and picker = picker
  and current = is_current
  and dark = dark in
  let p = Palette.of_dark dark in
  let appearance = if dark then P.Appearance.dark else P.Appearance.light in
  let preview =
    if Picker.is_open picker
    then Option.value_map (Picker.draft picker) ~default:confirmed ~f:Input.Snapshot.value
    else confirmed
  in
  let text value =
    V.text
      ~style:(style [ Foreground p.muted; Font_size 12.; Line_height (px 18.) ])
      value
  in
  let swatch color label =
    V.column
      ~style:(style [ Gap (px 8.); Grow 1. ])
      [ V.column
          ~style:
            (style
               [ Height (px 44.)
               ; Background (Gpuio.Background.solid p.raised)
               ; Radius 8.
               ; Border_color p.line
               ; Border_width 1.
               ; Padding (px 4.)
               ])
          [ V.column
              ~style:
                (style
                   [ Grow 1.
                   ; Radius 5.
                   ; Background (Gpuio.Background.solid (concrete color p))
                   ])
              []
          ]
      ; text (label ^ ": " ^ describe color)
      ]
  in
  V.column
    ~style:(style [ Gap (px 18.) ])
    [ P.settings_group
        appearance
        ~title:"A little color, with intent"
        ~description:
          "Choose a color for the run diagram's connectors. Preview here, then Apply to \
           update the diagram."
        [ V.row
            ~style:(style [ Gap (px 14.) ])
            [ swatch confirmed "Confirmed annotation"
            ; swatch preview "Preview annotation"
            ]
        ; Picker.view
            picker
            ~label:"Choose annotation color"
            ~apply_label:"Apply annotation"
            ~cancel_label:"Cancel annotation"
            ~overlay:
              (Gpuio.Overlay.Config.create
                 ~label:"Choose diagram annotation"
                 ~width:360.
                 ()
               |> ok)
            ~style:
              (style
                 [ Background (Gpuio.Background.solid p.surface)
                 ; Foreground p.text
                 ; Border_color p.line
                 ])
        ; text
            "Use a swatch, hex value or color channel. Alpha is preserved; Cancel keeps \
             the confirmed color."
        ; (match Picker.error picker with
           | None -> V.column []
           | Some _ ->
             text "Finish a valid color before applying, or cancel this preview.")
        ; V.button
            ~on_click:
              (E.Many
                 [ Picker.cancel picker
                 ; E.of_thunk (fun () ->
                     if current () then B.Expert.Var.set t Value.Empty)
                 ])
            ~style:
              (style
                 [ Foreground p.text
                 ; Background (Gpuio.Background.solid p.raised)
                 ; Padding (px 9.)
                 ; Radius 8.
                 ; Border_width 1.
                 ; Border_color p.line
                 ])
            "Use theme accent"
        ]
    ; P.banner
        appearance
        ~tone:Neutral
        ~live:Off
        ~title:"Your theme stays yours"
        ~style:
          (style
             [ Border_color p.line
             ; Radius 10.
             ; Background (Gpuio.Background.solid p.sidebar)
             ])
        [ text
            "A selected color stays the same in light and dark themes. Use theme accent \
             to follow the workspace palette again."
        ]
    ]
;;
