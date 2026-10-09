open Core
open Gpuio
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module Controller = Gpuio_eio.Slider

let ok = Or_error.ok_exn
let linear_domain = Numeric.Domain.create ~min:0. ~max:100. ~step:1. |> ok
let log_domain = Numeric.Domain.create ~min:1. ~max:1000. ~step:1. |> ok
let single_initial = Slider.Value.single 35. |> ok
let range_initial = Slider.Value.range ~lower:20. ~upper:80. |> ok

let value_text = function
  | Slider.Value.Single value -> sprintf "%.3g" value
  | Range { lower; upper } -> sprintf "%.3g–%.3g" lower upper
;;

let description controller =
  Option.value_map
    (Controller.snapshot controller)
    ~default:"Preparing slider"
    ~f:(fun snapshot ->
      sprintf
        "%s: %s · committed: %s"
        (if Option.is_some (Slider.Snapshot.dragging snapshot) then "Preview" else "Value")
        (value_text (Slider.Snapshot.value snapshot))
        (value_text (Slider.Snapshot.committed snapshot)))
;;

let error_message = function
  | Slider.Command_error.Not_mounted | Closed | Stale_slider ->
    "The slider is no longer available"
  | Stale_revision -> "The slider changed — try again"
  | Disabled | Read_only | Focus_blocked ->
    "The slider cannot receive that input right now"
  | Busy -> "The slider is busy — try again"
  | Wrong_mode
  | Wrong_thumb
  | Invalid_value
  | Invalid_config
  | Limit_exceeded
  | Native_failure -> "The slider request could not be applied"
;;

let component window palette ~read_only graph =
  let vertical, toggle_vertical = B.toggle ~default_model:false graph in
  let logarithmic, toggle_logarithmic = B.toggle ~default_model:false graph in
  let disabled, toggle_disabled = B.toggle ~default_model:false graph in
  let remaining, toggle_remaining = B.toggle ~default_model:false graph in
  let custom_colors, toggle_colors = B.toggle ~default_model:false graph in
  let large, toggle_large = B.toggle ~default_model:false graph in
  let notice, set_notice =
    B.state "Hover or drag a thumb; Escape cancels the preview" graph
  in
  let epoch = B.Expert.thunk ~f:(fun () -> ref 0) graph in
  B.Edge.lifecycle
    ~on_deactivate:(B.map epoch ~f:(fun epoch -> E.of_thunk (fun () -> incr epoch)))
    graph;
  let open B.Let_syntax in
  let config label =
    let%arr read_only = read_only
    and vertical = vertical
    and logarithmic = logarithmic
    and disabled = disabled in
    Slider.Config.create
      ~domain:(if logarithmic then log_domain else linear_domain)
      ~label
      ~axis:(if vertical then Vertical else Horizontal)
      ~scale:(if logarithmic then Logarithmic else Linear)
      ~disabled
      ~read_only
      ()
    |> ok
  in
  let single =
    Controller.create
      window
      ~config:(config "Preview level")
      ~initial:single_initial
      graph
  in
  let range =
    Controller.create
      window
      ~config:(config "Preview interval")
      ~initial:range_initial
      graph
  in
  let%arr p = palette
  and single = single
  and range = range
  and vertical = vertical
  and toggle_vertical = toggle_vertical
  and logarithmic = logarithmic
  and toggle_logarithmic = toggle_logarithmic
  and disabled = disabled
  and toggle_disabled = toggle_disabled
  and remaining = remaining
  and toggle_remaining = toggle_remaining
  and custom_colors = custom_colors
  and toggle_colors = toggle_colors
  and large = large
  and toggle_large = toggle_large
  and notice = notice
  and set_notice = set_notice
  and epoch = epoch in
  let command controller action success =
    let open E.Let_syntax in
    let%bind current =
      E.of_thunk (fun () ->
        incr epoch;
        !epoch)
    in
    let%bind result = Controller.command controller action in
    let%bind publish = E.of_thunk (fun () -> Int.equal current !epoch) in
    if publish
    then
      set_notice
        (match result with
         | Ok _ -> success
         | Error error -> error_message error)
    else E.Ignore
  in
  let row_style = Style.create_exn [ Gap (Length.px_exn 8.); Wrap Wrap ] in
  let control_style =
    Style.create_exn
      [ Width (Length.px_exn (Palette.size p (if vertical then 48. else 320.)))
      ; Height (Length.px_exn (Palette.size p (if vertical then 180. else 40.)))
      ; Foreground (Palette.accent p)
      ]
  in
  let action controller label action success =
    V.button
      ~config:(Button.Config.create ~focus:Preserve ())
      ~on_click:(command controller action success)
      label
  in
  let appearance =
    Slider.Appearance.create
      ~fill:(if remaining then Remaining else Selected)
      ~track_thickness:(Palette.size p (if large then 6. else 4.))
      ~track_radius:(Palette.size p (if large then 3. else 2.))
      ~thumb_size:(Palette.size p (if large then 24. else 12.))
      ~target_size:(Palette.size p (if large then 40. else 20.))
      ~ring_width:(Palette.size p 2.)
      ?track_color:(if custom_colors then Some (Palette.border p) else None)
      ?fill_color:(if custom_colors then Some (Palette.accent p) else None)
      ?thumb_color:(if custom_colors then Some (Palette.foreground p) else None)
      ?ring_color:(if custom_colors then Some (Palette.accent p) else None)
      ()
    |> ok
  in
  let preview controller title initial thumbs =
    V.column
      ~style:(Style.create_exn [ Gap (Length.px_exn 8.) ])
      [ Palette.text p title
      ; Controller.view ~style:control_style ~appearance controller
      ; Palette.text p ~muted:true (description controller)
      ; V.row
          ~style:row_style
          ([ action
               controller
               "Reset values"
               (Replace { value = initial; if_revision = None })
               "Values reset"
           ; action controller "Cancel drag" Cancel_drag "Returned to the committed value"
           ]
           @ List.map thumbs ~f:(fun (label, thumb) ->
             action controller label (Focus thumb) "Use arrows, Page Up/Down, Home or End")
          )
      ]
  in
  Palette.card
    p
    ~title:"A precise range"
    [ Palette.text
        p
        ~muted:true
        (if logarithmic
         then "1–1,000 · logarithmic positioning · whole-number steps"
         else "0–100 · linear positioning · whole-number steps")
    ; preview single "Single value" single_initial [ "Focus level", Slider.Thumb.Single ]
    ; preview
        range
        "Two independent thumbs"
        range_initial
        [ "Focus lower", Slider.Thumb.Lower; "Focus upper", Upper ]
    ; V.switch ~checked:vertical ~on_toggle:toggle_vertical "Vertical sliders"
    ; V.switch ~checked:logarithmic ~on_toggle:toggle_logarithmic "Logarithmic scale"
    ; V.switch ~checked:disabled ~on_toggle:toggle_disabled "Disable sliders"
    ; V.switch
        ~checked:remaining
        ~on_toggle:toggle_remaining
        "Fill the remaining amount (single value)"
    ; V.switch
        ~checked:custom_colors
        ~on_toggle:toggle_colors
        "Separate track and thumb colors"
    ; V.switch ~checked:large ~on_toggle:toggle_large "Larger slider thumbs"
    ; Palette.text
        p
        ~muted:true
        "Changing scale or direction cancels an active drag; values stay in range"
    ; Palette.text p ~muted:true notice
    ]
;;
