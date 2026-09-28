open Core
module App = Gpuio_eio.App
module Scope = Gpuio_eio.Scope
module Registered = Gpuio_eio.Chart
module Data = Gpuio.Chart_data
module Chart = Gpuio.Chart
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View

let ok = Or_error.ok_exn

let checked = function
  | Ok value -> value
  | Error error -> raise_s [%sexp (error : Registered.Error.t)]
;;

let dataset count phase =
  Data.line
    [ Data.Series.create
        ~id:(Data.Series_id.of_int64 1L |> ok)
        ~name:"Streaming signal"
        (List.init count ~f:(fun i ->
           Data.Point.create
             ~id:(Data.Datum_id.of_int64 (Int64.of_int (i + 1)) |> ok)
             ~x:(Float.of_int i)
             ~y:(Some (Float.sin ((Float.of_int i /. 300.) +. phase)))
             ()
           |> ok))
      |> ok
    ]
  |> ok
;;

let () =
  let completed = ref false in
  App.run (fun env app ->
    let handle = B.Expert.Var.create None in
    let exact = B.Expert.Var.create false in
    let observation = ref None in
    let on_event event = E.of_thunk (fun () -> observation := Some event) in
    let component _window _graph =
      let open B.Let_syntax in
      let%arr handle = B.Expert.Var.value handle
      and exact = B.Expert.Var.value exact in
      match handle with
      | None -> V.text "Preparing streaming workload"
      | Some data ->
        let sampling =
          if exact
          then Gpuio.Chart_sampling.create ~line:Gpuio.Chart_sampling.Line.exact ()
          else Gpuio.Chart_sampling.default
        in
        V.chart
          ~on_event
          (Chart.Config.create ~data ~label:"Streaming workload" ~sampling () |> ok)
          ~style:
            (Gpuio.Style.create_exn
               [ Width (Gpuio.Length.px_exn 800.)
               ; Height (Gpuio.Length.px_exn 400.)
               ; Background (Gpuio.Background.solid (Gpuio.Color.rgb_exn 0x172230))
               ])
    in
    let window =
      App.open_window
        app
        ~title:"GPUIO · Chart streaming workload"
        ~width:820.
        ~height:440.
        ~focus:true
        component
      |> ok
    in
    let scope = App.Window.scope window in
    Scope.start
      scope
      ~f:(fun () ->
        let clock = Eio.Stdenv.clock env in
        let mono () = Eio.Time.Mono.now (Eio.Stdenv.mono_clock env) in
        let elapsed start = Mtime.Span.to_float_ns (Mtime.span start (mono ())) /. 1e6 in
        Eio.Time.with_timeout_exn clock 180. (fun () ->
          let on_ui ui_effect =
            let promise, resolver = Eio.Promise.create () in
            Scope.Expert.enqueue scope (fun () ->
              E.Expert.handle (E.map ui_effect ~f:(Eio.Promise.resolve resolver)));
            Eio.Promise.await promise
          in
          let ui f = on_ui (E.of_thunk f) in
          let peak_source = ref 0
          and peak_pending = ref 0
          and peak_commands = ref 0 in
          let sample () =
            let d = App.diagnostics app in
            peak_source := Int.max !peak_source d.chart_data_bytes;
            peak_pending := Int.max !peak_pending d.pending_requests;
            peak_commands := Int.max !peak_commands d.queued_commands;
            d
          in
          let rec until f =
            if
              not
                (ui (fun () ->
                   ignore (sample () : App.Diagnostics.t);
                   f ()))
            then (
              Eio.Time.sleep clock 0.002;
              until f)
          in
          let registration =
            on_ui (Registered.create app ~scope (dataset 10_000 0.)) |> checked
          in
          ui (fun () -> B.Expert.Var.set handle (Some (Registered.handle registration)));
          let wait_ready revision =
            until (fun () ->
              Option.iter (Registered.error registration) ~f:(fun error ->
                raise_s [%sexp (error : Registered.Error.t)]);
              match !observation with
              | Some { Chart.Event.observation = Failed error; _ } ->
                raise_s [%sexp (error : Chart.Error.t)]
              | Some { data_revision; observation = Ready _; _ } ->
                Int64.equal data_revision revision && Registered.is_published registration
              | Some { observation = Selection_changed _; _ } | None -> false)
          in
          wait_ready 1L;
          let frame () =
            let promise, resolver = Eio.Promise.create () in
            ui (fun () ->
              App.Window.request_frame window ~on_rendered:(fun ~revision:_ ->
                E.of_thunk (fun () -> Eio.Promise.resolve resolver ()))
              |> ok);
            Eio.Promise.await promise
          in
          frame ();
          let revision = ref 1L in
          List.iter
            [ 10_000, false, 30, 1
            ; 100_000, false, 30, 1
            ; 100_000, true, 10, 1
            ; 100_000, false, 10, 8
            ]
            ~f:(fun (count, use_exact, iterations, burst) ->
              ui (fun () -> B.Expert.Var.set exact use_exact);
              for iteration = 1 to iterations do
                let build_start = mono () in
                let updates =
                  List.init burst ~f:(fun j ->
                    dataset count (Float.of_int ((iteration * burst) + j) /. 7.))
                in
                let build_ms = elapsed build_start in
                let before = ui (fun () -> sample ()) in
                let start = mono () in
                ui (fun () ->
                  List.iter updates ~f:(fun data ->
                    Registered.set registration data |> checked);
                  ignore (sample () : App.Diagnostics.t));
                revision := Int64.succ !revision;
                wait_ready !revision;
                frame ();
                let latency_ms = elapsed start in
                let metrics, after =
                  ui (fun () ->
                    assert (
                      Data.equal
                        (Registered.data registration |> Option.value_exn)
                        (List.last_exn updates));
                    match !observation with
                    | Some { observation = Ready metrics; _ } -> metrics, sample ()
                    | _ -> failwith "Missing prepared chart metrics")
                in
                assert (metrics.source_values = count);
                if use_exact then assert (metrics.retained_values = count);
                Eio.traceln
                  "CHART_STREAM_SAMPLE points=%d exact=%d burst=%d iteration=%d \
                   build_ms=%.3f update_frame_ms=%.3f representatives=%d vertices=%d \
                   quads=%d plan_bytes=%d source_charge=%d submitted_bytes=%d \
                   submitted_messages=%d"
                  count
                  (Bool.to_int use_exact)
                  burst
                  iteration
                  build_ms
                  latency_ms
                  metrics.retained_values
                  metrics.mesh_vertices
                  metrics.quads
                  metrics.bytes
                  after.chart_data_bytes
                  (after.traffic.submitted_bytes - before.traffic.submitted_bytes)
                  (after.traffic.submitted_messages - before.traffic.submitted_messages)
              done);
          ui (fun () ->
            Registered.release registration;
            B.Expert.Var.set handle None);
          until (fun () -> (App.diagnostics app).charts = 0);
          let final = ui (fun () -> sample ()) in
          assert (final.chart_data_bytes = 0);
          assert (!peak_source <= 128 * 1024 * 1024);
          Eio.traceln
            "CHART_STREAM_OK samples=80 updates=150 peak_source_charge=%d \
             peak_pending=%d peak_commands=%d final_source_charge=%d \
             native_queue_peak_bytes=%d"
            !peak_source
            !peak_pending
            !peak_commands
            final.chart_data_bytes
            final.native_command_queue.peak_bytes;
          completed := true))
      ~on_result:(fun result ->
        E.of_thunk (fun () ->
          ok result;
          App.shutdown app))
    |> ok
    |> fun (_ : Scope.Task.t) -> ());
  assert !completed
;;
