open Core
module App = Gpuio_eio.App
module Scope = Gpuio_eio.Scope
module Editor = Gpuio_eio.Text_input
module Document = Gpuio_eio.Document
module Pager = Gpuio_eio.List_paging
module Input = Gpuio.Text_input
module Source = Gpuio.Text_source
module Conversation = Gpuio_agent_chat_runtime.Conversation
module Workspace = Gpuio_agent_chat_runtime.Workspace
module Backend = Gpuio_agent_chat_model.Fake_backend
module List_view = Gpuio_bonsai.Virtual_list
module E = Bonsai.Effect

let perform scope ui_effect =
  let promise, resolve = Eio.Promise.create () in
  Scope.Expert.enqueue scope (fun () ->
    E.Expert.handle (E.map ui_effect ~f:(Eio.Promise.resolve resolve)));
  Eio.Promise.await promise
;;

let expect_editor result =
  match result with
  | Ok value -> value
  | Error error -> raise_s [%sexp (error : Input.Command_error.t)]
;;

let run ~self_test ~native_test ~attachment_directory =
  let passed = ref false in
  App.run (fun env app ->
    let clock = Eio.Stdenv.clock env in
    let sleep = Eio.Time.sleep clock in
    let app_scope = App.scope app in
    let conversations =
      List.mapi
        [ "Build a native app"; "Review a patch"; "Research notes" ]
        ~f:(fun i title ->
          Conversation.create app ~scope:app_scope ~id:(i + 1) ~title ~sleep
          |> Or_error.ok_exn)
    in
    let icons = Gpuio_agent_chat_runtime.Icons.create () in
    let resources_started = ref false in
    let windows = ref [] in
    let window_serial = ref 0 in
    let read_file path =
      Eio.Path.with_open_in
        Eio.Path.(Eio.Stdenv.fs env / Gpuio.File_path.to_string path)
        (fun flow -> Eio.Buf_read.(take_all (of_flow flow ~max_size:65537)))
    in
    let rec open_window selected =
      windows
      := List.filter !windows ~f:(fun (window, _) -> not (App.Window.is_closed window));
      if List.length !windows < 4
      then (
        incr window_serial;
        let workspace = Workspace.create ~icons conversations ~selected in
        let window =
          App.open_window
            app
            ~title:(sprintf "GPUIO · Agent workspace %d" !window_serial)
            ~width:1180.
            ~height:820.
            (Workspace.component workspace ~open_window ~read_file ~attachment_directory)
          |> Or_error.ok_exn
        in
        Workspace.install_close_handler workspace window;
        App.Window.on_change window (fun _snapshot ->
          if !resources_started
          then E.Ignore
          else (
            resources_started := true;
            E.Many
              (Gpuio_agent_chat_runtime.Icons.initialize icons app
               :: List.map conversations ~f:Conversation.initialize)));
        windows := !windows @ [ window, workspace ])
    in
    open_window 1;
    if native_test
    then
      Workspace.set_backend
        (snd (List.hd_exn !windows))
        (Backend.Config.create ~accept_delay_seconds:1.0 () |> Or_error.ok_exn);
    App.on_reopen app (fun () -> E.of_thunk (fun () -> open_window 1));
    if self_test
    then (
      let first, workspace = List.hd_exn !windows in
      Scope.start
        app_scope
        ~f:(fun () ->
          let started = Eio.Time.now clock in
          let wait f =
            Eio.Time.with_timeout_exn clock 25. (fun () ->
              while not (f ()) do
                sleep 0.005
              done)
          in
          let ui ui_effect = perform app_scope ui_effect in
          let sync f = ui (E.of_thunk f) in
          let panel id = Workspace.panel workspace id |> Option.value_exn in
          let conversation id =
            List.find_exn conversations ~f:(fun c -> Conversation.id c = id)
          in
          let one = conversation 1
          and two = conversation 2 in
          let ready workspace id =
            Option.exists (Workspace.panel workspace id) ~f:(fun p ->
              Option.is_some (Editor.snapshot p.editor))
          in
          wait (fun () ->
            ready workspace 1
            && Option.exists (List_view.Output.viewport (panel 1).list) ~f:(fun _ -> true));
          let fast =
            Backend.Config.create
              ~chunk_bytes:3
              ~delay_seconds:0.02
              ~accept_delay_seconds:0.3
              ()
            |> Or_error.ok_exn
          in
          sync (fun () -> Workspace.set_backend workspace fast);
          ignore
            (ui
               (Editor.replace
                  (panel 1).editor
                  ~selection:End
                  ~undo:Reset
                  "First prompt λ")
             |> expect_editor
             : Input.Snapshot.t);
          ignore (ui (Editor.submit (panel 1).editor) |> expect_editor : unit);
          wait (fun () -> Conversation.Phase.equal (Conversation.phase one) Accepting);
          ignore
            (ui
               (Editor.replace
                  (panel 1).editor
                  ~selection:End
                  ~undo:Record
                  "A newer draft")
             |> expect_editor
             : Input.Snapshot.t);
          wait (fun () -> Conversation.Phase.equal (Conversation.phase one) Streaming);
          wait (fun () ->
            String.is_substring (Workspace.notice workspace) ~substring:"newer draft");
          assert (
            String.equal
              (Input.Snapshot.text (Option.value_exn (Editor.snapshot (panel 1).editor)))
              "A newer draft");
          sync (fun () -> Workspace.select workspace 2);
          wait (fun () -> ready workspace 2);
          ignore
            (ui
               (Editor.replace
                  (panel 2).editor
                  ~selection:End
                  ~undo:Reset
                  "Independent second conversation")
             |> expect_editor
             : Input.Snapshot.t);
          ignore (ui (Editor.submit (panel 2).editor) |> expect_editor : unit);
          wait (fun () -> Conversation.Phase.equal (Conversation.phase two) Streaming);
          assert (Conversation.Phase.equal (Conversation.phase one) Streaming);
          sync (fun () -> Conversation.cancel two);
          wait (fun () -> Conversation.Phase.equal (Conversation.phase two) Cancelled);
          sync (fun () -> Workspace.select workspace 1);
          ui
            (List_view.Controller.scroll_to
               (List_view.Output.controller (panel 1).list)
               175
             |> Or_error.ok_exn);
          wait (fun () ->
            Option.exists (List_view.Output.viewport (panel 1).list) ~f:(fun v ->
              Option.exists v.anchor ~f:(fun (key, _) ->
                Gpuio.Key.equal key (Gpuio.Key.of_string_exn "175"))));
          assert (Conversation.Phase.equal (Conversation.phase one) Streaming);
          sync (fun () ->
            Pager.request (Conversation.pager one) Before |> Or_error.ok_exn);
          wait (fun () ->
            Gpuio.List_collection.length (Pager.items (Conversation.pager one)) >= 82);
          wait (fun () ->
            Option.exists (List_view.Output.viewport (panel 1).list) ~f:(fun v ->
              Option.exists v.anchor ~f:(fun (key, _) ->
                Gpuio.Key.equal key (Gpuio.Key.of_string_exn "175"))));
          wait (fun () -> Conversation.Phase.equal (Conversation.phase one) Complete);
          let document = Conversation.last_document one |> Option.value_exn in
          wait (fun () -> Document.is_published document);
          assert (
            String.equal
              (Source.to_string (Document.source document |> Option.value_exn))
              (Backend.response ~prompt:"First prompt λ"));
          assert (Scope.is_active (Conversation.scope one));
          assert (List_view.Output.active_rows (panel 1).list <= 32);
          sync (fun () -> Workspace.close_tab workspace 1);
          wait (fun () ->
            match ui (Editor.focus (panel 1).editor) with
            | Error Focus_blocked -> true
            | Ok _ | Error _ -> false);
          sync (fun () -> Workspace.select workspace 1);
          wait (fun () ->
            match ui (Editor.focus (panel 1).editor) with
            | Ok _ -> true
            | Error _ -> false);
          wait (fun () ->
            Option.exists (List_view.Output.viewport (panel 1).list) ~f:(fun v ->
              Option.exists v.anchor ~f:(fun (key, _) ->
                Gpuio.Key.equal key (Gpuio.Key.of_string_exn "175"))));
          assert (
            String.equal
              (Input.Snapshot.text (Option.value_exn (Editor.snapshot (panel 1).editor)))
              "A newer draft");
          sync (fun () -> open_window 1);
          let second, second_workspace = List.last_exn !windows in
          wait (fun () -> ready second_workspace 1);
          let second_panel () = Workspace.panel second_workspace 1 |> Option.value_exn in
          assert (
            String.is_empty
              (Input.Snapshot.text
                 (Option.value_exn (Editor.snapshot (second_panel ()).editor))));
          ignore
            (ui
               (Editor.replace
                  (second_panel ()).editor
                  ~selection:End
                  ~undo:Reset
                  "Other window draft")
             |> expect_editor
             : Input.Snapshot.t);
          wait (fun () ->
            String.equal
              (Input.Snapshot.text
                 (Option.value_exn (Editor.snapshot (second_panel ()).editor)))
              "Other window draft");
          (* A window owns pending acceptance, but never an accepted response. *)
          let three = conversation 3 in
          let late_accept = ref false in
          sync (fun () ->
            Conversation.submit
              three
              ~window_scope:(App.Window.scope first)
              ~config:
                (Backend.Config.create ~accept_delay_seconds:0.3 () |> Or_error.ok_exn)
              ~prompt:"Close before acceptance"
              ~on_accept:(E.of_thunk (fun () -> late_accept := true))
            |> Or_error.ok_exn);
          assert (Conversation.Phase.equal (Conversation.phase three) Accepting);
          let cancelled = ref false in
          sync (fun () ->
            Scope.Expert.on_cancel (App.Window.scope first) (fun () -> cancelled := true)
            |> Or_error.ok_exn
            |> fun (_ : unit -> unit) -> ());
          sync (fun () -> App.Window.close first);
          wait (fun () -> !cancelled && App.Window.is_closed first);
          assert (not (App.Window.is_closed second));
          let surviving =
            ui (Editor.read_snapshot (second_panel ()).editor) |> expect_editor
          in
          assert (String.equal (Input.Snapshot.text surviving) "Other window draft");
          sync (fun () ->
            Conversation.retry
              two
              ~config:
                (Backend.Config.create ~chunk_bytes:128 ~delay_seconds:0. ()
                 |> Or_error.ok_exn)
            |> Or_error.ok_exn);
          wait (fun () -> Conversation.Phase.equal (Conversation.phase two) Complete);
          assert (Conversation.Phase.equal (Conversation.phase three) Idle);
          assert ((not !late_accept) && Conversation.response_count three = 0);
          let before_attachment =
            Gpuio.List_collection.length (Pager.items (Conversation.pager one))
          in
          ui
            (Conversation.attach
               one
               ~name:"notes.txt"
               ~text:"Attachment λ\nLocal text preview.")
          |> Or_error.ok_exn;
          assert (
            Gpuio.List_collection.length (Pager.items (Conversation.pager one))
            = before_attachment + 1);
          assert (
            Result.is_error (ui (Conversation.attach one ~name:"binary" ~text:"\255")));
          assert (
            Result.is_error
              (ui
                 (Conversation.attach
                    one
                    ~name:"oversized.txt"
                    ~text:(String.make 65537 'x'))));
          sync (fun () ->
            Conversation.submit
              three
              ~window_scope:(App.Window.scope second)
              ~config:
                (Backend.Config.create
                   ~chunk_bytes:3
                   ~delay_seconds:0.
                   ~accept_delay_seconds:0.
                   ~fail_after_chunks:3
                   ()
                 |> Or_error.ok_exn)
              ~prompt:"Fail and retry"
              ~on_accept:E.Ignore
            |> Or_error.ok_exn);
          wait (fun () ->
            match Conversation.phase three with
            | Failed _ -> true
            | Idle | Accepting | Streaming | Complete | Cancelled -> false);
          let failed_document = Conversation.last_document three |> Option.value_exn in
          assert (
            Source.byte_length (Document.source failed_document |> Option.value_exn) > 0);
          sync (fun () ->
            Conversation.retry
              three
              ~config:
                (Backend.Config.create ~chunk_bytes:128 ~delay_seconds:0. ()
                 |> Or_error.ok_exn)
            |> Or_error.ok_exn);
          wait (fun () -> Conversation.Phase.equal (Conversation.phase three) Complete);
          assert (Conversation.response_count three = 1);
          sync (fun () -> Workspace.toggle_theme second_workspace);
          assert (not (Workspace.dark second_workspace));
          let rendered, resolve = Eio.Promise.create () in
          sync (fun () ->
            App.Window.request_frame second ~on_rendered:(fun ~revision:_ ->
              E.of_thunk (fun () -> Eio.Promise.resolve resolve ()))
            |> Or_error.ok_exn);
          Eio.Time.with_timeout_exn clock 5. (fun () -> Eio.Promise.await rendered);
          passed := true;
          let stats = App.stats app in
          Eio.Flow.copy_string
            (sprintf
               "GPUIO_AGENT_CHAT_METRICS elapsed_ms=%.0f stats=%s\n"
               ((Eio.Time.now clock -. started) *. 1000.)
               (Sexp.to_string (App.Stats.sexp_of_t stats)))
            (Eio.Stdenv.stdout env);
          Eio.Flow.copy_string
            "GPUIO_AGENT_CHAT_PUBLIC_OK: exact submit, protected draft, concurrent \
             streams, cancellation/retry, offscreen generation/history, retained tabs, \
             independent windows, attachments, errors/retry, pending-send cancellation, \
             theme and cleanup\n"
            (Eio.Stdenv.stdout env))
        ~on_result:(fun result ->
          E.of_thunk (fun () ->
            App.shutdown app;
            Or_error.ok_exn result))
      |> Or_error.ok_exn
      |> fun (_ : Scope.Task.t) -> ()));
  if self_test then assert !passed;
  if native_test
  then
    Eio_main.run (fun env ->
      Eio.Flow.copy_string
        "GPUIO_AGENT_CHAT_NATIVE_APP_RETURNED\n"
        (Eio.Stdenv.stdout env))
;;

let () =
  let flag value = Array.exists (Sys.get_argv ()) ~f:(String.equal value) in
  let attachment_directory =
    let args = Sys.get_argv () in
    Option.map
      (Array.findi args ~f:(fun _ value -> String.equal value "--directory"))
      ~f:(fun (index, _) ->
        if index + 1 >= Array.length args
        then failwith "--directory requires an absolute path";
        Gpuio.File_path.of_string args.(index + 1) |> Or_error.ok_exn)
  in
  run
    ~attachment_directory
    ~self_test:(flag "--self-test")
    ~native_test:(flag "--native-test")
;;
