open Core
module Brush = Gpuio_protocol.Chart_appearance_wire.Brush

let resolve brush ~theme =
  let open Or_error.Let_syntax in
  match Background.Expert.describe brush with
  | Solid color ->
    let%map color = Theme.resolve theme color in
    Brush.Solid color
  | Pattern_slash (color, width, interval) ->
    let%map color = Theme.resolve theme color in
    Brush.Pattern_slash (color, width, interval)
  | Checkerboard (color, size) ->
    let%map color = Theme.resolve theme color in
    Brush.Checkerboard (color, size)
  | Linear_gradient (space, angle, (from, start), (to_, stop)) ->
    let%bind from = Theme.resolve theme from in
    let%map to_ = Theme.resolve theme to_ in
    Brush.Linear
      { oklab = Background.Color_space.equal space Oklab; angle; from; start; to_; stop }
;;
