open Core
open Gpuio

let is_away = function
  | None -> false
  | Some viewport ->
    (not viewport.Virtual_list.Viewport.following_tail) && not viewport.at_end
;;

let key = Key.of_string_exn
let style = Style.create_exn
let px = Length.px_exn
let ok = Or_error.ok_exn

let view ~viewport ~jump_enabled ~motion ~fade ~jump list =
  let away = is_away viewport in
  let jump_visible = jump_enabled && away in
  let config targets =
    Animation.Config.create
      ~target:(Animation.Target.create targets |> ok)
      ~duration:(Time_ns.Span.of_ms (if motion then 200. else 0.))
      ~easing:Animation.Easing.ease_out
      ()
    |> ok
  in
  let opacity visible = if visible then 1. else 0. in
  let fade_color = Option.value fade ~default:(Color.rgb_exn 0) in
  let background =
    Background.linear_gradient
      ~angle:180.
      ~from:(Color.with_opacity fade_color 0. |> ok, 0.)
      ~to_:(fade_color, 1.)
    |> ok
  in
  let open Style.Property in
  View.column
    ~key:(key "message-follow")
    ~style:
      (style
         [ Position Relative; Min_width (px 0.); Shrink 0.; Gap (px 0.); Overflow Hidden ])
    [ list
    ; View.animate
        ~key:(key "bottom-fade")
        ~style:
          (style
             [ Position Absolute
             ; Left (px 0.)
             ; Right (px 18.)
             ; Bottom (px 0.)
             ; Height (px 48.)
             ; Background background
             ; Pointer_events false
             ])
        (config [ Opacity, opacity (away && Option.is_some fade) ])
        []
    ; View.animate
        ~key:(key "jump-motion")
        ~style:
          (style
             [ Position Absolute
             ; Left (px 0.)
             ; Right (px 18.)
             ; Height (px 48.)
             ; Overflow Hidden
             ; Pointer_events false
             ])
        (config
           [ Opacity, opacity jump_visible; (Bottom, if jump_visible then 16. else -48.) ])
        [ View.row
            ~style:
              (style
                 [ Height (Length.percent_exn 100.)
                 ; Justify_content Center
                 ; Align_items Center
                 ])
            [ View.column
                ~key:(key "jump-gate")
                ~style:(style [ Pointer_events true; Inert (not jump_visible) ])
                [ jump ]
            ]
        ]
    ]
;;
