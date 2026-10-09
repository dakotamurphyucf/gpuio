open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module Number = Gpuio_eio.Number_input

let domain = Numeric.Domain.create ~min:0. ~max:100. ~step:0.25 |> Or_error.ok_exn
let initial = Number_input.Value.of_float 12. |> Or_error.ok_exn
let unfinished = Number_input.Draft.of_string "1e-" |> Or_error.ok_exn

let classification snapshot =
  if Option.is_some (Number_input.Snapshot.composition snapshot)
  then "Composing…"
  else (
    match Number_input.Snapshot.classification snapshot with
    | Empty -> "Empty draft"
    | Incomplete -> "Keep typing — this number is unfinished"
    | Invalid _ -> "This draft is not a finite number"
    | Valid _ -> "Ready to commit"
    | Out_of_range _ -> "Commit will bring this number into range")
;;

let error_message = function
  | Number_input.Command_error.Composing | Rejected Composing ->
    "Finish composing before changing this number"
  | Rejected Empty_required -> "Enter a number, or allow an empty value"
  | Rejected Incomplete -> "Finish the number before committing"
  | Rejected (Syntax | Non_finite) | Invalid_text | Invalid_value ->
    "Enter a finite number before committing"
  | Disabled | Read_only | Focus_blocked -> "This control is not editable right now"
  | Stale_input | Stale_revision -> "The field changed — try again"
  | Not_mounted | Closed -> "The field is no longer available"
  | Busy -> "The field is busy — try again"
  | Invalid_selection | Limit_exceeded | Native_failure | Invalid_config ->
    "The request could not be applied"
;;

let adaptive_resolution request =
  let snapshot = Number_input.Step_request.snapshot request in
  let domain = Number_input.Snapshot.domain snapshot in
  let proposal =
    let open Or_error.Let_syntax in
    let%bind value =
      match Number_input.Snapshot.classification snapshot with
      | Empty -> Ok 0.
      | Valid value | Out_of_range value -> Numeric.Domain.normalize domain value
      | Incomplete | Invalid _ -> Or_error.error_string "Finish the numeric draft"
    in
    let amount =
      if Float.(value < 10.) then 0.25 else if Float.(value < 50.) then 1. else 5.
    in
    let next =
      match Number_input.Step_request.direction request with
      | Increase -> value +. amount
      | Decrease -> value -. amount
    in
    Number_input.Value.of_float next
  in
  match proposal with
  | Ok value -> Number_input.Step_resolution.Apply value
  | Error _ -> Decline
;;

let component window palette ~read_only graph =
  let controls, set_controls = B.state Number_input.Step_controls.Sides graph in
  let allow_empty, toggle_empty = B.toggle ~default_model:false graph in
  let disabled, toggle_disabled = B.toggle ~default_model:false graph in
  let framed, toggle_framed = B.toggle ~default_model:true graph in
  let custom_step, toggle_step = B.toggle ~default_model:false graph in
  let custom_symbols, toggle_symbols = B.toggle ~default_model:false graph in
  let notice, set_notice =
    B.state "A draft can change without changing its value" graph
  in
  let request_epoch = B.Expert.thunk ~f:(fun () -> ref 0) graph in
  B.Edge.lifecycle
    ~on_deactivate:
      (B.map request_epoch ~f:(fun epoch -> B.Effect.of_thunk (fun () -> Int.incr epoch)))
    graph;
  let open B.Let_syntax in
  let config =
    let%arr read_only = read_only
    and disabled = disabled
    and controls = controls
    and allow_empty = allow_empty
    and custom_step = custom_step in
    Number_input.Config.create
      ~domain
      ~label:"Preview quantity"
      ~step_mode:(if custom_step then Application else Native)
      ~step_controls:controls
      ~allow_empty
      ~read_only
      ~disabled
      ()
    |> Or_error.ok_exn
  in
  let step_scope =
    Preview_scope.acquire
      window
      ~name:"numeric-preview"
      ~create:(fun scope -> B.Effect.return (Ok scope))
      graph
  in
  let number = Number.create window ~config ~initial graph in
  let%arr p = palette
  and number = number
  and step_scope = step_scope
  and controls = controls
  and set_controls = set_controls
  and allow_empty = allow_empty
  and toggle_empty = toggle_empty
  and disabled = disabled
  and toggle_disabled = toggle_disabled
  and framed = framed
  and toggle_framed = toggle_framed
  and custom_step = custom_step
  and toggle_step = toggle_step
  and custom_symbols = custom_symbols
  and toggle_symbols = toggle_symbols
  and read_only = read_only
  and request_epoch = request_epoch
  and notice = notice
  and set_notice = set_notice in
  let command action success =
    let open B.Effect.Let_syntax in
    let%bind epoch =
      B.Effect.of_thunk (fun () ->
        Int.incr request_epoch;
        !request_epoch)
    in
    let%bind result = Number.command number action in
    let%bind current = B.Effect.of_thunk (fun () -> Int.equal epoch !request_epoch) in
    if current
    then
      set_notice
        (match result with
         | Ok _ -> success
         | Error error -> error_message error)
    else B.Effect.Ignore
  in
  let snapshot = Number.snapshot number in
  let committed =
    match Option.map snapshot ~f:Number_input.Snapshot.committed with
    | None -> "Preparing quantity"
    | Some Empty -> "No quantity committed"
    | Some (Number value) -> sprintf "Committed quantity: %g" value
  in
  let row_style = Style.create_exn [ Gap (Length.px_exn 8.); Wrap Wrap ] in
  let action label action success =
    V.button
      ~config:(Button.Config.create ~focus:Preserve ())
      ~disabled:(disabled || read_only)
      ~on_click:(command action success)
      label
  in
  let input =
    Number.view
      ~on_event:(function
        | Step_requested request ->
          (match step_scope with
           | Loading | Failed _ ->
             command (Resolve_step (request, Decline)) "Step processing is unavailable"
           | Ready scope ->
             let open B.Effect.Let_syntax in
             let%bind epoch =
               B.Effect.of_thunk (fun () ->
                 Int.incr request_epoch;
                 !request_epoch)
             in
             let publish message =
               let%bind current =
                 B.Effect.of_thunk (fun () -> Int.equal epoch !request_epoch)
               in
               if current then set_notice message else B.Effect.Ignore
             in
             let%bind started =
               Number.run_step
                 number
                 ~scope
                 request
                 ~f:(fun () -> adaptive_resolution request)
                 ~on_result:(function
                   | Ok _ -> publish "Applied a step selected by the application"
                   | Error (Work_failed _) ->
                     publish "The application could not compute this step"
                   | Error (Resolution_failed error) -> publish (error_message error))
             in
             (match started with
              | Ok _ -> B.Effect.Ignore
              | Error _ -> publish "Step processing is unavailable"))
        | Observed _ | Changed _ | Committed _ | Rejected _ | Cancelled _ ->
          B.Effect.Ignore)
      ~initial_draft:unfinished
      ~style:
        (Style.create_exn
           [ Height (Length.px_exn (Palette.size p 44.))
           ; Width (Length.px_exn (Palette.size p 360.))
           ])
      number
  in
  let input =
    if not framed
    then input
    else (
      let frame_style =
        Style.create_exn
          [ Background (Background.solid (Palette.surface p))
          ; Foreground (Palette.foreground p)
          ; Border_color (Palette.border p)
          ; Radius (Palette.size p 8.)
          ]
        |> fun style ->
        Style.with_state_exn style Focused [ Border_color (Palette.accent p) ]
      in
      let button_style =
        Style.create_exn [ Foreground (Palette.foreground p) ]
        |> fun style ->
        Style.with_state_exn
          style
          Hovered
          [ Background (Background.solid (Palette.border p)) ]
        |> fun style ->
        Style.with_state_exn
          style
          Pressed
          [ Background (Background.solid (Palette.accent p)) ]
        |> fun style ->
        Style.with_state_exn style Disabled [ Foreground (Palette.muted p) ]
      in
      let appearance =
        Number_input.Appearance.create
          ~gap:(Palette.size p 4.)
          ~button_width:(Palette.size p 32.)
          ~button_min_height:(Palette.size p 24.)
          ~stacked_button_min_height:(Palette.size p 18.)
          ~editor_padding:(Palette.size p 4.)
          ~border_width:1.
          ~frame_style
          ~decrement_style:button_style
          ~increment_style:button_style
          ()
        |> Or_error.ok_exn
      in
      V.number_frame
        ~appearance
        ~leading:
          (V.button
             ~style:
               (Style.create_exn
                  [ Foreground (Palette.muted p); Font_size (Palette.size p 11.) ])
             ~on_click:
               (set_notice
                  "The prefix action keeps the numeric draft and committed value")
             "Qty")
        ~trailing:(Palette.text p ~muted:true ~size:11. "units")
        ?decrement:(if custom_symbols then Some (Palette.text p "↓") else None)
        ?increment:(if custom_symbols then Some (Palette.text p "↑") else None)
        input
      |> Or_error.ok_exn)
  in
  Palette.card
    p
    ~title:"Numbers with clear intent"
    [ Palette.text p ~muted:true "0–100 · quarter steps · Enter commits, Escape restores"
    ; input
    ; Palette.text p committed
    ; Palette.text
        p
        (Option.value_map snapshot ~default:"Preparing draft" ~f:classification)
    ; V.row
        ~style:row_style
        (List.map
           [ "Side controls", Number_input.Step_controls.Sides
           ; "Stacked controls", Stacked
           ; "Keyboard only", Hidden
           ]
           ~f:(fun (label, choice) ->
             V.button
               ~config:(Button.Config.create ~focus:Preserve ())
               ~on_click:(set_controls choice)
               (if Number_input.Step_controls.equal controls choice
                then "✓ " ^ label
                else label)))
    ; V.row
        ~style:row_style
        [ action "Commit quantity" Commit "Value committed"
        ; action "Restore quantity" Cancel "Restored the last committed value"
        ; action "Undo quantity" Undo "Undo leaves the committed value unchanged"
        ; action "Redo quantity" Redo "Redo leaves the committed value unchanged"
        ]
    ; V.switch ~checked:allow_empty ~on_toggle:toggle_empty "Allow an empty quantity"
    ; V.switch ~checked:disabled ~on_toggle:toggle_disabled "Disable quantity"
    ; V.switch
        ~checked:framed
        ~on_toggle:toggle_framed
        "Style the complete numeric control"
    ; V.switch ~checked:custom_step ~on_toggle:toggle_step "Choose step size in OCaml"
    ; Palette.text p ~muted:true "Custom steps: ¼ below 10, 1 below 50, then 5"
    ; V.switch ~checked:custom_symbols ~on_toggle:toggle_symbols "Custom step symbols"
    ; Palette.text p ~muted:true notice
    ]
;;
