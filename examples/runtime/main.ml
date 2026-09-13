open Core
module App = Gpuio_eio.App
module Scope = Gpuio_eio.Scope
module Stream = Gpuio_eio.Stream
module View = Gpuio_bonsai.View
module B = Bonsai.Cont
module E = Bonsai.Effect

let unit_result = fun result -> E.of_thunk (fun () -> Or_error.ok_exn result)

let start scope f =
  Scope.start scope ~f ~on_result:unit_result
  |> Or_error.ok_exn
  |> fun (_ : Scope.Task.t) -> ()
;;

let component ~name ~shared ~events ~timer_samples ~now window graph =
  let open B.Let_syntax in
  let value, set_value = B.state 0 graph in
  let awake, set_awake = B.state false graph in
  let sleep = B.Clock.sleep graph in
  let on_activate =
    let%arr sleep = sleep
    and set_awake = set_awake in
    let open E.Let_syntax in
    let%bind () = E.of_thunk (fun () -> events := (name ^ "+") :: !events) in
    let%bind started = E.of_thunk now in
    let%bind () = sleep (Time_ns.Span.of_sec 0.05) in
    let%bind () =
      E.of_thunk (fun () ->
        timer_samples := (now () -. started -. 0.05) :: !timer_samples)
    in
    set_awake true
  in
  B.Edge.lifecycle
    ~on_activate
    ~on_deactivate:(B.return (E.of_thunk (fun () -> events := (name ^ "-") :: !events)))
    graph;
  B.Edge.on_change
    awake
    ~equal:Bool.equal
    ~callback:
      (B.return (fun ready ->
         E.of_thunk (fun () -> if ready then events := (name ^ " ready") :: !events)))
    graph;
  let shared = B.Expert.Var.value shared in
  let%arr value = value
  and set_value = set_value
  and awake = awake
  and shared = shared in
  View.column
    ~style:
      (Gpuio.Style.create_exn
         [ Padding (Gpuio.Length.px_exn 20.); Gap (Gpuio.Length.px_exn 12.) ])
    [ View.text
        (name ^ ": " ^ if awake then "Bonsai timer fired" else "waiting for Bonsai timer")
    ; View.text (sprintf "Shared stream: %d" shared)
    ; View.button ~on_click:(set_value (value + 1)) (sprintf "Count %d" value)
    ; View.button
        ~on_click:(E.of_thunk (fun () -> App.Window.close window))
        "Close window"
    ]
;;

let run ~self_test =
  let timer_samples = ref [] in
  let events = ref []
  and cancellation = ref false
  and application_survived = ref false in
  App.run
    ~tick_hz:(if self_test then 5. else 60.)
    ~exit_on_last_window:(not self_test)
    (fun env app ->
       let clock = Eio.Stdenv.clock env in
       let shared = B.Expert.Var.create 0 in
       let make name =
         App.open_window
           app
           ~title:name
           ~width:480.
           ~height:300.
           (component ~name ~shared ~events ~timer_samples ~now:(fun () ->
              Eio.Time.now clock))
         |> Or_error.ok_exn
       in
       let never_activated = ref true in
       let discarded =
         App.open_window
           app
           ~title:"Close before opened"
           ~width:300.
           ~height:200.
           (fun _ graph ->
              B.Edge.lifecycle
                ~on_activate:(B.return (E.of_thunk (fun () -> never_activated := false)))
                graph;
              B.return (View.text "Never activated"))
         |> Or_error.ok_exn
       in
       App.Window.close discarded;
       let first = make "First"
       and second = make "Second" in
       let conversation =
         Scope.child (App.scope app) ~name:"conversation" |> Or_error.ok_exn
       in
       let stream =
         Stream.create ~scope:conversation ~capacity:32 ~on_batch:(fun values ->
           E.of_thunk (fun () -> B.Expert.Var.set shared (List.last_exn values)))
         |> Or_error.ok_exn
       in
       start conversation (fun () ->
         for i = 1 to 10 do
           Eio.Time.sleep clock 0.03;
           Stream.push stream i |> Or_error.ok_exn
         done);
       start (App.Window.scope first) (fun () ->
         Exn.protect ~f:Eio.Fiber.await_cancel ~finally:(fun () -> cancellation := true));
       if self_test
       then
         start (App.scope app) (fun () ->
           Eio.Time.with_timeout_exn clock 10. (fun () ->
             while
               List.count !events ~f:(String.is_suffix ~suffix:" ready") < 2
               || B.Expert.Var.get shared < 10
             do
               Eio.Time.sleep clock 0.01
             done);
           Eio.Time.sleep clock 0.1;
           assert !never_activated;
           assert (B.Expert.Var.get shared = 10);
           assert (List.count !events ~f:(String.is_suffix ~suffix:"+") = 2);
           let before = App.stats app in
           Eio.Time.sleep clock 0.45;
           let after = App.stats app in
           assert (after.clock_ticks > before.clock_ticks);
           assert (after.commits = before.commits);
           let rendered = Eio.Promise.create () in
           let promise, resolver = rendered in
           let requested = Eio.Time.now clock in
           App.Window.request_frame first ~on_rendered:(fun ~revision:_ ->
             E.of_thunk (fun () ->
               Eio.Promise.resolve resolver (Eio.Time.now clock -. requested)))
           |> Or_error.ok_exn;
           let frame_latency =
             Eio.Time.with_timeout_exn clock 3. (fun () -> Eio.Promise.await promise)
           in
           let completed, resolver = Eio.Promise.create () in
           let task_started = ref 0. in
           ignore
             (Scope.start
                (App.scope app)
                ~f:(fun () ->
                  Eio.Time.sleep clock 0.013;
                  task_started := Eio.Time.now clock)
                ~on_result:(fun result ->
                  E.of_thunk (fun () ->
                    Or_error.ok_exn result;
                    Eio.Promise.resolve resolver (Eio.Time.now clock -. !task_started)))
              |> Or_error.ok_exn
              : Scope.Task.t);
           let task_latency =
             Eio.Time.with_timeout_exn clock 3. (fun () -> Eio.Promise.await completed)
           in
           App.Window.close first;
           Eio.Time.sleep clock 0.05;
           assert !cancellation;
           assert (Scope.is_active conversation);
           assert (not (App.Window.is_closed second));
           App.Window.close second;
           Eio.Time.sleep clock 0.1;
           application_survived := true;
           let third = make "Reopened" in
           Eio.Time.sleep clock 0.3;
           App.Window.close third;
           Eio.Time.sleep clock 0.05;
           assert (List.count !events ~f:(String.is_suffix ~suffix:"-") = 3);
           let deactivated, resolver = Eio.Promise.create () in
           ignore
             (App.open_window
                app
                ~title:"Close during activation"
                ~width:300.
                ~height:200.
                (fun window graph ->
                   B.Edge.lifecycle
                     ~on_activate:
                       (B.return (E.of_thunk (fun () -> App.Window.close window)))
                     ~on_deactivate:
                       (B.return (E.of_thunk (fun () -> Eio.Promise.resolve resolver ())))
                     graph;
                   B.return (View.text "Close safely from lifecycle"))
              |> Or_error.ok_exn
              : App.Window.t);
           Eio.Time.with_timeout_exn clock 3. (fun () -> Eio.Promise.await deactivated);
           Eio.Flow.copy_string
             (sprintf
                "RUNTIME_PASS scopes=true empty_diff=true clocks=true reopen=true \
                 lifecycle_close=true close_before_open=true frame_latency_ms=%.3f \
                 task_latency_ms=%.3f timer_lateness_ms=%s stats=%s\n"
                (frame_latency *. 1000.)
                (task_latency *. 1000.)
                (Sexp.to_string_hum
                   ([%sexp_of: float list]
                      (List.map !timer_samples ~f:(fun seconds -> seconds *. 1000.))))
                (Sexp.to_string_hum (App.Stats.sexp_of_t (App.stats app))))
             (Eio.Stdenv.stdout env);
           App.shutdown app));
  if self_test then assert !application_survived
;;

let () =
  let has flag = Array.exists (Sys.get_argv ()) ~f:(String.equal flag) in
  if has "--shutdown-test" || has "--last-window-test"
  then
    App.run (fun _ app ->
      ignore
        (App.open_window
           app
           ~title:"Shutdown test"
           ~width:300.
           ~height:200.
           (fun window graph ->
              B.Edge.lifecycle
                ~on_activate:(B.return (E.of_thunk (fun () -> App.Window.close window)))
                graph;
              B.return (View.text "Shutdown"))
         |> Or_error.ok_exn
         : App.Window.t);
      if has "--shutdown-test" then App.shutdown app)
  else (
    let self_test = Array.exists (Sys.get_argv ()) ~f:(String.equal "--self-test") in
    run ~self_test)
;;
