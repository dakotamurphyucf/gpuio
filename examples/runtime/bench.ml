open Core
module App = Gpuio_eio.App
module Scope = Gpuio_eio.Scope
module B = Bonsai.Cont
module View = Gpuio_bonsai.View
module E = Bonsai.Effect

let component ~clock_dependent _window graph =
  let fixed =
    View.column
      (List.init 50 ~f:(fun i ->
         View.text ~key:(Gpuio.Key.of_int i) (sprintf "Static row %d" i)))
  in
  if clock_dependent
  then (
    let time = B.Clock.approx_now ~tick_every:(Time_ns.Span.of_sec 1.) graph in
    let open B.Let_syntax in
    let%arr time = time in
    View.column
      [ View.text (Time_ns.to_int63_ns_since_epoch time |> Int63.to_string); fixed ])
  else B.return fixed
;;

let () =
  let args = Sys.get_argv () in
  let windows = if Array.length args > 1 then Int.of_string args.(1) else 1 in
  let tick_hz =
    Array.find_map args ~f:(fun arg ->
      Option.map (String.chop_prefix arg ~prefix:"--hz=") ~f:Float.of_string)
    |> Option.value ~default:60.
  in
  let clock_dependent = Array.exists args ~f:(String.equal "--clock") in
  if windows < 1 || windows > 8 then invalid_arg "benchmark windows must be 1..8";
  App.run ~tick_hz (fun env app ->
    let opened =
      List.init windows ~f:(fun i ->
        App.open_window
          app
          ~title:(sprintf "Runtime measurement %d" i)
          ~width:400.
          ~height:400.
          (component ~clock_dependent)
        |> Or_error.ok_exn)
    in
    let clock = Eio.Stdenv.clock env in
    ignore
      (Scope.start
         (App.scope app)
         ~f:(fun () ->
           Eio.Time.with_timeout_exn clock 15. (fun () ->
             while (App.stats app).commits < windows do
               Eio.Time.sleep clock 0.01
             done);
           List.iter opened ~f:(fun window ->
             let ready, resolver = Eio.Promise.create () in
             App.Window.request_frame window ~on_rendered:(fun ~revision:_ ->
               E.of_thunk (fun () -> Eio.Promise.resolve resolver ()))
             |> Or_error.ok_exn;
             Eio.Time.with_timeout_exn clock 5. (fun () -> Eio.Promise.await ready));
           Eio.Time.sleep clock 0.25;
           let before = App.stats app in
           let cpu = Stdlib.Sys.time ()
           and allocated = Gc.allocated_bytes ()
           and started = Eio.Time.Mono.now (Eio.Stdenv.mono_clock env) in
           Eio.Time.Mono.sleep (Eio.Stdenv.mono_clock env) 3.;
           let elapsed =
             Mtime.Span.to_float_ns
               (Mtime.span started (Eio.Time.Mono.now (Eio.Stdenv.mono_clock env)))
             /. 1e9
           in
           let cpu = Stdlib.Sys.time () -. cpu
           and allocated = Gc.allocated_bytes () -. allocated in
           let after = App.stats app in
           if not clock_dependent then assert (after.commits = before.commits);
           Eio.Flow.copy_string
             (sprintf
                "RUNTIME_MEASURE windows=%d clock_dependent=%b target_hz=%.1f \
                 seconds=%.3f cpu_seconds=%.6f cpu_percent_one_core=%.3f \
                 allocated_bytes=%.0f turns=%d ticks=%d commits=%d \
                 render_notifications=%d\n"
                windows
                clock_dependent
                tick_hz
                elapsed
                cpu
                (100. *. cpu /. elapsed)
                allocated
                (after.turns - before.turns)
                (after.clock_ticks - before.clock_ticks)
                (after.commits - before.commits)
                (after.rendered - before.rendered))
             (Eio.Stdenv.stdout env);
           App.shutdown app)
         ~on_result:(fun result -> E.of_thunk (fun () -> Or_error.ok_exn result))
       |> Or_error.ok_exn
       : Scope.Task.t))
;;
