open Core
module App = Gpuio_eio.App
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module Counter = Gpuio_example_counter

let component ~smoke window graph =
  let value, set_value = B.state 7 graph in
  let sequence, set_sequence = B.state 1L graph in
  let open B.Let_syntax in
  let%arr value = value
  and set_value = set_value
  and sequence = sequence
  and set_sequence = set_sequence in
  let properties = Counter.Properties.create ~value ~step:1 () |> Or_error.ok_exn in
  let instance =
    Counter.instance properties ~generation:1L ~set_value:(sequence, 12) ()
    |> Or_error.ok_exn
  in
  let on_event = function
    | Gpuio.Extension.Event.Data value -> set_value value
    | Failed error ->
      E.of_thunk (fun () ->
        failwith (Sexp.to_string_hum (Gpuio.Extension.Error.sexp_of_t error)))
    | Mounted -> E.Ignore
    | Command_completed _ ->
      if smoke
      then
        E.of_thunk (fun () ->
          App.Window.request_frame window ~on_rendered:(fun ~revision:_ ->
            E.of_thunk (fun () -> App.Window.close window))
          |> Or_error.ok_exn)
      else E.Ignore
  in
  V.column
    ~style:
      (Gpuio.Style.create_exn
         [ Padding (Gpuio.Length.px_exn 32.); Gap (Gpuio.Length.px_exn 20.) ])
    [ V.text "A native component. An OCaml application."
    ; V.text "Properties, commands and events cross a typed, bounded bridge."
    ; V.extension ~on_event instance
    ; V.text (sprintf "OCaml observed value: %d" value)
    ; V.button "Set native value to 12" ~on_click:(set_sequence (Int64.succ sequence))
    ]
;;

let () =
  let flag name = Array.exists (Sys.get_argv ()) ~f:(String.equal name) in
  let smoke = flag "--smoke" in
  let catalog = App.extension_catalog () |> Or_error.ok_exn in
  if not (List.exists catalog ~f:(Gpuio.Extension.Schema.equal Counter.schema))
  then failwith "the linked backend does not provide the counter schema";
  App.run (fun _ app ->
    let (_ : App.Window.t) =
      App.open_window
        app
        ~focus:(not (flag "--background"))
        ~title:"GPUIO · Extension SDK"
        ~width:640.
        ~height:360.
        (component ~smoke)
      |> Or_error.ok_exn
    in
    ())
;;
