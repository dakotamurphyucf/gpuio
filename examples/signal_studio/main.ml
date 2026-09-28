open Core
module W = Signal_studio_model.Workspace
module App = Gpuio_eio.App
module Scope = Gpuio_eio.Scope
module Scene = Gpuio_eio.Canvas
module Chart = Gpuio_eio.Chart
module Canvas = Gpuio.Canvas
module B = Bonsai.Cont
module E = Bonsai.Effect
module Counter = Gpuio_example_counter

let ok = Or_error.ok_exn

let scene_ok = function
  | Ok x -> x
  | Error e -> raise_s [%sexp (e : Scene.Error.t)]
;;

let chart_ok = function
  | Ok x -> x
  | Error e -> raise_s [%sexp (e : Chart.Error.t)]
;;

let () =
  let self_test = Array.exists (Sys.get_argv ()) ~f:(String.equal "--self-test") in
  let completed = ref false in
  let catalog = App.extension_catalog () |> ok in
  assert (List.exists catalog ~f:(Gpuio.Extension.Schema.equal Counter.schema));
  App.run (fun env app ->
    let initial = W.create () in
    let state =
      B.Expert.Var.create
        { Ui.Snapshot.workspace = initial
        ; canvas = None
        ; chart = None
        ; command = None
        ; inspector = true
        ; compact = false
        ; running = false
        ; status = "Preparing workspace"
        ; extension_generation = 1L
        ; extension_disabled = false
        ; extension_visible = true
        }
    in
    let update f = B.Expert.Var.set state (f (B.Expert.Var.get state)) in
    let canvas = ref None
    and chart = ref None in
    let sequence = ref 0L
    and ack = ref 0L in
    let chart_ready = ref 0L
    and extension_mounts = ref 0
    and motion_events = ref 0 in
    let layout = ref "" in
    let emit message = Eio.traceln "SIGNAL_STUDIO: %s" message in
    let publish workspace =
      Option.iter !canvas ~f:(fun source ->
        Scene.set source (W.scene workspace) |> scene_ok);
      Option.iter !chart ~f:(fun source ->
        Chart.set source (W.chart workspace) |> chart_ok);
      update (fun s -> { s with workspace; status = "Workspace updated" })
    in
    let command action =
      sequence := Int64.succ !sequence;
      let command = Canvas.Command.create ~sequence:!sequence action |> ok in
      update (fun s -> { s with command = Some command });
      !sequence
    in
    let set_run run =
      match W.set_run (B.Expert.Var.get state).workspace run with
      | Ok workspace ->
        publish workspace;
        emit (sprintf "run %d" run)
      | Error _ ->
        (* A native click already queued before the limit disabled the control
           may still arrive. Keep the validated workspace authoritative. *)
        update (fun s -> { s with status = "Run control is limited to 0–100." });
        emit "run rejected"
    in
    let on_canvas event =
      E.of_thunk (fun () ->
        match event.Canvas.Event.observation with
        | Selection_changed selected ->
          update (fun s -> { s with workspace = W.select s.workspace selected |> ok });
          emit "canvas selected"
        | Activated id ->
          update (fun s ->
            { s with workspace = W.select s.workspace (Some id) |> ok; inspector = true });
          emit "canvas activated"
        | Moved (id, transform) ->
          let point =
            Gpuio.Canvas_geometry.Transform.apply
              transform
              (Gpuio.Canvas_geometry.Point.create ~x:0. ~y:0. |> ok)
            |> ok
          in
          publish (W.move (B.Expert.Var.get state).workspace id point |> ok);
          emit "canvas moved"
        | Viewport_changed viewport ->
          update (fun s ->
            { s with
              status = sprintf "View %.0f%%" (Canvas.Viewport.zoom viewport *. 100.)
            });
          emit "viewport changed"
        | Command_completed sequence -> ack := Int64.max !ack sequence
        | Failed error -> raise_s [%sexp (error : Canvas.Error.t)])
    in
    let on_chart (event : Gpuio.Chart.Event.t) =
      E.of_thunk (fun () ->
        match event.observation with
        | Ready _ ->
          chart_ready := event.data_revision;
          emit (sprintf "chart ready %Ld" event.data_revision)
        | Selection_changed selection ->
          update (fun s ->
            { s with
              status =
                (if Option.is_some selection
                 then "Chart value selected"
                 else "Chart selection cleared")
            });
          emit "chart selected"
        | Failed error -> raise_s [%sexp (error : Gpuio.Chart.Error.t)])
    in
    let on_extension = function
      | Gpuio.Extension.Event.Data value -> E.of_thunk (fun () -> set_run value)
      | Mounted ->
        E.of_thunk (fun () ->
          incr extension_mounts;
          emit "extension mounted")
      | Command_completed _ -> E.Ignore
      | Failed error ->
        E.of_thunk (fun () -> raise_s [%sexp (error : Gpuio.Extension.Error.t)])
    in
    let task = ref None in
    let stream_epoch = ref 0 in
    let stop () =
      incr stream_epoch;
      Option.iter !task ~f:Scope.Task.cancel;
      task := None;
      update (fun s -> { s with running = false })
    in
    let start () =
      stop ();
      update (fun s -> { s with running = true });
      let scope = App.scope app in
      let epoch = !stream_epoch in
      let job =
        Scope.start
          scope
          ~f:(fun () ->
            for _ = 1 to 12 do
              Eio.Time.sleep (Eio.Stdenv.clock env) 0.3;
              let promise, resolver = Eio.Promise.create () in
              Scope.Expert.enqueue scope (fun () ->
                let run = W.run (B.Expert.Var.get state).workspace in
                if epoch = !stream_epoch then set_run (if run = 100 then 0 else run + 1);
                Eio.Promise.resolve resolver ());
              Eio.Promise.await promise
            done)
          ~on_result:(fun result ->
            E.of_thunk (fun () ->
              ok result;
              task := None;
              update (fun s ->
                { s with running = false; status = "Run sequence complete" });
              emit "stream complete"))
        |> ok
      in
      task := Some job
    in
    let actions : Ui.Actions.t =
      { select =
          (fun id -> E.of_thunk (fun () -> ignore (command (Select (Some id)) : int64)))
      ; canvas = on_canvas
      ; chart = on_chart
      ; extension = on_extension
      ; inspector =
          E.of_thunk (fun () -> update (fun s -> { s with inspector = not s.inspector }))
      ; run =
          E.of_thunk (fun () ->
            if (B.Expert.Var.get state).running then stop () else start ())
      ; reset =
          E.of_thunk (fun () ->
            stop ();
            publish initial;
            update (fun s ->
              { s with extension_generation = Int64.succ s.extension_generation });
            ignore (command (Select None) : int64))
      ; lock_control =
          E.of_thunk (fun () ->
            update (fun s -> { s with extension_disabled = not s.extension_disabled }))
      ; hide_control =
          E.of_thunk (fun () ->
            update (fun s -> { s with extension_visible = not s.extension_visible }))
      ; reset_viewport = E.of_thunk (fun () -> ignore (command Reset_viewport : int64))
      ; on_layout =
          (fun selected ->
            E.of_thunk (fun () ->
              layout := Gpuio.Container_query.Branch_id.to_string selected.branch;
              update (fun s -> { s with compact = String.equal !layout "compact" });
              emit ("layout " ^ !layout)))
      ; on_motion = (fun _ -> E.of_thunk (fun () -> incr motion_events))
      }
    in
    let component _window _graph =
      let open B.Let_syntax in
      let%arr state = B.Expert.Var.value state in
      Ui.view state actions
    in
    let window =
      App.open_window
        app
        ~title:"GPUIO · Signal Studio"
        ~width:1160.
        ~height:860.
        ~focus:true
        component
      |> ok
    in
    let scope = App.scope app in
    Scope.start
      scope
      ~f:(fun () ->
        let clock = Eio.Stdenv.clock env in
        Eio.Time.with_timeout_exn clock 60. (fun () ->
          let on_ui ui_effect =
            let promise, resolver = Eio.Promise.create () in
            Scope.Expert.enqueue scope (fun () ->
              E.Expert.handle (E.map ui_effect ~f:(Eio.Promise.resolve resolver)));
            Eio.Promise.await promise
          in
          let ui f = on_ui (E.of_thunk f) in
          let rec until f =
            if not (ui f)
            then (
              Eio.Time.sleep clock 0.005;
              until f)
          in
          let scene = on_ui (Scene.create app ~scope (W.scene initial)) |> scene_ok in
          let signal = on_ui (Chart.create app ~scope (W.chart initial)) |> chart_ok in
          ui (fun () ->
            canvas := Some scene;
            chart := Some signal;
            update (fun s ->
              { s with
                canvas = Some (Scene.handle scene)
              ; chart = Some (Chart.handle signal)
              ; status = "Ready to explore"
              }));
          until (fun () -> Int64.(!chart_ready > 0L) && !extension_mounts > 0);
          emit "ready";
          if self_test
          then (
            let selected =
              ui (fun () ->
                command (Select (Some (List.hd_exn (W.samples initial) |> W.Sample.id))))
            in
            until (fun () -> Int64.(!ack >= selected));
            ui (fun () ->
              assert (Option.is_some (W.selected (B.Expert.Var.get state).workspace));
              set_run 7);
            until (fun () -> Int64.(!chart_ready >= 2L));
            on_ui (on_extension (Gpuio.Extension.Event.Data 101));
            ui (fun () -> assert (W.run (B.Expert.Var.get state).workspace = 7));
            let frame, resolver = Eio.Promise.create () in
            ui (fun () ->
              App.Window.request_frame window ~on_rendered:(fun ~revision:_ ->
                E.of_thunk (fun () -> Eio.Promise.resolve resolver ()))
              |> ok);
            Eio.Promise.await frame;
            ui (fun () ->
              Scene.release scene;
              Chart.release signal;
              update (fun s -> { s with canvas = None; chart = None }));
            until (fun () ->
              let d = App.diagnostics app in
              d.charts = 0 && d.canvases = 0);
            ui (fun () ->
              assert ((App.diagnostics app).chart_data_bytes = 0);
              assert ((App.diagnostics app).canvas_scene_bytes = 0));
            emit
              (sprintf
                 "self-test passed layout=%s mounts=%d motion_events=%d"
                 !layout
                 !extension_mounts
                 !motion_events);
            completed := true)))
      ~on_result:(fun result ->
        E.of_thunk (fun () ->
          ok result;
          if self_test then App.shutdown app))
    |> ok
    |> fun (_ : Scope.Task.t) -> ());
  if self_test then assert !completed
;;
