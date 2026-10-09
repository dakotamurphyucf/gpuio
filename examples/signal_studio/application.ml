open Core
module Workspace = Signal_studio_model.Workspace
module App = Gpuio_eio.App
module Scope = Gpuio_eio.Scope
module Scene = Gpuio_eio.Canvas
module Chart = Gpuio_eio.Chart
module Canvas = Gpuio.Canvas
module Effect = Bonsai.Effect
module Bonsai = Bonsai.Cont
module Counter = Gpuio_example_counter
module Desktop = Gpuio_eio.Desktop
module File = Signal_studio_files.Document_file
module N = Gpuio_eio.Notification
module Alerts = Signal_studio_notifications.Run_alerts

let ok = Or_error.ok_exn

let scene_ok = function
  | Ok x -> x
  | Error e -> raise_s [%sexp (e : Scene.Error.t)]
;;

let chart_ok = function
  | Ok x -> x
  | Error e -> raise_s [%sexp (e : Chart.Error.t)]
;;

let run () =
  let self_test = Array.exists (Sys.get_argv ()) ~f:(String.equal "--self-test") in
  let flag name = Array.exists (Sys.get_argv ()) ~f:(String.equal name) in
  let motion =
    match flag "--full-motion", flag "--reduced-motion" with
    | true, true -> failwith "Choose only one motion override"
    | true, false -> Gpuio.Animation.Preference.Full
    | false, true -> Reduce
    | false, false -> System
  in
  let trace_motion = flag "--motion-check" in
  let workload = flag "--workload-check" in
  let document_path =
    Array.find_map (Sys.get_argv ()) ~f:(String.chop_prefix ~prefix:"--document-path=")
    |> Option.map ~f:(fun path -> Gpuio.File_path.of_string path |> ok)
  in
  let unavailable_check =
    Array.exists (Sys.get_argv ()) ~f:(String.equal "--notification-unavailable-check")
  in
  let completed = ref false in
  let catalog = App.extension_catalog () |> ok in
  assert (List.exists catalog ~f:(Gpuio.Extension.Schema.equal Counter.schema));
  let exit_on_last_window =
    Array.exists (Sys.get_argv ()) ~f:(String.equal "--exit-on-close")
  in
  let startup_links =
    let arguments = Array.to_list (Sys.get_argv ()) |> List.tl_exn in
    let before =
      List.take_while arguments ~f:(fun arg -> not (String.equal arg "--open-uris"))
    in
    let trailing =
      match
        List.drop_while arguments ~f:(fun arg -> not (String.equal arg "--open-uris"))
      with
      | [] -> []
      | _marker :: links -> links
    in
    List.filter_map before ~f:(String.chop_prefix ~prefix:"--open-uri=") @ trailing
  in
  (App.run_desktop
     Application_identity.value
     ~motion
     ~startup_links
     ~exit_on_last_window
     (fun env app ->
        let initial = Workspace.create () in
        let state =
          Bonsai.Expert.Var.create
            { Ui.Snapshot.workspace = initial
            ; canvas = None
            ; chart = None
            ; command = None
            ; inspector = true
            ; compact = false
            ; running = false
            ; status = "Preparing workspace"
            ; extension_generation = 1L
            ; extension_command = None
            ; extension_disabled = false
            ; extension_visible = true
            ; documents = Documents.State.initial
            ; alerts = Alerts.State.initial
            ; alerts_open = false
            }
        in
        let update f = Bonsai.Expert.Var.set state (f (Bonsai.Expert.Var.get state)) in
        let alerts = ref None in
        let with_alerts f =
          Effect.bind
            (Effect.of_thunk (fun () -> !alerts))
            ~f:(function
              | None -> Effect.Ignore
              | Some alerts -> f alerts)
        in
        let current_window = ref None in
        let documents = ref None in
        let refresh_document () =
          Option.iter !documents ~f:(fun documents ->
            Effect.Expert.handle (Documents.refresh documents))
        in
        let canvas = ref None
        and chart = ref None in
        let sequence = ref 0L
        and ack = ref 0L in
        let chart_ready = ref 0L
        and extension_mounts = ref 0
        and extension_ack = ref 0L
        and motion_events = ref 0 in
        let layout = ref "" in
        let emit message = Eio.traceln "SIGNAL_STUDIO: %s" message in
        let publish workspace =
          Option.iter !canvas ~f:(fun source ->
            Scene.set source (Workspace.scene workspace) |> scene_ok);
          Option.iter !chart ~f:(fun source ->
            Chart.set source (Workspace.chart workspace) |> chart_ok);
          update (fun s -> { s with workspace; status = "Workspace updated" });
          refresh_document ()
        in
        let command action =
          sequence := Int64.succ !sequence;
          let command = Canvas.Command.create ~sequence:!sequence action |> ok in
          update (fun s -> { s with command = Some command });
          !sequence
        in
        let set_run run =
          match Workspace.set_run (Bonsai.Expert.Var.get state).workspace run with
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
          Effect.of_thunk (fun () ->
            match event.Canvas.Event.observation with
            | Selection_changed selected ->
              update (fun s ->
                { s with workspace = Workspace.select s.workspace selected |> ok });
              refresh_document ();
              emit "canvas selected"
            | Activated id ->
              update (fun s ->
                { s with
                  workspace = Workspace.select s.workspace (Some id) |> ok
                ; inspector = true
                });
              refresh_document ();
              emit "canvas activated"
            | Moved (id, transform) ->
              let point =
                Gpuio.Canvas_geometry.Transform.apply
                  transform
                  (Gpuio.Canvas_geometry.Point.create ~x:0. ~y:0. |> ok)
                |> ok
              in
              publish
                (Workspace.move (Bonsai.Expert.Var.get state).workspace id point |> ok);
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
          Effect.of_thunk (fun () ->
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
          | Gpuio.Extension.Event.Data value -> Effect.of_thunk (fun () -> set_run value)
          | Mounted ->
            Effect.of_thunk (fun () ->
              incr extension_mounts;
              emit "extension mounted")
          | Command_completed sequence ->
            Effect.of_thunk (fun () ->
              extension_ack := sequence;
              emit (sprintf "extension command completed %Ld" sequence))
          | Failed error ->
            Effect.of_thunk (fun () -> raise_s [%sexp (error : Gpuio.Extension.Error.t)])
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
                    let run = Workspace.run (Bonsai.Expert.Var.get state).workspace in
                    if epoch = !stream_epoch
                    then set_run (if run = 100 then 0 else run + 1);
                    Eio.Promise.resolve resolver ());
                  Eio.Promise.await promise
                done)
              ~on_result:(fun result ->
                Effect.bind
                  (Effect.of_thunk (fun () ->
                     ok result;
                     task := None;
                     update (fun s ->
                       { s with running = false; status = "Run sequence complete" });
                     emit "stream complete"))
                  ~f:(fun () ->
                    with_alerts (fun alerts ->
                      Alerts.notify
                        alerts
                        ~run:(Workspace.run (Bonsai.Expert.Var.get state).workspace))))
            |> ok
          in
          task := Some job
        in
        let document_controller =
          Documents.create
            app
            ~model:
              { Documents.Model.current =
                  (fun () -> (Bonsai.Expert.Var.get state).workspace)
              ; replace =
                  (fun workspace ->
                    publish workspace;
                    update (fun s ->
                      { s with extension_generation = Int64.succ s.extension_generation });
                    ignore
                      (command
                         (Select
                            (Option.map
                               (Workspace.selected workspace)
                               ~f:Workspace.Sample.id))
                       : int64))
              ; stop_stream = stop
              ; report =
                  (fun status ->
                    update (fun s -> { s with status });
                    emit status)
              }
            ~window:(fun () -> !current_window)
            ~load:(fun path ->
              File.load Eio.Path.(Eio.Stdenv.fs env / Gpuio.File_path.to_string path))
            ~save:(fun path workspace ->
              File.save
                Eio.Path.(Eio.Stdenv.fs env / Gpuio.File_path.to_string path)
                ~random:(Eio.Stdenv.secure_random env)
                workspace)
            ~on_state:(fun documents -> update (fun s -> { s with documents }))
        in
        documents := Some document_controller;
        let directory =
          let configured =
            Array.find_map
              (Sys.get_argv ())
              ~f:(String.chop_prefix ~prefix:"--directory=")
          in
          Option.first_some configured (Sys.getenv "HOME")
          |> Option.value ~default:"/"
          |> Gpuio.File_path.of_string
          |> Result.ok
          |> Option.value ~default:(Gpuio.File_path.of_string "/" |> ok)
        in
        let actions : Ui.Actions.t =
          { select =
              (fun id ->
                Effect.of_thunk (fun () -> ignore (command (Select (Some id)) : int64)))
          ; canvas = on_canvas
          ; chart = on_chart
          ; extension = on_extension
          ; inspector =
              Effect.of_thunk (fun () ->
                update (fun s -> { s with inspector = not s.inspector }))
          ; run =
              Effect.of_thunk (fun () ->
                if (Bonsai.Expert.Var.get state).running then stop () else start ())
          ; reset =
              Effect.of_thunk (fun () ->
                stop ();
                publish initial;
                update (fun s ->
                  { s with
                    extension_generation = Int64.succ s.extension_generation
                  ; extension_command = None
                  });
                ignore (command (Select None) : int64);
                Effect.Expert.handle (Documents.reset document_controller))
          ; lock_control =
              Effect.of_thunk (fun () ->
                update (fun s -> { s with extension_disabled = not s.extension_disabled }))
          ; hide_control =
              Effect.of_thunk (fun () ->
                update (fun s -> { s with extension_visible = not s.extension_visible }))
          ; reset_viewport =
              Effect.of_thunk (fun () -> ignore (command Reset_viewport : int64))
          ; on_layout =
              (fun selected ->
                Effect.of_thunk (fun () ->
                  layout := Gpuio.Container_query.Branch_id.to_string selected.branch;
                  update (fun s -> { s with compact = String.equal !layout "compact" });
                  emit ("layout " ^ !layout)))
          ; open_document = Documents.open_ document_controller ~directory
          ; save_document = Documents.save_as document_controller ~directory
          ; reveal_document = Documents.reveal document_controller
          ; quit = Effect.of_thunk (fun () -> App.shutdown app)
          ; toggle_alerts =
              Effect.of_thunk (fun () ->
                update (fun s -> { s with alerts_open = not s.alerts_open }))
          ; close_alerts =
              Effect.of_thunk (fun () -> update (fun s -> { s with alerts_open = false }))
          ; enable_alerts = with_alerts Alerts.enable
          ; notify_run =
              with_alerts (fun alerts ->
                Alerts.notify
                  alerts
                  ~run:(Workspace.run (Bonsai.Expert.Var.get state).workspace))
          ; dismiss_alert = with_alerts Alerts.dismiss
          ; on_motion =
              (fun ~name event ->
                Effect.of_thunk (fun () ->
                  incr motion_events;
                  if trace_motion
                  then
                    emit
                      (sprintf
                         "motion %s %s"
                         name
                         (Sexp.to_string
                            ([%sexp_of: Gpuio.Animation.Program.Event.t] event)))))
          }
        in
        let ensure_window ?(focus = true) () =
          match !current_window with
          | Some window when not (App.Window.is_closed window) -> window
          | None | Some _ ->
            let window =
              App.open_window
                app
                ~title:"GPUIO · Signal Studio"
                ~width:1160.
                ~height:860.
                ~focus
                (Component.component state actions)
              |> ok
            in
            current_window := Some window;
            let initialized = ref false in
            App.Window.on_change window (fun _ ->
              if !initialized
              then Effect.Ignore
              else (
                initialized := true;
                Documents.refresh document_controller));
            emit "window opened";
            window
        in
        let activate () =
          let window = ensure_window () in
          match App.Window.snapshot window with
          | None -> Effect.Ignore
          | Some _ ->
            Effect.map (App.Window.command window Activate) ~f:(function
              | Ok _ -> ()
              | Error error ->
                emit
                  ("Activation: "
                   ^ Sexp.to_string ([%sexp_of: Gpuio.Window.Error.t] error)))
        in
        let notification =
          N.attach app ~on_event:(fun event ->
            with_alerts (fun alerts -> Alerts.handle_event alerts event))
          |> function
          | Ok service -> service
          | Error error -> raise_s [%sexp (error : N.Error.t)]
        in
        let run_alerts =
          Alerts.create
            { Alerts.Backend.authorization = (fun () -> N.authorization notification)
            ; authorize =
                (fun () ->
                  Effect.bind
                    (Effect.of_thunk (fun () -> N.retry notification))
                    ~f:(fun () -> N.request_authorization notification))
            ; capabilities = (fun () -> N.capabilities notification)
            ; post =
                (fun content ->
                  N.post notification ~tag:(N.Tag.of_string "completed-run" |> ok) content)
            ; replace = N.replace notification
            ; dismiss = N.dismiss notification
            ; close = (fun () -> N.close notification)
            }
            ~activate:(fun () ->
              Effect.bind (activate ()) ~f:(fun () ->
                Effect.of_thunk (fun () -> emit "notification workspace activated")))
            ~on_state:(fun alerts -> update (fun s -> { s with alerts }))
            ~log:emit
        in
        alerts := Some run_alerts;
        let (_unregister_alerts : unit -> unit) =
          Scope.on_cancel (App.scope app) (fun () -> Alerts.close run_alerts) |> ok
        in
        App.on_reopen app (fun () -> activate ());
        let receiver =
          Desktop.attach app ~on_event:(function
            | Desktop.Event.Link link ->
              Effect.bind
                (Effect.of_thunk (fun () ->
                   match Workspace.route (Bonsai.Expert.Var.get state).workspace link with
                   | Error _ ->
                     emit "link rejected";
                     false
                   | Ok workspace ->
                     update (fun s -> { s with workspace });
                     ignore (ensure_window () : App.Window.t);
                     ignore
                       (command
                          (Select
                             (Option.map
                                (Workspace.selected workspace)
                                ~f:Workspace.Sample.id))
                        : int64);
                     refresh_document ();
                     emit ("link " ^ Gpuio.Deep_link.to_string link);
                     true))
                ~f:(fun accepted -> if accepted then activate () else Effect.Ignore)
            | Rejected_link _ -> Effect.of_thunk (fun () -> emit "link rejected")
            | Overflow count ->
              Effect.of_thunk (fun () -> emit (sprintf "link overflow %Ld" count))
            | Failed error ->
              Effect.of_thunk (fun () ->
                emit ("Desktop: " ^ Sexp.to_string ([%sexp_of: Desktop.Error.t] error))))
          |> function
          | Ok value -> value
          | Error error -> raise_s [%sexp (error : Desktop.Error.t)]
        in
        let window = ensure_window ~focus:(not (flag "--background")) () in
        let scope = App.scope app in
        Scope.start
          scope
          ~f:(fun () ->
            let clock = Eio.Stdenv.clock env in
            let timeout = if workload then 150. else 60. in
            Eio.Time.with_timeout_exn clock timeout (fun () ->
              let on_ui ui_effect = Ui_thread.perform scope ui_effect in
              let ui f = on_ui (Effect.of_thunk f) in
              let rec until f =
                if not (ui f)
                then (
                  Eio.Time.sleep clock 0.005;
                  until f)
              in
              let scene =
                on_ui (Scene.create app ~scope (Workspace.scene initial)) |> scene_ok
              in
              let signal =
                on_ui (Chart.create app ~scope (Workspace.chart initial)) |> chart_ok
              in
              ui (fun () ->
                canvas := Some scene;
                chart := Some signal;
                update (fun s ->
                  { s with
                    canvas = Some (Scene.handle scene)
                  ; chart = Some (Chart.handle signal)
                  ; status = "Ready to explore"
                  }));
              (* Routing needs live model/resources and a native window, not paint.
              macOS may defer an occluded window's first container-layout frame;
              waiting for chart paint here could prevent the link that activates it. *)
              until (fun () -> Option.is_some (App.Window.snapshot window));
              ui (fun () ->
                emit "ready";
                Desktop.ready receiver;
                N.ready notification);
              on_ui (Alerts.probe run_alerts);
              completed
              := Checks.run
                   { Checks.Context.initial
                   ; state
                   ; window
                   ; scene
                   ; signal
                   ; chart_ready
                   ; extension_mounts
                   ; extension_ack
                   ; ack
                   ; motion_events
                   ; layout
                   ; ensure_window
                   ; set_run
                   ; update
                   ; command
                   ; on_extension
                   ; publish
                   ; stop
                   ; document_controller
                   ; run_alerts
                   }
                   ~env
                   ~app
                   ~self_test
                   ~workload
                   ~unavailable_check
                   ~document_path))
          ~on_result:(fun result ->
            Effect.of_thunk (fun () ->
              ok result;
              if self_test || unavailable_check || workload then App.shutdown app))
        |> ok
        |> fun (_ : Scope.Task.t) -> ())
   |> function
   | Ok App.Launch_outcome.Exited -> ()
   | Ok Forwarded -> Eio.traceln "SIGNAL_STUDIO: forwarded"
   | Error error -> raise_s [%sexp (error : Desktop.Error.t)]);
  if self_test || unavailable_check || workload then assert !completed
;;
