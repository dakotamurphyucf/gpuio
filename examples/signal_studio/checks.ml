open Core
module W = Signal_studio_model.Workspace
module App = Gpuio_eio.App
module Scene = Gpuio_eio.Canvas
module Chart = Gpuio_eio.Chart
module B = Bonsai.Cont
module E = Bonsai.Effect
module Desktop = Gpuio_eio.Desktop
module File = Signal_studio_files.Document_file
module N = Gpuio_eio.Notification
module Alerts = Signal_studio_notifications.Run_alerts

let ok = Or_error.ok_exn
let emit message = Eio.traceln "SIGNAL_STUDIO: %s" message

module Context = struct
  type t =
    { initial : Signal_studio_model.Workspace.t
    ; state : Ui.Snapshot.t Bonsai.Cont.Expert.Var.t
    ; window : Gpuio_eio.App.Window.t
    ; scene : Gpuio_eio.Canvas.t
    ; signal : Gpuio_eio.Chart.t
    ; chart_ready : int64 ref
    ; extension_mounts : int ref
    ; extension_ack : int64 ref
    ; ack : int64 ref
    ; motion_events : int ref
    ; layout : string ref
    ; ensure_window : ?focus:bool -> unit -> Gpuio_eio.App.Window.t
    ; set_run : int -> unit
    ; update : (Ui.Snapshot.t -> Ui.Snapshot.t) -> unit
    ; command : Gpuio.Canvas.Command.Action.t -> int64
    ; on_extension : int Gpuio.Extension.Event.t -> unit Bonsai.Effect.t
    ; publish : Signal_studio_model.Workspace.t -> unit
    ; stop : unit -> unit
    ; document_controller : Documents.t
    ; run_alerts : Signal_studio_notifications.Run_alerts.t
    }
end

let run context ~env ~app ~self_test ~workload ~unavailable_check ~document_path =
  let { Context.initial
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
    =
    context
  in
  let completed = ref false in
  let clock = Eio.Stdenv.clock env in
  let on_ui ui_effect = Ui_thread.perform (App.scope app) ui_effect in
  let ui f = on_ui (E.of_thunk f) in
  let rec until f =
    if not (ui f)
    then (
      Eio.Time.sleep clock 0.005;
      until f)
  in
  if self_test || workload
  then until (fun () -> Int64.(!chart_ready > 0L) && !extension_mounts > 0);
  if workload
  then (
    let mono () = Eio.Time.Mono.now (Eio.Stdenv.mono_clock env) in
    let elapsed start = Mtime.Span.to_float_ns (Mtime.span start (mono ())) /. 1e6 in
    let peak_source = ref 0 in
    let peak_pending = ref 0 in
    let sample () =
      let d = App.diagnostics app in
      peak_source := Int.max !peak_source (d.chart_data_bytes + d.canvas_scene_bytes);
      peak_pending := Int.max !peak_pending d.pending_requests;
      d
    in
    let await f =
      until (fun () ->
        ignore (sample () : App.Diagnostics.t);
        f ())
    in
    let frame window =
      let promise, resolver = Eio.Promise.create () in
      ui (fun () ->
        App.Window.request_frame window ~on_rendered:(fun ~revision:_ ->
          E.of_thunk (fun () -> Eio.Promise.resolve resolver ()))
        |> ok);
      Eio.Promise.await promise
    in
    let settled () =
      Scene.is_published scene
      && Chart.is_published signal
      && (App.diagnostics app).pending_requests = 0
    in
    let counter_sequence = ref 0L in
    for cycle = 0 to 11 do
      let window = ui (fun () -> ensure_window ()) in
      await (fun () -> App.Window.is_open window);
      frame window;
      for batch = 0 to 7 do
        let before = ui sample in
        let previous_data = ui (fun () -> Chart.data signal) in
        let previous_revision = ui (fun () -> !chart_ready) in
        let start = mono () in
        (* One UI turn publishes four desired snapshots; resource
                    schedulers admit only the latest unstarted publication. *)
        let run =
          ui (fun () ->
            let run = ref 0 in
            for offset = 0 to 3 do
              run := ((((cycle * 8) + batch) * 4) + offset + 1) % 101;
              set_run !run
            done;
            !run)
        in
        let publish_ms = elapsed start in
        let changed =
          ui (fun () ->
            not (Option.equal Gpuio.Chart_data.equal previous_data (Chart.data signal)))
        in
        await (fun () ->
          Option.iter (Chart.error signal) ~f:(fun error ->
            raise_s [%sexp (error : Chart.Error.t)]);
          Option.iter (Scene.error scene) ~f:(fun error ->
            raise_s [%sexp (error : Scene.Error.t)]);
          settled () && ((not changed) || Int64.(!chart_ready > previous_revision)));
        frame window;
        let after = ui sample in
        ui (fun () ->
          assert (W.run (B.Expert.Var.get state).workspace = run);
          assert (
            Option.equal
              Gpuio.Chart_data.equal
              (Chart.data signal)
              (Some (W.chart (B.Expert.Var.get state).workspace)));
          assert (after.canvases = 1 && after.charts = 1));
        emit
          (sprintf
             "workload sample cycle=%d batch=%d run=%d publish_ms=%.3f \
              update_frame_ms=%.3f source_charge=%d submitted_bytes=%d \
              submitted_messages=%d"
             cycle
             batch
             run
             publish_ms
             (elapsed start)
             (after.chart_data_bytes + after.canvas_scene_bytes)
             (after.traffic.submitted_bytes - before.traffic.submitted_bytes)
             (after.traffic.submitted_messages - before.traffic.submitted_messages))
      done;
      (* A command is distinct from a property update: keep the same
                  generation, await its exact acknowledgement, then clear it so
                  later windows never replay a completed command. *)
      let mounts = ui (fun () -> !extension_mounts) in
      counter_sequence := Int64.succ !counter_sequence;
      let command_sequence = !counter_sequence in
      ui (fun () ->
        set_run (cycle + 20);
        update (fun s ->
          { s with extension_command = Some (command_sequence, cycle + 20) }));
      await (fun () -> Int64.equal !extension_ack command_sequence && settled ());
      frame window;
      ui (fun () ->
        assert (!extension_mounts = mounts);
        update (fun s -> { s with extension_command = None }));
      (* Hidden presentation retains its instance. Explicit generation
                  change retires it; window close must retire the replacement. *)
      ui (fun () -> update (fun s -> { s with extension_visible = false }));
      frame window;
      ui (fun () -> update (fun s -> { s with extension_visible = true }));
      frame window;
      ui (fun () -> assert (!extension_mounts = mounts));
      ui (fun () ->
        update (fun s ->
          { s with extension_generation = Int64.succ s.extension_generation }));
      await (fun () -> !extension_mounts = mounts + 1);
      frame window;
      ui (fun () -> App.Window.close window);
      await (fun () ->
        let d = App.diagnostics app in
        d.windows = 0 && d.pending_requests = 0 && d.queued_commands = 0);
      let closed = ui sample in
      assert (closed.charts = 1 && closed.canvases = 1);
      emit
        (sprintf
           "workload closed cycle=%d windows=%d scopes=%d tasks=%d source_charge=%d"
           cycle
           closed.windows
           closed.scopes.scopes
           closed.scopes.tasks
           (closed.chart_data_bytes + closed.canvas_scene_bytes))
    done;
    ui (fun () ->
      Scene.release scene;
      Chart.release signal;
      update (fun s -> { s with canvas = None; chart = None }));
    await (fun () ->
      let d = App.diagnostics app in
      d.charts = 0 && d.canvases = 0 && d.pending_requests = 0);
    let final = ui sample in
    assert (final.chart_data_bytes = 0 && final.canvas_scene_bytes = 0);
    emit
      (sprintf
         "workload passed samples=96 desired_updates=384 cycles=12 mounts=%d \
          peak_source_charge=%d peak_pending=%d final_source_charge=0 \
          native_queue_peak_bytes=%d"
         !extension_mounts
         !peak_source
         !peak_pending
         final.native_command_queue.peak_bytes);
    completed := true);
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
        String.is_substring (Alerts.state run_alerts).message ~substring:"unavailable"));
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
            App.Window.snapshot window |> Option.bind ~f:Desktop.Document.of_snapshot
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
          (File.load Eio.Path.(Eio.Stdenv.fs env / Gpuio.File_path.to_string path) |> ok)
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
              { Documents.Model.current = (fun () -> (B.Expert.Var.get state).workspace)
              ; replace = publish
              ; stop_stream = stop
              ; report = (fun message -> messages := message :: !messages)
              }
            ~window:(fun () -> Some window)
            ~load:(fun _ ->
              incr reads;
              Eio.Promise.await (if !reads = 1 then gate else reset_gate);
              Ok initial)
            ~save:(fun _ _ -> failwith "Busy controller admitted overlapping save")
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
      Eio.Path.unlink Eio.Path.(Eio.Stdenv.fs env / Gpuio.File_path.to_string path);
      on_ui (Documents.load_path document_controller path);
      wait_document document_controller;
      ui (fun () ->
        assert (W.run (B.Expert.Var.get state).workspace = 10);
        assert (
          String.equal (B.Expert.Var.get state).status "Could not load the workspace."));
      emit "missing-file load preserves workspace";
      emit
        "document snapshot, metadata, concurrent-edit, reset, invalid-load and busy \
         checks passed");
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
    completed := true);
  !completed
;;
