open Core
open Gpuio
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module Registered = Gpuio_eio.Asset

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn

(* Original circular arrow; asymmetric artwork makes rotation easy to see. *)
let arrow =
  {|<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32"><path d="M25 16a9 9 0 1 1-5-8" fill="none" stroke="black" stroke-width="3" stroke-linecap="round"/><path d="M18 3l8 6-10 2z"/></svg>|}
;;

let component app window palette graph =
  let assets =
    Preview_scope.acquire
      window
      ~name:"gallery custom spinner"
      ~create:(fun scope ->
        let register bytes =
          E.map
            (Registered.register
               app
               ~scope
               (Asset.Source.of_bytes ~format:Svg bytes |> ok))
            ~f:(fun result ->
              Result.map result ~f:Registered.handle
              |> Result.map_error ~f:(fun error ->
                Error.create_s [%sexp (error : Registered.Error.t)]))
        in
        E.bind (register arrow) ~f:(function
          | Error error -> E.return (Error error)
          | Ok arrow ->
            E.map
              (register "Invalid SVG")
              ~f:(Result.map ~f:(fun invalid -> arrow, invalid))))
      graph
  in
  let animated, toggle_animated = B.toggle ~default_model:false graph in
  let slow, toggle_slow = B.toggle ~default_model:false graph in
  let linear, toggle_linear = B.toggle ~default_model:false graph in
  let mode, next_mode =
    B.state_machine0
      ~default_model:1
      ~apply_action:(fun _ mode () -> (mode + 1) % 3)
      graph
  in
  let icon_state, set_icon_state = B.state Image.State.Loading graph in
  let open B.Let_syntax in
  let%arr p = palette
  and assets = assets
  and animated = animated
  and toggle_animated = toggle_animated
  and slow = slow
  and toggle_slow = toggle_slow
  and linear = linear
  and toggle_linear = toggle_linear
  and mode = mode
  and next_mode = next_mode
  and icon_state = icon_state
  and set_icon_state = set_icon_state in
  match assets with
  | Preview_scope.Loading -> Palette.text p "Preparing spinner previews…"
  | Failed error -> Palette.text p (Error.to_string_hum error)
  | Ready (arrow, invalid) ->
    let icon =
      match mode with
      | 0 -> None
      | 1 -> Some arrow
      | _ -> Some invalid
    in
    let source_label =
      match mode with
      | 0 -> "Built-in strokes"
      | 1 -> "Circular arrow"
      | _ -> "Unavailable icon"
    in
    let status =
      match mode, icon_state with
      | 0, _ -> "Built-in indicator"
      | _, Image.State.Loading -> "Loading icon…"
      | _, Ready _ -> "Icon ready"
      | _, Failed _ -> "Icon unavailable · showing fallback"
    in
    Palette.card
      p
      ~title:"Custom spinners"
      [ V.row
          ~style:
            (style [ Gap (px 20.); Align_items Center; Foreground (Palette.accent p) ])
          [ V.spinner
              ~key:(Key.of_string_exn "custom-spinner")
              ~style:(style [ Width (px 40.); Height (px 40.) ])
              ~on_icon_change:set_icon_state
              ~config:
                (Spinner.Config.create
                   ~label:"Spinner preview"
                   ?icon
                   ~animated
                   ~period:(Time_ns.Span.of_ms (if slow then 2000. else 800.))
                   ~easing:
                     (if linear
                      then Animation.Easing.linear
                      else Animation.Easing.ease_in_out)
                   ()
                 |> ok)
              ()
          ; Palette.text p status
          ]
      ; V.switch ~checked:animated ~on_toggle:toggle_animated "Animate custom spinner"
      ; V.row
          ~style:(style [ Gap (px 8.); Wrap Wrap ])
          [ Palette.button p source_label (next_mode ())
          ; Palette.button
              p
              (if slow then "Cycle: 2 seconds" else "Cycle: 800 ms")
              toggle_slow
          ; Palette.button
              p
              (if linear then "Linear easing" else "Ease in and out")
              toggle_linear
          ]
      ; Palette.text
          p
          ~muted:true
          "The icon follows the theme color. Reduced motion keeps it still."
      ]
;;
