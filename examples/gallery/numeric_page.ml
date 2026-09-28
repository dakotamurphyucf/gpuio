open Core
open Gpuio
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module Slider_controller = Gpuio_eio.Slider
module Number_controller = Gpuio_eio.Number_input
module Otp_controller = Gpuio_eio.Otp_input
module State = Gpuio_gallery_model.Numeric_state

let ok = Or_error.ok_exn
let px = Length.px_exn
let style = Style.create_exn
let domain = Numeric.Domain.create ~min:0. ~max:100. ~step:1. |> ok
let policy = Otp_input.Policy.create ~length:6 () |> ok

let slider_description slider =
  Option.value_map
    (Slider_controller.snapshot slider)
    ~default:"Preparing control"
    ~f:(fun snapshot ->
      match Slider.Snapshot.value snapshot with
      | Single value -> sprintf "Level: %.0f" value
      | Range { lower; upper } -> sprintf "Interval: %.0f–%.0f" lower upper)
;;

let component window palette graph =
  let state, inject =
    B.state_machine0
      ~default_model:State.initial
      ~apply_action:(fun _ state action -> State.apply state action)
      graph
  in
  let read_only = B.map state ~f:State.is_read_only in
  let toggle_read_only =
    B.map inject ~f:(fun inject -> inject State.Action.Toggle_read_only)
  in
  let masked, toggle_masked = B.toggle ~default_model:false graph in
  let open B.Let_syntax in
  let slider_config label =
    let%arr read_only = read_only in
    Slider.Config.create ~domain ~label ~read_only () |> ok
  in
  let single =
    Slider_controller.create
      window
      ~config:(slider_config "Preview level")
      ~initial:(Slider.Value.single 35. |> ok)
      graph
  in
  let range =
    Slider_controller.create
      window
      ~config:(slider_config "Preview interval")
      ~initial:(Slider.Value.range ~lower:20. ~upper:80. |> ok)
      graph
  in
  let number_config =
    let%arr read_only = read_only in
    Number_input.Config.create ~domain ~label:"Preview quantity" ~read_only () |> ok
  in
  let number =
    Number_controller.create
      window
      ~config:number_config
      ~initial:(Number_input.Value.of_float 12. |> ok)
      graph
  in
  let otp_config =
    let%arr read_only = read_only
    and masked = masked in
    Otp_input.Config.create
      ~policy
      ~label:"Preview verification code"
      ~read_only
      ~masked
      ()
    |> ok
  in
  let otp =
    Otp_controller.create window ~config:otp_config ~initial:Otp_input.Value.empty graph
  in
  let rating = B.map state ~f:State.rating in
  let inject_rating =
    B.map inject ~f:(fun inject request -> inject (State.Action.Rate request))
  in
  let%arr p = palette
  and single = single
  and range = range
  and number = number
  and otp = otp
  and rating = rating
  and inject_rating = inject_rating
  and read_only = read_only
  and toggle_read_only = toggle_read_only
  and masked = masked
  and toggle_masked = toggle_masked in
  let control_style =
    style [ Width (px (Palette.size p 320.)); Height (px (Palette.size p 40.)) ]
  in
  let committed =
    Option.value_map
      (Number_controller.snapshot number)
      ~default:"Preparing quantity"
      ~f:(fun snapshot ->
        match Number_input.Snapshot.committed snapshot with
        | Empty -> "No quantity committed"
        | Number value -> sprintf "Committed quantity: %.0f" value)
  in
  let complete =
    Option.exists (Otp_controller.snapshot otp) ~f:Otp_input.Snapshot.is_complete
  in
  V.column
    ~style:(style [ Gap (px 20.) ])
    [ V.switch ~checked:read_only ~on_toggle:toggle_read_only "Read-only numeric inputs"
    ; Palette.card
        p
        ~title:"A precise range"
        [ Slider_controller.view ~style:control_style single
        ; Palette.text p (slider_description single)
        ; Slider_controller.view ~style:control_style range
        ; Palette.text p (slider_description range)
        ]
    ; Palette.card
        p
        ~title:"Numbers with clear intent"
        [ Number_controller.view ~style:control_style number
        ; Palette.text p committed
        ; Palette.text
            p
            ~muted:true
            "Type a draft, press Enter to commit, or use the stepper."
        ]
    ; Palette.card
        p
        ~title:"A six-digit verification code"
        [ Otp_controller.view ~style:control_style otp
        ; V.switch ~checked:masked ~on_toggle:toggle_masked "Mask verification code"
        ; Palette.text p (if complete then "Code complete" else "Waiting for six digits")
        ]
    ; Palette.card
        p
        ~title:"A quick rating"
        [ V.rating ~config:rating ~on_request:inject_rating ()
        ; Palette.text p (sprintf "Rating: %d of 5" (Rating.Config.value rating))
        ]
    ]
;;
