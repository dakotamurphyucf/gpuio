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

let identity =
  Desktop.Identity.create
    ~identifier:"com.gpuio.signal-studio"
    ~name:"GPUIO Signal Studio"
    ~schemes:[ W.scheme ]
    ()
  |> ok
;;

let main () =
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
  (App.run_desktop identity ~motion ~startup_links ~exit_on_last_window (fun env app ->
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
         ; documents = Documents.State.initial
         ; alerts = Alerts.State.initial
         ; alerts_open = false
         }
     in
     let update f = B.Expert.Var.set state (f (B.Expert.Var.get state)) in
     let alerts = ref None in
     let with_alerts f =
       E.bind
         (E.of_thunk (fun () -> !alerts))
         ~f:(function
           | None -> E.Ignore
           | Some alerts -> f alerts)
     in
     let current_window = ref None in
     let documents = ref None in
     let refresh_document () =
       Option.iter !documents ~f:(fun documents ->
         E.Expert.handle (Documents.refresh documents))
     in
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
           refresh_document ();
           emit "canvas selected"
         | Activated id ->
           update (fun s ->
             { s with workspace = W.select s.workspace (Some id) |> ok; inspector = true });
           refresh_document ();
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
             E.bind
               (E.of_thunk (fun () ->
                  ok result;
                  task := None;
                  update (fun s ->
                    { s with running = false; status = "Run sequence complete" });
                  emit "stream complete"))
               ~f:(fun () ->
                 with_alerts (fun alerts ->
                   Alerts.notify alerts ~run:(W.run (B.Expert.Var.get state).workspace))))
         |> ok
       in
       task := Some job
     in
     let document_controller =
       Documents.create
         app
         ~model:
           { Documents.Model.current = (fun () -> (B.Expert.Var.get state).workspace)
           ; replace =
               (fun workspace ->
                 publish workspace;
                 update (fun s ->
                   { s with extension_generation = Int64.succ s.extension_generation });
                 ignore
                   (command (Select (Option.map (W.selected workspace) ~f:W.Sample.id))
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
         Array.find_map (Sys.get_argv ()) ~f:(String.chop_prefix ~prefix:"--directory=")
       in
       Option.first_some configured (Sys.getenv "HOME")
       |> Option.value ~default:"/"
       |> Gpuio.File_path.of_string
       |> Result.ok
       |> Option.value ~default:(Gpuio.File_path.of_string "/" |> ok)
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
             ignore (command (Select None) : int64);
             E.Expert.handle (Documents.reset document_controller))
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
       ; open_document = Documents.open_ document_controller ~directory
       ; save_document = Documents.save_as document_controller ~directory
       ; reveal_document = Documents.reveal document_controller
       ; quit = E.of_thunk (fun () -> App.shutdown app)
       ; toggle_alerts =
           E.of_thunk (fun () ->
             update (fun s -> { s with alerts_open = not s.alerts_open }))
       ; close_alerts =
           E.of_thunk (fun () -> update (fun s -> { s with alerts_open = false }))
       ; enable_alerts = with_alerts Alerts.enable
       ; notify_run =
           with_alerts (fun alerts ->
             Alerts.notify alerts ~run:(W.run (B.Expert.Var.get state).workspace))
       ; dismiss_alert = with_alerts Alerts.dismiss
       ; on_motion =
           (fun ~name event ->
             E.of_thunk (fun () ->
               incr motion_events;
               if trace_motion
               then
                 emit
                   (sprintf
                      "motion %s %s"
                      name
                      (Sexp.to_string ([%sexp_of: Gpuio.Animation.Program.Event.t] event)))))
       }
     in
     let component _window _graph =
       let open B.Let_syntax in
       let%arr state = B.Expert.Var.value state in
       Ui.view state actions
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
             component
           |> ok
         in
         current_window := Some window;
         let initialized = ref false in
         App.Window.on_change window (fun _ ->
           if !initialized
           then E.Ignore
           else (
             initialized := true;
             Documents.refresh document_controller));
         emit "window opened";
         window
     in
     let activate () =
       let window = ensure_window () in
       match App.Window.snapshot window with
       | None -> E.Ignore
       | Some _ ->
         E.map (App.Window.command window Activate) ~f:(function
           | Ok _ -> ()
           | Error error ->
             emit
               ("Activation: " ^ Sexp.to_string ([%sexp_of: Gpuio.Window.Error.t] error)))
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
               E.bind
                 (E.of_thunk (fun () -> N.retry notification))
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
           E.bind (activate ()) ~f:(fun () ->
             E.of_thunk (fun () -> emit "notification workspace activated")))
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
           E.bind
             (E.of_thunk (fun () ->
                match W.route (B.Expert.Var.get state).workspace link with
                | Error _ ->
                  emit "link rejected";
                  false
                | Ok workspace ->
                  update (fun s -> { s with workspace });
                  ignore (ensure_window () : App.Window.t);
                  ignore
                    (command (Select (Option.map (W.selected workspace) ~f:W.Sample.id))
                     : int64);
                  refresh_document ();
                  emit ("link " ^ Gpuio.Deep_link.to_string link);
                  true))
             ~f:(fun accepted -> if accepted then activate () else E.Ignore)
         | Rejected_link _ -> E.of_thunk (fun () -> emit "link rejected")
         | Overflow count ->
           E.of_thunk (fun () -> emit (sprintf "link overflow %Ld" count))
         | Failed error ->
           E.of_thunk (fun () ->
             emit ("Desktop: " ^ Sexp.to_string ([%sexp_of: Desktop.Error.t] error))))
       |> function
       | Ok value -> value
       | Error error -> raise_s [%sexp (error : Desktop.Error.t)]
     in
     let window = ensure_window () in
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
           ui (fun () ->
             emit "ready";
             Desktop.ready receiver;
             N.ready notification);
           on_ui (Alerts.probe run_alerts);
           if unavailable_check
           then (
             ui (fun () ->
               assert (
                 Option.equal
                   (Result.equal N.Authorization.equal N.Error.equal)
                   (Alerts.state run_alerts).authorization
                   (Some (Error Unavailable))));
             on_ui (Alerts.enable run_alerts);
             on_ui (Alerts.notify run_alerts ~run:12);
             ui (fun () ->
               assert (not (Alerts.state run_alerts).enabled);
               assert (
                 String.is_substring
                   (Alerts.state run_alerts).message
                   ~substring:"unavailable"));
             emit "unbundled alerts fallback passed";
             completed := true);
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
             Option.iter document_path ~f:(fun path ->
               let wait_document controller =
                 until (fun () -> not (Documents.state controller).busy)
               in
               let check_metadata ~edited =
                 until (fun () ->
                   match
                     App.Window.snapshot window
                     |> Option.bind ~f:Desktop.Document.of_snapshot
                   with
                   | None -> false
                   | Some document ->
                     Bool.equal (Desktop.Document.edited document) edited
                     && Option.equal
                          Gpuio.File_path.equal
                          (Desktop.Document.path document)
                          (Some path))
               in
               on_ui (Documents.save_path document_controller path);
               wait_document document_controller;
               check_metadata ~edited:false;
               ui (fun () ->
                 assert (not (Documents.state document_controller).edited);
                 assert (
                   Option.equal
                     Gpuio.File_path.equal
                     (Documents.state document_controller).path
                     (Some path));
                 set_run 8);
               on_ui
                 (E.bind (Documents.save_path document_controller path) ~f:(fun () ->
                    E.of_thunk (fun () -> set_run 9)));
               wait_document document_controller;
               check_metadata ~edited:true;
               ui (fun () -> assert (Documents.state document_controller).edited);
               assert (
                 W.run
                   (File.load
                      Eio.Path.(Eio.Stdenv.fs env / Gpuio.File_path.to_string path)
                    |> ok)
                 = 8);
               on_ui (Documents.load_path document_controller path);
               wait_document document_controller;
               ui (fun () ->
                 assert (W.run (B.Expert.Var.get state).workspace = 8);
                 assert (not (Documents.state document_controller).edited));
               (* Delay the same controller contract deterministically to check
                 concurrent edits and busy admission without slow real I/O. *)
               let gate, resolve = Eio.Promise.create () in
               let reset_gate, resolve_reset = Eio.Promise.create () in
               let reads = ref 0 in
               let messages = ref [] in
               let delayed =
                 ui (fun () ->
                   Documents.create
                     app
                     ~model:
                       { Documents.Model.current =
                           (fun () -> (B.Expert.Var.get state).workspace)
                       ; replace = publish
                       ; stop_stream = stop
                       ; report = (fun message -> messages := message :: !messages)
                       }
                     ~window:(fun () -> Some window)
                     ~load:(fun _ ->
                       incr reads;
                       Eio.Promise.await (if !reads = 1 then gate else reset_gate);
                       Ok initial)
                     ~save:(fun _ _ ->
                       failwith "Busy controller admitted overlapping save")
                     ~on_state:ignore)
               in
               on_ui (Documents.load_path delayed path);
               on_ui (Documents.save_path delayed path);
               ui (fun () -> set_run 10);
               Eio.Promise.resolve resolve ();
               wait_document delayed;
               ui (fun () ->
                 assert (W.run (B.Expert.Var.get state).workspace = 10);
                 assert (
                   List.mem
                     !messages
                     "A document operation is already in progress."
                     ~equal:String.equal);
                 assert (
                   List.mem
                     !messages
                     "Load abandoned: the workspace changed while reading."
                     ~equal:String.equal));
               on_ui (Documents.load_path delayed path);
               on_ui (Documents.reset delayed);
               Eio.Promise.resolve resolve_reset ();
               wait_document delayed;
               ui (fun () -> assert (W.run (B.Expert.Var.get state).workspace = 10));
               Eio.Path.save
                 Eio.Path.(Eio.Stdenv.fs env / Gpuio.File_path.to_string path)
                 ~create:(`Or_truncate 0o600)
                 "(invalid document)";
               on_ui (Documents.load_path document_controller path);
               wait_document document_controller;
               ui (fun () -> assert (W.run (B.Expert.Var.get state).workspace = 10));
               Eio.Path.unlink
                 Eio.Path.(Eio.Stdenv.fs env / Gpuio.File_path.to_string path);
               on_ui (Documents.load_path document_controller path);
               wait_document document_controller;
               ui (fun () ->
                 assert (W.run (B.Expert.Var.get state).workspace = 10);
                 assert (
                   String.equal
                     (B.Expert.Var.get state).status
                     "Could not load the workspace."));
               emit "missing-file load preserves workspace";
               emit
                 "document snapshot, metadata, concurrent-edit, reset, invalid-load and \
                  busy checks passed");
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
           if self_test || unavailable_check then App.shutdown app))
     |> ok
     |> fun (_ : Scope.Task.t) -> ())
   |> function
   | Ok App.Launch_outcome.Exited -> ()
   | Ok Forwarded -> Eio.traceln "SIGNAL_STUDIO: forwarded"
   | Error error -> raise_s [%sexp (error : Desktop.Error.t)]);
  if self_test || unavailable_check then assert !completed
;;

let () =
  match Array.to_list (Sys.get_argv ()) with
  | [ _; "--print-info-plist" ] ->
    let text =
      Gpuio.Desktop_package.macos_info_plist
        identity
        ~executable:"gpuio-signal"
        ~version:"0.1.0"
        ~build:"1"
      |> ok
      |> Gpuio.Desktop_package.contents
    in
    Eio_main.run (fun env -> Eio.Flow.copy_string text (Eio.Stdenv.stdout env))
  | [ _; "--print-desktop-entry"; executable ] ->
    let executable = Gpuio.File_path.of_string executable |> ok in
    let text =
      Gpuio.Desktop_package.linux_entry identity ~executable ()
      |> ok
      |> Gpuio.Desktop_package.contents
    in
    Eio_main.run (fun env -> Eio.Flow.copy_string text (Eio.Stdenv.stdout env))
  | _ -> main ()
;;
