open Core
module B = Bonsai.Cont
module View = Gpuio_bonsai.View
module App = Gpuio_eio.App

let config ~label ~width ~outside =
  Gpuio.Overlay.Config.create ~label ~width ~dismiss_on_outside_pointer:outside ()
  |> Or_error.ok_exn
;;

let component ~self_test ~completed window graph =
  let dialog_open, set_dialog = B.state self_test graph in
  let popover_open, set_popover = B.state false graph in
  let input =
    Gpuio_eio.Text_input.create
      window
      ~config:
        (B.return
           (Gpuio.Text_input.Config.create ~mode:Single_line ~label:"Agent name" ()
            |> Or_error.ok_exn))
      graph
  in
  let outside =
    Gpuio_eio.Text_input.create
      window
      ~config:
        (B.return
           (Gpuio.Text_input.Config.create ~mode:Single_line ~label:"Workspace" ()
            |> Or_error.ok_exn))
      graph
  in
  let sleep = B.Clock.sleep graph in
  let started = ref false in
  let open B.Let_syntax in
  B.Edge.on_change
    (B.map input ~f:Gpuio_eio.Text_input.snapshot)
    ~equal:(Option.equal Gpuio.Text_input.Snapshot.equal)
    ~callback:
      (let%arr input = input
       and outside = outside
       and sleep = sleep
       and set_dialog = set_dialog
       and set_popover = set_popover in
       fun observation ->
         let module E = Bonsai.Effect in
         let open E.Let_syntax in
         let%bind begin_test =
           E.of_thunk (fun () ->
             if self_test && Option.is_some observation && not !started
             then (
               started := true;
               true)
             else false)
         in
         if not begin_test
         then E.Ignore
         else (
           let expect result =
             E.of_thunk (fun () ->
               Result.ok_or_failwith
                 (Result.map_error result ~f:(fun error ->
                    Sexp.to_string (Gpuio.Text_input.Command_error.sexp_of_t error))))
           in
           let%bind changed =
             Gpuio_eio.Text_input.replace input ~selection:End ~undo:Reset "Agent é界"
             >>= expect
           in
           let%bind () =
             E.of_thunk (fun () ->
               assert (String.equal (Gpuio.Text_input.Snapshot.text changed) "Agent é界"))
           in
           let%bind blocked = Gpuio_eio.Text_input.focus outside in
           let%bind () =
             E.of_thunk (fun () ->
               assert (
                 Result.equal
                   Gpuio.Text_input.Snapshot.equal
                   Gpuio.Text_input.Command_error.equal
                   blocked
                   (Error Focus_blocked)))
           in
           let%bind () = set_dialog false in
           let%bind () = sleep (Time_ns.Span.of_sec 0.1) in
           let%bind stale = Gpuio_eio.Text_input.focus input in
           let%bind () =
             E.of_thunk (fun () ->
               assert (
                 Result.equal
                   Gpuio.Text_input.Snapshot.equal
                   Gpuio.Text_input.Command_error.equal
                   stale
                   (Error Stale_editor)))
           in
           let%bind _ = Gpuio_eio.Text_input.focus outside >>= expect in
           let%bind () = set_popover true in
           let%bind () = sleep (Time_ns.Span.of_sec 0.1) in
           let%bind _ = Gpuio_eio.Text_input.focus outside >>= expect in
           let%bind () = set_popover false in
           let%bind () = sleep (Time_ns.Span.of_sec 0.1) in
           E.of_thunk (fun () ->
             completed := true;
             App.Window.close window)))
    graph;
  let%arr dialog_open = dialog_open
  and set_dialog = set_dialog
  and popover_open = popover_open
  and set_popover = set_popover
  and input = input
  and outside = outside in
  let close_dialog = set_dialog false in
  View.column
    ~style:
      (Gpuio.Style.create_exn
         [ Padding (Gpuio.Length.px_exn 24.); Gap (Gpuio.Length.px_exn 12.) ])
    [ View.text "Overlays retain native focus and editor state while open."
    ; Gpuio_eio.Text_input.view outside
    ; View.button ~on_click:(set_dialog true) "Edit agent"
    ; View.popover
        ~key:(Gpuio.Key.of_string_exn "help")
        ~config:(config ~label:"Help" ~width:280. ~outside:true)
        ~anchor:(View.button ~on_click:(set_popover (not popover_open)) "Help")
        ~on_dismiss:(fun _ -> set_popover false)
        (if popover_open
         then
           Some
             (View.column
                [ View.text "Escape or an outside click requests closure."
                ; View.button ~on_click:(set_popover false) "Close help"
                ])
         else None)
    ; View.dialog
        ~key:(Gpuio.Key.of_string_exn "settings")
        ~config:(config ~label:"Agent settings" ~width:360. ~outside:false)
        ~on_dismiss:(fun _ -> close_dialog)
        (if dialog_open
         then
           Some
             (View.column
                ~style:(Gpuio.Style.create_exn [ Gap (Gpuio.Length.px_exn 12.) ])
                [ View.text "Agent settings"
                ; Gpuio_eio.Text_input.view input
                ; View.button ~on_click:close_dialog "Done"
                ])
         else None)
    ; View.button
        ~on_click:(Bonsai.Effect.of_thunk (fun () -> App.Window.close window))
        "Quit"
    ]
;;

let () =
  let self_test = Array.exists (Sys.get_argv ()) ~f:(String.equal "--self-test") in
  let completed = ref false in
  App.run (fun _ app ->
    App.open_window
      app
      ~title:"GPUIO overlays"
      ~width:520.
      ~height:400.
      (component ~self_test ~completed)
    |> Or_error.ok_exn
    |> fun (_ : App.Window.t) -> ());
  if self_test
  then (
    assert !completed;
    print_endline
      "GPUIO_OVERLAYS_PUBLIC_OK: native dialog/popover mount, editor commands, modal \
       focus denial, unmount and close")
;;
