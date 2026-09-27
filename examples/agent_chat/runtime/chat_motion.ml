open Core
module A = Gpuio.Animation
module V = Gpuio_bonsai.View

let ok = Or_error.ok_exn
let px = Gpuio.Length.px_exn
let style = Gpuio.Style.create_exn
let target property value = A.Target.create [ property, value ] |> ok
let stage timing target = A.Stage.create ~timing ~target () |> ok
let tween ms = A.Timing.tween ~easing:A.Easing.ease_out (Time_ns.Span.of_ms ms) |> ok

let spring =
  A.Spring.create
    ~stiffness:210.
    ~damping:27.
    ~mass:1.
    ~epsilon:0.05
    ~max_duration:(Time_ns.Span.of_sec 1.5)
    ()
  |> ok
;;

let stage_context ~expanded children =
  let program =
    A.Program.create
      [ stage (A.Timing.spring spring) (target Height (if expanded then 112. else 0.)) ]
    |> ok
  in
  V.animate_program
    ~key:(Gpuio.Key.of_string_exn "stage-context-motion")
    ~style:(style [ Shrink 0.; Overflow_y Hidden; Width (Gpuio.Length.percent_exn 100.) ])
    program
    [ V.panel
        ~key:(Gpuio.Key.of_string_exn "stage-context")
        ~label:"Stage context"
        ~active:expanded
        ~hidden:Unmount
        ~style:(style [ Height (px 112.); Shrink 0.; Gap (px 8.); Padding_top (px 8.) ])
        children
    ]
;;

let destination ~index content =
  let program =
    A.Program.create
      ~initial:(target Opacity 0.35)
      ~delay:
        (Time_ns.Span.of_ms (Float.of_int (Int.clamp_exn index ~min:0 ~max:8) *. 55.))
      [ stage (tween 100.) (target Opacity 0.72); stage (tween 170.) (target Opacity 1.) ]
    |> ok
  in
  V.animate_program
    ~key:(Gpuio.Key.of_string_exn ("destination-" ^ Int.to_string index))
    ~style:(style [ Shrink 0. ])
    program
    [ content ]
;;

let activity ~key ~group content =
  let program =
    A.Program.create
      ~initial:(target Opacity 1.)
      ~repeat:Alternate
      ~clock:(A.Clock.group group |> ok)
      [ stage (tween 650.) (target Opacity 0.35) ]
    |> ok
  in
  V.animate_program
    ~key:(Gpuio.Key.of_string_exn key)
    ~style:(style [ Shrink 0. ])
    program
    [ content ]
;;
