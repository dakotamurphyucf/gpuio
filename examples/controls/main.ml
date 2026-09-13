open Core
module B = Bonsai.Cont
module View = Gpuio_bonsai.View
module App = Gpuio_eio.App

let component window graph =
  let enabled, toggle_enabled = B.toggle ~default_model:true graph in
  let streaming, toggle_streaming = B.toggle ~default_model:true graph in
  let notify, toggle_notify = B.toggle ~default_model:false graph in
  let open B.Let_syntax in
  let%arr enabled = enabled
  and toggle_enabled = toggle_enabled
  and streaming = streaming
  and toggle_streaming = toggle_streaming
  and notify = notify
  and toggle_notify = toggle_notify in
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
    ; View.button
        ~on_click:(Bonsai.Effect.of_thunk (fun () -> App.Window.close window))
        "Close"
    ]
;;

let () =
  App.run (fun _ app ->
    App.open_window app ~title:"GPUIO native controls" ~width:460. ~height:350. component
    |> Or_error.ok_exn
    |> fun (_ : App.Window.t) -> ())
;;
