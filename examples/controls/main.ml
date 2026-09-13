open Core
module B = Bonsai.Cont
module View = Gpuio_bonsai.View
module App = Gpuio_eio.App

let choice_id value = Gpuio.Choice.Id.of_string value |> Or_error.ok_exn

let modes =
  [ "fast", "Fast responses"; "deep", "Detailed reasoning" ]
  |> List.map ~f:(fun (id, label) ->
    Gpuio.Choice.create ~id:(choice_id id) ~label () |> Or_error.ok_exn)
  |> Gpuio.Choice.Collection.create
  |> Or_error.ok_exn
;;

let choice_appearance =
  let open Gpuio in
  Choice.Appearance.create
    ~row_height:40.
    ~empty_label:"No reasoning modes available"
    ~popup_style:
      (Style.create_exn
         [ Background (Background.solid (Color.token_exn "background"))
         ; Foreground (Color.token_exn "foreground")
         ; Border_color (Color.token_exn "muted")
         ])
    ~option_style:
      (Style.with_state_exn
         Style.empty
         Focused
         [ Background (Background.solid (Color.token_exn "accent")) ])
    ()
  |> Or_error.ok_exn
;;

let component window graph =
  let enabled, toggle_enabled = B.toggle ~default_model:true graph in
  let streaming, toggle_streaming = B.toggle ~default_model:true graph in
  let notify, toggle_notify = B.toggle ~default_model:false graph in
  let mode, set_mode = B.state (Some (choice_id "fast")) graph in
  let open B.Let_syntax in
  let combo_config =
    let%arr enabled = enabled
    and mode = mode in
    Gpuio.Combobox.Config.create
      ~label:"Search reasoning mode"
      ~options:modes
      ~selected:mode
      ~disabled:(not enabled)
      ~placeholder:"Type to filter"
      ()
    |> Or_error.ok_exn
  in
  let on_select =
    let%arr set_mode = set_mode in
    fun selected -> set_mode (Some (Gpuio.Combobox.Selection.id selected))
  in
  let combo = Gpuio_eio.Combobox.create window ~config:combo_config ~on_select graph in
  let%arr enabled = enabled
  and toggle_enabled = toggle_enabled
  and streaming = streaming
  and toggle_streaming = toggle_streaming
  and notify = notify
  and toggle_notify = toggle_notify
  and mode = mode
  and set_mode = set_mode
  and combo = combo in
  View.column
    ~style:
      (Gpuio.Style.create_exn
         [ Padding (Gpuio.Length.px_exn 24.); Gap (Gpuio.Length.px_exn 12.) ])
    [ View.text "Agent preferences"
    ; View.checkbox
        ~state:(Gpuio.Check_state.of_bool enabled)
        ~on_toggle:toggle_enabled
        "Enable preferences"
    ; View.switch
        ~disabled:(not enabled)
        ~checked:streaming
        ~on_toggle:toggle_streaming
        "Stream responses"
    ; View.checkbox
        ~disabled:(not enabled)
        ~state:(Gpuio.Check_state.of_bool notify)
        ~on_toggle:toggle_notify
        "Notify when finished"
    ; View.text
        (if streaming
         then "Responses stream as they arrive."
         else "Show completed responses.")
    ; View.radio_group
        ~config:
          (Gpuio.Choice.Config.create
             ~label:"Reasoning mode"
             ~options:modes
             ~selected:mode
             ~disabled:(not enabled)
             ()
           |> Or_error.ok_exn)
        ~on_select:(fun id -> set_mode (Some id))
        ()
    ; View.select
        ~appearance:choice_appearance
        ~config:
          (Gpuio.Choice.Config.create
             ~label:"Reasoning mode dropdown"
             ~options:modes
             ~selected:mode
             ~disabled:(not enabled)
             ()
           |> Or_error.ok_exn)
        ~on_select:(fun id -> set_mode (Some id))
        ()
    ; Gpuio_eio.Combobox.view
        ~appearance:choice_appearance
        ~style:(Gpuio.Style.create_exn [ Height (Gpuio.Length.px_exn 36.) ])
        combo
    ; View.text
        ("Search query: "
         ^ Option.value_map
             (Gpuio_eio.Combobox.snapshot combo)
             ~default:""
             ~f:Gpuio.Text_input.Snapshot.text)
    ; View.button
        ~on_click:(Bonsai.Effect.of_thunk (fun () -> App.Window.close window))
        "Close"
    ]
;;

let () =
  App.run (fun _ app ->
    App.open_window app ~title:"GPUIO native controls" ~width:460. ~height:600. component
    |> Or_error.ok_exn
    |> fun (_ : App.Window.t) -> ())
;;
