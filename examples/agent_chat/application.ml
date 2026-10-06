open Core
module App = Gpuio_eio.App
module Scope = Gpuio_eio.Scope
module Conversation = Gpuio_agent_chat_runtime.Conversation
module Workspace = Gpuio_agent_chat_runtime.Workspace
module Backend = Gpuio_agent_chat_model.Fake_backend
module Effect = Bonsai.Effect

let output env text = Gpuio_eio.Output.write (Eio.Stdenv.stdout env) text

let run ~self_test ~native_test ~workload_metrics ~attachment_directory ~motion =
  let passed = ref false in
  App.run ~motion (fun env app ->
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
            (fun window ->
               let sources =
                 Gpuio_agent_chat_runtime.Sources.create
                   ~scope:(App.Window.scope window)
                   ~sleep
                   ~build_large:(fun () ->
                     Eio.Domain_manager.run
                       (Eio.Stdenv.domain_mgr env)
                       Gpuio_agent_chat_runtime.Source_data.large)
                 |> Or_error.ok_exn
               in
               let results =
                 Gpuio_agent_chat_runtime.Results.create
                   ~scope:(App.Window.scope window)
                   ~sleep
                   ~build:(fun source query ->
                     Eio.Domain_manager.run (Eio.Stdenv.domain_mgr env) (fun () ->
                       Gpuio_agent_chat_runtime.Result_data.replace source query
                       |> Or_error.ok_exn))
                 |> Or_error.ok_exn
               in
               Workspace.component
                 workspace
                 ~app
                 ~sources
                 ~results
                 ~open_window
                 ~read_file
                 ~attachment_directory
                 window)
          |> Or_error.ok_exn
        in
        if workload_metrics
        then
          Workspace.set_backend
            workspace
            (Backend.Config.create ~chunk_bytes:7 ~delay_seconds:5. () |> Or_error.ok_exn);
        Workspace.install_close_handler workspace window;
        App.Window.on_change window (fun _snapshot ->
          if !resources_started
          then Effect.Ignore
          else (
            resources_started := true;
            Effect.Many
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
    App.on_reopen app (fun () -> Effect.of_thunk (fun () -> open_window 1));
    if workload_metrics then Workload_metrics.start ~env ~app ~conversations;
    if self_test
    then
      Self_test.start ~env ~app ~conversations ~windows ~open_window ~on_pass:(fun () ->
        passed := true));
  if self_test then assert !passed;
  if native_test
  then Eio_main.run (fun env -> output env "GPUIO_AGENT_CHAT_NATIVE_APP_RETURNED\n")
;;
