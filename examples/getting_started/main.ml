open Core
module App = Gpuio_eio.App
module B = Bonsai.Cont
module V = Gpuio_bonsai.View

let component window graph =
  let count, set_count = B.state 0 graph in
  let open B.Let_syntax in
  let%arr count = count
  and set_count = set_count in
  V.column
    ~style:
      (Gpuio.Style.create_exn
         [ Padding (Gpuio.Length.px_exn 24.); Gap (Gpuio.Length.px_exn 12.) ])
    [ V.text "Hello from OCaml"
    ; V.text (sprintf "Count: %d" count)
    ; V.button "Increment" ~on_click:(set_count (count + 1))
    ; V.button "Reset" ~on_click:(set_count 0)
    ; V.button
        "Close"
        ~on_click:(Bonsai.Effect.of_thunk (fun () -> App.Window.request_close window))
    ]
;;

let () =
  App.run (fun _env app ->
    let (_ : App.Window.t) =
      App.open_window
        app
        ~title:"GPUIO · Getting started"
        ~width:440.
        ~height:320.
        component
      |> Or_error.ok_exn
    in
    ())
;;
