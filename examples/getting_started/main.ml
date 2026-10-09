open Core
module App = Gpuio_eio.App
module Effect = Bonsai.Effect
module Bonsai = Bonsai.Cont
module View = Gpuio_bonsai.View

(* GPUIO: describe the layout from current values and actions. *)
let counter_view ~count ~increment ~reset ~close =
  View.column
    ~style:
      (Gpuio.Style.create_exn
         [ Padding (Gpuio.Length.px_exn 24.); Gap (Gpuio.Length.px_exn 12.) ])
    [ View.text "Hello from OCaml"
    ; View.text (sprintf "Count: %d" count)
    ; View.button "Increment" ~on_click:increment
    ; View.button "Reset" ~on_click:reset
    ; View.button "Close" ~on_click:close
    ]
;;

(* Bonsai: allocate state once, then derive the view whenever it changes. *)
let component window graph =
  let count, set_count = Bonsai.state 0 graph in
  let open Bonsai.Let_syntax in
  let%arr count = count
  and set_count = set_count in
  counter_view
    ~count
    ~increment:(set_count (count + 1))
    ~reset:(set_count 0)
    ~close:(Effect.of_thunk (fun () -> App.Window.request_close window))
;;

(* Application: start the native/Eio runtime and mount the Bonsai component. *)
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
