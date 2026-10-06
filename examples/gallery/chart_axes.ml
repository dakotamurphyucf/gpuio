open Core
open Gpuio

type t =
  | Default
  | Styled
  | Floating
  | Labels_only
  | Lines_only
[@@deriving equal]

let all = [ Default; Styled; Floating; Labels_only; Lines_only ]

let label = function
  | Default -> "Default axes"
  | Styled -> "Styled axes"
  | Floating -> "Floating axes"
  | Labels_only -> "Axis labels only"
  | Lines_only -> "Axis lines only"
;;

let configuration t palette =
  let ok = Or_error.ok_exn in
  match t with
  | Default -> Chart_axis.default, Chart_axis.default, Chart_grid.default
  | Styled | Floating | Labels_only | Lines_only ->
    let tick fraction text align =
      Chart_axis.Tick.create
        ~position:(Chart_axis.Tick_position.fraction fraction |> ok)
        ~text
        ~align
        ~color:(Palette.accent palette)
        ~font_size:13.
        ()
      |> ok
    in
    let ticks =
      [ tick 0. "Start" Left; tick 0.5 "Physical midpoint" Center; tick 1. "End" Right ]
    in
    let line = not (equal t Labels_only) in
    let labels = not (equal t Lines_only) in
    let x_axis =
      Chart_axis.create
        ~line
        ~labels
        ~position:(if equal t Floating then 0.5 else 0.)
        ~ticks
        ~line_width:2.
        ~line_color:(Palette.accent palette)
        ~label_width:120.
        ~label_gap:8.
        ()
      |> ok
    in
    let y_axis =
      Chart_axis.create
        ~line
        ~labels
        ~position:(if equal t Floating then 0.5 else 1.)
        ~tick_count:4
        ~font_size:12.
        ~line_color:(Palette.foreground palette)
        ~label_color:(Palette.foreground palette)
        ()
      |> ok
    in
    let grid =
      Chart_grid.create ~dashes:[ 8.; 4.; 2. ] ~color:(Palette.muted palette) () |> ok
    in
    x_axis, y_axis, grid
;;
