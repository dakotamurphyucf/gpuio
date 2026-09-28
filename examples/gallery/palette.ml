open Core
open Gpuio
module Appearance = Gpuio_gallery_model.Appearance
module V = Gpuio_bonsai.View

type t =
  { background : Color.t
  ; surface : Color.t
  ; foreground : Color.t
  ; muted : Color.t
  ; accent : Color.t
  ; border : Color.t
  ; appearance : Presentation.Appearance.t
  ; factor : float
  }

let create appearance scale =
  let background, surface, foreground, muted, accent, border, appearance =
    match appearance with
    | Appearance.Dark ->
      ( 0x10151d
      , 0x19212c
      , 0xeaf0f7
      , 0x98a7bd
      , 0x89ddc9
      , 0x2d3949
      , Presentation.Appearance.dark )
    | Light ->
      ( 0xf1f4f7
      , 0xffffff
      , 0x1b2939
      , 0x52667d
      , 0x096e5b
      , 0xd3dce5
      , Presentation.Appearance.light )
  in
  { background = Color.rgb_exn background
  ; surface = Color.rgb_exn surface
  ; foreground = Color.rgb_exn foreground
  ; muted = Color.rgb_exn muted
  ; accent = Color.rgb_exn accent
  ; border = Color.rgb_exn border
  ; appearance
  ; factor = Appearance.Scale.factor scale
  }
;;

let background t = t.background
let surface t = t.surface
let foreground t = t.foreground
let muted t = t.muted
let accent t = t.accent
let border t = t.border
let appearance t = t.appearance

let theme t =
  Theme.create
    [ "background", t.surface
    ; "foreground", t.foreground
    ; "accent", t.accent
    ; "muted", t.muted
    ]
  |> Or_error.ok_exn
;;

let size t value = value *. t.factor
let px = Length.px_exn
let style = Style.create_exn

let text t ?(size = 14.) ?(muted = false) value =
  V.text
    ~style:
      (style
         [ Font_size (size *. t.factor)
         ; Foreground (if muted then t.muted else t.foreground)
         ])
    value
;;

let card t ~title children =
  V.column
    ~style:
      (style
         [ Background (Background.solid t.surface)
         ; Border_width 1.
         ; Border_color t.border
         ; Radius 14.
         ; Padding (px (size t 22.))
         ; Gap (px (size t 16.))
         ; Shrink 0.
         ])
    (V.text
       ~style:(style [ Font_size (size t 17.); Font_weight 600; Foreground t.foreground ])
       title
     :: children)
;;

let button t ?(selected = false) label on_click =
  V.button
    label
    ~on_click
    ~style:
      (style
         [ Padding (px (size t 10.))
         ; Radius 8.
         ; Font_size (size t 13.)
         ; Background (Background.solid (if selected then t.border else t.surface))
         ; Foreground (if selected then t.accent else t.foreground)
         ]
       |> fun s ->
       Style.with_state_exn s Hovered [ Background (Background.solid t.border) ])
;;
