open Core
open Gpuio
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module Otp_controller = Gpuio_eio.Otp_input
module State = Gpuio_gallery_model.Numeric_state

let ok = Or_error.ok_exn
let px = Length.px_exn
let style = Style.create_exn
let policy = Otp_input.Policy.create ~length:6 () |> ok

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
  let grouped, toggle_grouped = B.toggle ~default_model:true graph in
  let large_cells, toggle_large_cells = B.toggle ~default_model:false graph in
  let open B.Let_syntax in
  let sliders = Slider_preview.component window palette ~read_only graph in
  let numbers = Number_preview.component window palette ~read_only graph in
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
  and sliders = sliders
  and numbers = numbers
  and otp = otp
  and rating = rating
  and state = state
  and inject = inject
  and inject_rating = inject_rating
  and read_only = read_only
  and toggle_read_only = toggle_read_only
  and grouped = grouped
  and toggle_grouped = toggle_grouped
  and large_cells = large_cells
  and toggle_large_cells = toggle_large_cells
  and masked = masked
  and toggle_masked = toggle_masked in
  let control_style =
    style [ Width (px (Palette.size p 320.)); Height (px (Palette.size p 40.)) ]
  in
  let complete =
    Option.exists (Otp_controller.snapshot otp) ~f:Otp_input.Snapshot.is_complete
  in
  V.column
    ~style:(style [ Gap (px 20.) ])
    [ V.switch ~checked:read_only ~on_toggle:toggle_read_only "Read-only numeric inputs"
    ; sliders
    ; numbers
    ; Palette.card
        p
        ~title:"A six-digit verification code"
        [ Otp_controller.view
            ~style:control_style
            ~appearance:
              (Otp_input.Appearance.create
                 ~groups:(if grouped then 2 else 1)
                 ~cell_width:(Palette.size p (if large_cells then 40. else 28.))
                 ~cell_gap:(Palette.size p 4.)
                 ~group_gap:(Palette.size p 20.)
                 ~background:(Palette.surface p)
                 ~border:(Palette.border p)
                 ~focus_border:(Palette.accent p)
                 ~caret:(Palette.foreground p)
                 ()
               |> ok)
            otp
        ; V.switch ~checked:grouped ~on_toggle:toggle_grouped "Group verification cells"
        ; V.switch
            ~checked:large_cells
            ~on_toggle:toggle_large_cells
            "Larger verification cells"
        ; V.switch ~checked:masked ~on_toggle:toggle_masked "Mask verification code"
        ; Palette.text p (if complete then "Code complete" else "Waiting for six digits")
        ]
    ; Palette.card
        p
        ~title:"A quick rating"
        [ V.rating
            ?appearance:
              (if State.custom_rating_colors state
               then
                 Some
                   (Rating.Appearance.create
                      ~active:(Palette.accent p)
                      ~inactive:(Palette.muted p)
                      ())
               else None)
            ~config:rating
            ~on_request:inject_rating
            ()
        ; Palette.text
            p
            (sprintf
               "Rating: %d of %d"
               (Rating.Config.value rating)
               (Rating.Config.maximum rating))
        ; V.row
            ~style:(style [ Gap (px 8.) ])
            [ Palette.button
                p
                (sprintf "Maximum: %d stars" (Rating.Config.maximum rating))
                (inject State.Action.Cycle_rating_maximum)
            ; Palette.button
                p
                (sprintf "Star size: %.0f px" (Rating.Config.star_size rating))
                (inject State.Action.Cycle_rating_size)
            ]
        ; V.switch
            ~checked:(State.custom_rating_colors state)
            ~on_toggle:(inject State.Action.Toggle_rating_colors)
            "Separate star colors"
        ; V.switch
            ~checked:(Rating.Config.is_disabled rating)
            ~on_toggle:(inject State.Action.Toggle_rating_disabled)
            "Disable rating"
        ; V.switch
            ~checked:(State.step_down_rating state)
            ~on_toggle:(inject State.Action.Toggle_rating_step_down)
            "Step down filled stars on click"
        ]
    ]
;;
