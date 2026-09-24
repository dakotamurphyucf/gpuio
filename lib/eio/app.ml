open Core
open Gpuio_protocol
module Guard = Gpuio_runtime_core.Domain_guard
module Driver = Gpuio_runtime_core.Window_driver
module Input = Gpuio.Text_input
module Dialog = Gpuio.File_dialog
module Native_window = Gpuio.Window

type editor_result = (Input.Snapshot.t, Input.Command_error.t) Result.t

type editor_request =
  { window : Window_id.t
  ; node : Node_id.t
  ; complete : editor_result -> unit
  }

type window_result = (Native_window.Snapshot.t, Native_window.Error.t) Result.t

type window_request =
  { window : Window_id.t
  ; complete : window_result -> unit
  }

type dialog_result = Wire.File_dialog.Result.t

type dialog_request =
  { window : Window_id.t
  ; complete : dialog_result -> unit
  }

module Stats = struct
  type t =
    { turns : int
    ; clock_ticks : int
    ; commits : int
    ; rendered : int
    ; completed_jobs : int
    }
  [@@deriving sexp_of]
end

type phase =
  | Opening
  | Open
  | Closing_before_open
  | Closing
  | Closed

type t =
  { guard : Guard.t
  ; native : Gpuio_native.t
  ; inbox : Inbox.t
  ; scope : Scope.t
  ; now : unit -> Time_ns.t
  ; mutable windows : window Int.Map.t
  ; mutable generations : int64 Int.Map.t
  ; commands : Wire.Message.t Queue.t
  ; mutable opens : Wire.Message.t Int64.Map.t
  ; mutable closes : Window_id.t Int64.Map.t
  ; mutable frames : (Window_id.t * (revision:int64 -> unit Bonsai.Effect.t)) Int64.Map.t
  ; mutable editors : editor_request Int64.Map.t
  ; mutable dialogs : dialog_request Int64.Map.t
  ; mutable window_requests : window_request Int64.Map.t
  ; mutable window_capabilities : Native_window.Capabilities.t option
  ; mutable quit_pending : int64 option
  ; mutable on_reopen : unit -> unit Bonsai.Effect.t
  ; asset_registry : Asset_registry.t
  ; document_registry : Document_registry.t
  ; mutable assets : (Wire.Asset.Response.t -> unit) Int64.Map.t
  ; mutable documents : (Wire.Document.Response.t -> unit) Int64.Map.t
  ; mutable correlation : int64
  ; mutable motion : Gpuio.Animation.Preference.t option
  ; mutable welcomed : bool
  ; mutable stopping : bool
  ; mutable stopped : bool
  ; mutable stats : Stats.t
  }

and window =
  { app : t
  ; id : Window_id.t
  ; scope : Scope.t
  ; mutable phase : phase
  ; mutable driver : Driver.t option
  ; mutable snapshot : Native_window.Snapshot.t option
  ; mutable on_change : Native_window.Snapshot.t -> unit Bonsai.Effect.t
  ; mutable on_close :
      Native_window.Close_reason.t -> Native_window.Close_decision.t Bonsai.Effect.t
  ; mutable close_pending : int64 option
  ; mutable close_requested : bool
  ; mutable close_waiters :
      (Native_window.Close_reason.t * (Native_window.Close_decision.t -> unit)) list
  }

let check t = Guard.check t.guard

let scope t =
  check t;
  t.scope
;;

let stats t =
  check t;
  t.stats
;;

let correlation t =
  if Int64.equal t.correlation Int64.max_value then failwith "request identity exhausted";
  t.correlation <- Int64.succ t.correlation;
  t.correlation
;;

let queue t message =
  Queue.enqueue t.commands message;
  Inbox.wake t.inbox
;;

let set_motion t preference =
  check t;
  if not t.stopping
  then (
    t.motion <- Some preference;
    Inbox.wake t.inbox)
;;

module Expert = struct
  let document_request t ~limit request =
    Bonsai.Effect.Expert.of_fun ~f:(fun ~callback ->
      check t;
      let fail error = callback (Wire.Document.Response.Failed error) in
      if t.stopping
      then fail Closed
      else if not t.welcomed
      then fail Not_ready
      else if Map.length t.documents >= limit
      then fail Resource_limit
      else (
        let oversized =
          match request with
          | Wire.Document.Request.Chunk (_, _, _, data) ->
            String.length data > Wire.Document.max_chunk_bytes
          | Create | Begin _ | Publish _ | Abort _ | Release _ -> false
        in
        if oversized
        then fail Invalid_range
        else (
          let id = correlation t in
          t.documents <- Map.set t.documents ~key:id ~data:callback;
          queue t (Document (id, request)))))
  ;;

  let document t request = document_request t ~limit:63 request

  let register_document t ~scope source =
    Bonsai.Effect.Expert.of_fun ~f:(fun ~callback ->
      check t;
      if t.stopping
      then callback (Error Wire.Document.Error.Closed)
      else
        Document_registry.register t.document_registry ~scope source ~on_result:callback)
  ;;

  let asset_request t ~limit request =
    Bonsai.Effect.Expert.of_fun ~f:(fun ~callback ->
      check t;
      let fail error = callback (Wire.Asset.Response.Failed error) in
      if t.stopping
      then fail Closed
      else if not t.welcomed
      then fail Not_ready
      else if Map.length t.assets >= limit
      then fail Resource_limit
      else (
        let id = correlation t in
        let oversized =
          match request with
          | Wire.Asset.Request.Append (_, _, data) ->
            String.length data > Wire.Asset.max_chunk_bytes
          | Begin _ | Finish _ | Release _ -> false
        in
        if oversized
        then fail Invalid_chunk
        else (
          t.assets <- Map.set t.assets ~key:id ~data:callback;
          queue t (Asset (id, request)))))
  ;;

  let asset t request = asset_request t ~limit:63 request

  let register_asset t ~scope source =
    Bonsai.Effect.Expert.of_fun ~f:(fun ~callback ->
      check t;
      if t.stopping
      then callback (Error Asset_registry.Error.Closed)
      else if not t.welcomed
      then callback (Error Asset_registry.Error.Not_ready)
      else Asset_registry.register t.asset_registry ~scope source ~on_result:callback)
  ;;
end

let complete_close window decision =
  let waiters = window.close_waiters in
  window.close_waiters <- [];
  window.close_pending <- None;
  window.close_requested <- false;
  List.iter waiters ~f:(fun (_, complete) -> complete decision)
;;

let enqueue t f = Scope.Expert.enqueue t.scope f

let decide_close window reason complete =
  window.close_waiters
  <- List.filter window.close_waiters ~f:(fun (existing, _) ->
       not (Native_window.Close_reason.equal existing reason))
     @ [ reason, complete ];
  if Option.is_none window.close_pending
  then (
    let token = correlation window.app in
    window.close_pending <- Some token;
    enqueue window.app (fun () ->
      if Option.equal Int64.equal window.close_pending (Some token)
      then
        Bonsai.Effect.Expert.handle
          (Bonsai.Effect.map (window.on_close reason) ~f:(fun decision ->
             enqueue window.app (fun () ->
               if Option.equal Int64.equal window.close_pending (Some token)
               then complete_close window decision)))))
;;

let release_window window =
  match window.phase with
  | Closed -> ()
  | Opening | Open | Closing_before_open | Closing ->
    window.phase <- Closed;
    complete_close window Allow;
    let cancelled_windows, remaining_windows =
      Map.partition_tf window.app.window_requests ~f:(fun request ->
        Window_id.equal request.window window.id)
    in
    window.app.window_requests <- remaining_windows;
    Map.iter cancelled_windows ~f:(fun request ->
      request.complete (Error Native_window.Error.Closed));
    let cancelled, remaining =
      Map.partition_tf window.app.editors ~f:(fun request ->
        Window_id.equal request.window window.id)
    in
    window.app.editors <- remaining;
    Map.iter cancelled ~f:(fun request -> request.complete (Error Closed));
    let cancelled_dialogs, remaining_dialogs =
      Map.partition_tf window.app.dialogs ~f:(fun request ->
        Window_id.equal request.window window.id)
    in
    window.app.dialogs <- remaining_dialogs;
    Map.iter cancelled_dialogs ~f:(fun request ->
      request.complete (Wire.File_dialog.Result.Failed Closed));
    Scope.cancel window.scope;
    Option.iter window.driver ~f:Driver.close;
    window.driver <- None;
    window.on_change <- (fun _ -> Bonsai.Effect.Ignore);
    window.on_close <- (fun _ -> Bonsai.Effect.return Native_window.Close_decision.Allow);
    window.app.frames
    <- Map.filter window.app.frames ~f:(fun (id, _) -> not (Window_id.equal id window.id));
    window.app.windows
    <- Map.remove window.app.windows (Window_id.slot window.id |> Int64.to_int_exn)
;;

let shutdown t =
  check t;
  if not t.stopping
  then (
    t.stopping <- true;
    Asset_registry.close t.asset_registry;
    Document_registry.close t.document_registry;
    Scope.cancel t.scope;
    queue t Shutdown)
;;

module Window = struct
  type t = window

  let scope t =
    check t.app;
    t.scope
  ;;

  let is_closed t =
    check t.app;
    match t.phase with
    | Closing_before_open | Closing | Closed -> true
    | Opening | Open -> false
  ;;

  let close t =
    check t.app;
    match t.phase with
    | Closing_before_open | Closing | Closed -> ()
    | Opening ->
      t.phase <- Closing_before_open;
      complete_close t Allow;
      Scope.cancel t.scope;
      Inbox.wake t.app.inbox
    | Open ->
      t.phase <- Closing;
      complete_close t Allow;
      Scope.cancel t.scope;
      let request = correlation t.app in
      t.app.closes <- Map.set t.app.closes ~key:request ~data:t.id;
      queue t.app (Close (request, t.id))
  ;;

  let request_close t =
    check t.app;
    if not (is_closed t || t.app.stopping || t.close_requested)
    then (
      t.close_requested <- true;
      decide_close t Window_close (function
        | Allow -> close t
        | Keep_open -> ()))
  ;;

  let set_close_handler t f =
    check t.app;
    t.on_close <- f
  ;;

  let snapshot t =
    check t.app;
    t.snapshot
  ;;

  let on_change t f =
    check t.app;
    t.on_change <- f
  ;;

  let command t command =
    Bonsai.Effect.Expert.of_fun ~f:(fun ~callback ->
      check t.app;
      let fail error = callback (Error error) in
      if is_closed t || t.app.stopping
      then fail Native_window.Error.Closed
      else if Result.is_error (Native_window.Command.validate command)
      then fail Invalid_request
      else if
        match t.phase with
        | Open -> false
        | Opening | Closing_before_open | Closing | Closed -> true
      then fail Not_ready
      else if Map.length t.app.window_requests >= 64
      then fail Busy
      else (
        let request = correlation t.app in
        t.app.window_requests
        <- Map.set
             t.app.window_requests
             ~key:request
             ~data:{ window = t.id; complete = callback };
        queue t.app (Window_command (request, t.id, command))))
  ;;

  let set_theme t theme =
    check t.app;
    Option.iter t.driver ~f:(fun driver -> Driver.set_theme driver theme);
    Inbox.wake t.app.inbox
  ;;

  let request_frame t ~on_rendered =
    check t.app;
    if
      (match t.phase with
       | Open -> false
       | Opening | Closing_before_open | Closing | Closed -> true)
      || t.app.stopping
    then Or_error.error_string "window is not open or application is stopping"
    else if Map.exists t.app.frames ~f:(fun (id, _) -> Window_id.equal id t.id)
    then Or_error.error_string "frame request already pending"
    else if Map.length t.app.frames >= 64
    then Or_error.error_string "frame request limit reached"
    else (
      let request = correlation t.app in
      t.app.frames <- Map.set t.app.frames ~key:request ~data:(t.id, on_rendered);
      queue t.app (Request_frame (request, t.id));
      Ok ())
  ;;

  module Expert = struct
    let file_dialog_raw t config =
      Bonsai.Effect.Expert.of_fun ~f:(fun ~callback ->
        check t.app;
        let fail error = callback (Wire.File_dialog.Result.Failed error) in
        if is_closed t || t.app.stopping
        then fail Closed
        else if
          match t.phase with
          | Open -> false
          | Opening | Closing_before_open | Closing | Closed -> true
        then fail Not_ready
        else if
          Map.exists t.app.dialogs ~f:(fun request -> Window_id.equal request.window t.id)
        then fail Busy
        else (
          let request = correlation t.app in
          t.app.dialogs
          <- Map.set
               t.app.dialogs
               ~key:request
               ~data:{ window = t.id; complete = callback };
          queue t.app (File_dialog (request, t.id, config))))
    ;;

    let file_dialog t config =
      Bonsai.Effect.map
        (file_dialog_raw t (Dialog.Expert.to_wire config))
        ~f:(Dialog.Expert.result_of_wire config)
    ;;

    let file_dialog_capabilities t =
      Bonsai.Effect.map
        (file_dialog_raw t Capabilities)
        ~f:Dialog.Expert.capabilities_of_wire
    ;;

    let editor_command t snapshot command =
      Bonsai.Effect.Expert.of_fun ~f:(fun ~callback ->
        check t.app;
        let complete = callback in
        let invalid_text =
          match command with
          | Input.Command.Replace { text; _ } ->
            if String.length text > Input.max_text_bytes
            then Some Input.Command_error.Limit_exceeded
            else if Result.is_error (Input.validate_text ~mode:Multiline text)
            then Some Invalid_text
            else None
          | Select _ | Focus | Undo | Redo -> None
        in
        if is_closed t || t.app.stopping
        then complete (Error Input.Command_error.Closed)
        else if Option.is_some invalid_text
        then complete (Error (Option.value_exn invalid_text))
        else if not (Window_id.equal t.id (Input.Expert.window snapshot))
        then complete (Error Stale_editor)
        else if Map.length t.app.editors >= 64
        then complete (Error Busy)
        else (
          let request = correlation t.app in
          let node = Input.Expert.node snapshot in
          t.app.editors
          <- Map.set t.app.editors ~key:request ~data:{ window = t.id; node; complete };
          queue
            t.app
            (Editor_command (request, t.id, node, Input.Expert.command_to_wire command))))
    ;;
  end
end

let window_capabilities t =
  check t;
  t.window_capabilities
;;

let on_reopen t f =
  check t;
  t.on_reopen <- f
;;

let request_quit t =
  check t;
  if (not t.stopping) && Option.is_none t.quit_pending
  then (
    let token = correlation t in
    let windows =
      Map.data t.windows |> List.filter ~f:(fun window -> not (Window.is_closed window))
    in
    t.quit_pending <- Some token;
    let remaining = ref (List.length windows) in
    let complete decision =
      if Option.equal Int64.equal t.quit_pending (Some token)
      then (
        match decision with
        | Native_window.Close_decision.Keep_open -> t.quit_pending <- None
        | Allow ->
          decr remaining;
          if !remaining = 0
          then (
            t.quit_pending <- None;
            shutdown t))
    in
    if List.is_empty windows
    then (
      t.quit_pending <- None;
      shutdown t)
    else
      List.iter windows ~f:(fun window -> decide_close window Application_quit complete))
;;

let open_window_config t ?(theme = Gpuio.Theme.default) config component =
  let config = Native_window.Config.Expert.to_wire config in
  let { Wire.Window.Config.title; width; height; focus = _; chrome = _; resizable = _ } =
    config
  in
  check t;
  if t.stopping || Option.is_some t.quit_pending
  then Or_error.error_string "application stopping or quit decision pending"
  else if
    String.is_empty title
    || (not (Stdlib.String.is_valid_utf_8 title))
    || String.length title > 4096
    || (not (Float.is_finite width && Float.is_finite height))
    || Float.(width < 1. || height < 1. || width > 16384. || height > 16384.)
  then Or_error.error_string "invalid window title or dimensions"
  else (
    match
      List.find (List.init 32 ~f:Fn.id) ~f:(fun slot ->
        (not (Map.mem t.windows slot))
        && not
             (Int64.equal
                (Map.find t.generations slot |> Option.value ~default:0L)
                0xffff_ffffL))
    with
    | None -> Or_error.error_string "native window limit reached"
    | Some slot ->
      let open Or_error.Let_syntax in
      let generation =
        Int64.succ (Map.find t.generations slot |> Option.value ~default:0L)
      in
      let%bind id = Window_id.create ~slot:(Int64.of_int slot) ~generation in
      let%bind scope = Scope.child t.scope ~name:"window" in
      let window =
        { app = t
        ; id
        ; scope
        ; phase = Opening
        ; driver = None
        ; snapshot = None
        ; on_change = (fun _ -> Bonsai.Effect.Ignore)
        ; on_close = (fun _ -> Bonsai.Effect.return Native_window.Close_decision.Allow)
        ; close_pending = None
        ; close_requested = false
        ; close_waiters = []
        }
      in
      let driver =
        try
          Driver.create
            ~asset_owner:(Asset_registry.Expert.owner t.asset_registry)
            ~document_owner:(Document_registry.Expert.owner t.document_registry)
            id
            ~start:(t.now ())
            ~theme
            (component window)
        with
        | exn ->
          Scope.cancel scope;
          raise exn
      in
      window.driver <- Some driver;
      t.generations <- Map.set t.generations ~key:slot ~data:generation;
      t.windows <- Map.set t.windows ~key:slot ~data:window;
      let request = correlation t in
      let message = Wire.Message.Open_configured (request, id, config) in
      t.opens <- Map.set t.opens ~key:request ~data:message;
      queue t message;
      Ok window)
;;

let open_window
      t
      ?theme
      ?(focus = true)
      ?(chrome = Native_window.Chrome.Standard)
      ?(resizable = true)
      ~title
      ~width
      ~height
      component
  =
  Result.bind
    (Native_window.Config.create ~focus ~chrome ~resizable ~title ~width ~height ())
    ~f:(fun config -> open_window_config t ?theme config component)
;;

let find_window t id =
  Map.find t.windows (Window_id.slot id |> Int64.to_int_exn)
  |> Option.filter ~f:(fun window -> Window_id.equal window.id id)
;;

let native_error code = Error.create_s (Wire.Error_code.sexp_of_t code)

let process t = function
  | Wire.Event.Close_requested id ->
    Option.iter (find_window t id) ~f:Window.request_close
  | Quit_requested -> request_quit t
  | Reopen_requested ->
    if not t.stopping
    then enqueue t (fun () -> Bonsai.Effect.Expert.handle (t.on_reopen ()))
  | Window_capabilities capabilities -> t.window_capabilities <- Some capabilities
  | Window_changed (id, snapshot) ->
    Option.iter (find_window t id) ~f:(fun window ->
      if not (Window.is_closed window)
      then (
        let changed =
          not (Option.equal Native_window.Snapshot.equal window.snapshot (Some snapshot))
        in
        window.snapshot <- Some snapshot;
        if changed then Bonsai.Effect.Expert.handle (window.on_change snapshot)))
  | Window_response (request, id, response) ->
    (match Map.find t.window_requests request with
     | Some pending when Window_id.equal pending.window id ->
       t.window_requests <- Map.remove t.window_requests request;
       let result =
         match response with
         | Wire.Window.Response.Observed snapshot -> Ok snapshot
         | Failed error -> Error error
       in
       pending.complete result
     | Some _ | None -> ())
  | Wire.Event.Welcome _ -> t.welcomed <- true
  | Opened (request, id) ->
    t.opens <- Map.remove t.opens request;
    Option.iter (find_window t id) ~f:(fun window ->
      match window.phase with
      | Opening -> window.phase <- Open
      | Closing_before_open ->
        window.phase <- Open;
        Window.close window
      | Open | Closing | Closed -> ())
  | Closed (request, id) ->
    t.closes <- Map.remove t.closes request;
    Option.iter (find_window t id) ~f:release_window
  | List_retained (id, revision, notices) ->
    Option.iter (find_window t id) ~f:(fun window ->
      if not (Window.is_closed window)
      then
        Option.iter window.driver ~f:(fun driver ->
          Driver.retry_list_rows driver ~revision notices |> Or_error.ok_exn));
    Inbox.wake t.inbox
  | Accepted (id, revision) ->
    Option.iter (find_window t id) ~f:(fun window ->
      if not (Window.is_closed window)
      then
        Option.iter window.driver ~f:(fun driver ->
          Driver.acknowledge driver ~revision |> Or_error.ok_exn));
    Inbox.wake t.inbox
  | Document_navigation (id, _, _, _, source, generation, _) as event ->
    if Document_registry.accepts_navigation t.document_registry source ~generation
    then
      Option.iter (find_window t id) ~f:(fun window ->
        if not (Window.is_closed window)
        then Option.iter window.driver ~f:(fun driver -> Driver.dispatch driver event))
  | ( Press (id, _, _, _)
    | Editor_event (id, _, _, _, _, _)
    | Choice (id, _, _, _, _)
    | Combobox_selected (id, _, _, _, _, _)
    | Palette_dismissed (id, _, _, _, _)
    | Toast_dismissed (id, _, _, _, _)
    | Drag_source_event (id, _, _, _, _)
    | Image_state (id, _, _, _, _)
    | Animation_endpoint (id, _, _, _, _)
    | List_viewport (id, _, _, _, _)
    | Drop_target_event (id, _, _, _, _)
    | Pointer_event (id, _, _, _, _)
    | Overlay_dismissed (id, _, _, _, _)
    | Tooltip_open_changed (id, _, _, _, _)
    | Command_invoked (id, _, _, _, _, _, _) ) as event ->
    Option.iter (find_window t id) ~f:(fun window ->
      if not (Window.is_closed window)
      then Option.iter window.driver ~f:(fun driver -> Driver.dispatch driver event))
  | Document_response (request, response) ->
    (match Map.find t.documents request with
     | None -> ()
     | Some complete ->
       t.documents <- Map.remove t.documents request;
       complete (if t.stopping then Wire.Document.Response.Failed Closed else response))
  | Asset_response (request, response) ->
    (match Map.find t.assets request with
     | None -> ()
     | Some complete ->
       t.assets <- Map.remove t.assets request;
       complete (if t.stopping then Wire.Asset.Response.Failed Closed else response))
  | File_dialog_result (request, id, result) ->
    (match Map.find t.dialogs request with
     | Some pending when Window_id.equal pending.window id ->
       t.dialogs <- Map.remove t.dialogs request;
       let result =
         match find_window t id with
         | Some window when (not (Window.is_closed window)) && not t.stopping -> result
         | Some _ | None -> Wire.File_dialog.Result.Failed Closed
       in
       pending.complete result
     | Some _ | None -> ())
  | Editor_result (request, id, node, result) ->
    (match Map.find t.editors request with
     | Some pending
       when Window_id.equal pending.window id && Node_id.equal pending.node node ->
       t.editors <- Map.remove t.editors request;
       let result =
         match result with
         | Failed error -> Error (Input.Expert.error_of_wire error)
         | Applied snapshot ->
           Input.Expert.snapshot_of_wire ~window:id ~node snapshot
           |> Result.map_error ~f:(fun _ -> Input.Command_error.Native_failure)
       in
       pending.complete result
     | Some _ | None -> ())
  | Rendered _ -> t.stats <- { t.stats with rendered = t.stats.rendered + 1 }
  | Frame_requested (request, id, revision) ->
    (match Map.find t.frames request with
     | Some (expected, callback) when Window_id.equal expected id ->
       t.frames <- Map.remove t.frames request;
       Option.iter (find_window t id) ~f:(fun window ->
         if not (Window.is_closed window)
         then Bonsai.Effect.Expert.handle (callback ~revision))
     | Some _ | None -> ())
  | Failed (request, _) when Map.mem t.window_requests request ->
    let pending = Map.find_exn t.window_requests request in
    t.window_requests <- Map.remove t.window_requests request;
    pending.complete (Error Native_window.Error.Native_failure)
  | Failed (request, code) when Map.mem t.assets request ->
    let complete = Map.find_exn t.assets request in
    t.assets <- Map.remove t.assets request;
    let error : Wire.Asset.Error.t =
      match code with
      | Closed -> Closed
      | Not_ready -> Not_ready
      | Stale_handle -> Stale_handle
      | Busy | Limit_exceeded -> Resource_limit
      | Malformed
      | Unsupported_version
      | Unsupported_capability
      | Invalid_revision
      | Invalid_tree
      | Overloaded
      | Native_failure -> Native_failure
    in
    complete (Wire.Asset.Response.Failed error)
  | Failed (request, code) when Map.mem t.dialogs request ->
    let pending = Map.find_exn t.dialogs request in
    t.dialogs <- Map.remove t.dialogs request;
    let error : Wire.File_dialog.Error.t =
      match code with
      | Closed | Stale_handle -> Closed
      | Busy -> Busy
      | Limit_exceeded -> Limit_exceeded
      | Unsupported_version | Unsupported_capability -> Unsupported
      | Not_ready -> Not_ready
      | Malformed | Invalid_revision | Invalid_tree | Overloaded | Native_failure ->
        Native_failure
    in
    pending.complete (Wire.File_dialog.Result.Failed error)
  | Failed (request, code) when Map.mem t.editors request ->
    let pending = Map.find_exn t.editors request in
    t.editors <- Map.remove t.editors request;
    let error : Input.Command_error.t =
      match code with
      | Closed -> Closed
      | Stale_handle -> Stale_editor
      | Busy -> Busy
      | Limit_exceeded -> Limit_exceeded
      | Unsupported_version
      | Unsupported_capability
      | Malformed
      | Not_ready
      | Invalid_revision
      | Invalid_tree
      | Overloaded
      | Native_failure -> Native_failure
    in
    pending.complete (Error error)
  | Failed (_, (Closed | Stale_handle)) when t.stopping -> ()
  | Rejected (id, _, (Closed | Stale_handle))
    when t.stopping || Option.for_all (find_window t id) ~f:Window.is_closed -> ()
  | Failed (request, Busy) ->
    (match Map.find t.opens request with
     | Some message -> queue t message
     | None -> Error.raise (native_error Busy))
  | Failed (request, ((Closed | Stale_handle) as code)) ->
    (match Map.find t.closes request with
     | Some id ->
       t.closes <- Map.remove t.closes request;
       Option.iter (find_window t id) ~f:release_window
     | None ->
       if Map.mem t.frames request
       then t.frames <- Map.remove t.frames request
       else Error.raise (native_error code))
  | Failed (request, _) when Map.mem t.documents request ->
    let complete = Map.find_exn t.documents request in
    t.documents <- Map.remove t.documents request;
    complete (Wire.Document.Response.Failed Native_failure)
  | Failed (_, code) | Rejected (_, _, code) -> Error.raise (native_error code)
  | Overloaded _ -> failwith "native input mailbox overloaded"
  | Stopped ->
    t.stopped <- true;
    t.stopping <- true;
    Asset_registry.close t.asset_registry;
    Document_registry.close t.document_registry;
    let assets = t.assets in
    t.assets <- Int64.Map.empty;
    Map.iter assets ~f:(fun complete -> complete (Wire.Asset.Response.Failed Closed));
    let documents = t.documents in
    t.documents <- Int64.Map.empty;
    Map.iter documents ~f:(fun complete ->
      complete (Wire.Document.Response.Failed Closed))
;;

let submit_commands t =
  let rec loop remaining =
    if remaining > 0
    then (
      match Queue.peek t.commands with
      | None -> ()
      | Some message ->
        (match Gpuio_native.submit t.native message with
         | Ok () ->
           ignore (Queue.dequeue_exn t.commands : Wire.Message.t);
           loop (remaining - 1)
         | Error Busy -> ()
         | Error code -> Error.raise (native_error code)))
  in
  if t.welcomed
  then (
    let ready =
      match t.motion with
      | None -> true
      | Some preference ->
        let preference =
          match preference with
          | System -> Wire.Animation.Preference.System
          | Reduce -> Reduce
          | Full -> Full
        in
        (match Gpuio_native.submit t.native (Set_motion preference) with
         | Ok () ->
           t.motion <- None;
           true
         | Error Busy -> false
         | Error code -> Error.raise (native_error code))
    in
    if ready then loop 64)
;;

let step t =
  t.stats <- { t.stats with turns = t.stats.turns + 1 };
  let events = Gpuio_native.drain t.native |> Or_error.ok_exn in
  if
    List.exists events ~f:(function
      | Wire.Event.Stopped -> true
      | _ -> false)
  then t.stopping <- true;
  let now = t.now () in
  Map.iter t.windows ~f:(fun window ->
    Option.iter window.driver ~f:(fun driver -> Driver.advance_clock driver ~now));
  List.iter events ~f:(process t);
  let jobs = Inbox.take_turn t.inbox in
  if not t.stopped
  then (
    List.iter jobs ~f:(fun job -> job ());
    t.stats <- { t.stats with completed_jobs = t.stats.completed_jobs + List.length jobs };
    (* Disposal can trigger user lifecycle effects. Defer it until after the
     current callback/driver operation has returned, never re-enter a driver. *)
    Map.iter t.windows ~f:(fun window ->
      match window.phase with
      | Closing_before_open | Closing ->
        let driver = window.driver in
        window.driver <- None;
        Option.iter driver ~f:Driver.close
      | Opening | Open | Closed -> ());
    if t.welcomed && not t.stopping
    then
      Option.iter (Asset_registry.next_request t.asset_registry) ~f:(fun request ->
        Bonsai.Effect.Expert.handle
          (Bonsai.Effect.map
             (Expert.asset_request t ~limit:64 request)
             ~f:(Asset_registry.complete t.asset_registry)));
    if t.welcomed && not t.stopping
    then
      Option.iter (Document_registry.next_request t.document_registry) ~f:(fun request ->
        Bonsai.Effect.Expert.handle
          (Bonsai.Effect.map
             (Expert.document_request t ~limit:64 request)
             ~f:(Document_registry.complete t.document_registry)));
    submit_commands t;
    if not t.stopping
    then (
      let now = t.now () in
      Map.iter t.windows ~f:(fun window ->
        match window.phase, window.driver with
        | Open, Some driver when not t.stopping ->
          Driver.cycle driver ~now |> Or_error.ok_exn;
          if (not t.stopping) && not (Window.is_closed window)
          then
            Option.iter (Driver.next_message driver) ~f:(fun message ->
              match Gpuio_native.submit t.native message with
              | Ok () ->
                Driver.submitted driver;
                t.stats <- { t.stats with commits = t.stats.commits + 1 }
              | Error Busy -> ()
              | Error code -> Error.raise (native_error code))
        | (Opening | Closing_before_open | Closing | Closed), _ | Open, _ -> ())))
;;

let worker native read ~tick_hz ~max_tasks ~motion initialize =
  Eio_main.run (fun env ->
    Eio.Switch.run (fun sw ->
      let inbox = Inbox.create ~capacity:1024 () in
      let scope = Scope.Expert.create ~sw ~inbox ~max_tasks in
      let now () =
        Time_ns.of_span_since_epoch
          (Time_ns.Span.of_sec (Eio.Time.now (Eio.Stdenv.clock env)))
      in
      let app =
        { guard = Guard.create ()
        ; native
        ; inbox
        ; scope
        ; now
        ; windows = Int.Map.empty
        ; generations = Int.Map.empty
        ; commands = Queue.create ()
        ; opens = Int64.Map.empty
        ; closes = Int64.Map.empty
        ; frames = Int64.Map.empty
        ; editors = Int64.Map.empty
        ; dialogs = Int64.Map.empty
        ; window_requests = Int64.Map.empty
        ; window_capabilities = None
        ; quit_pending = None
        ; on_reopen = (fun () -> Bonsai.Effect.Ignore)
        ; asset_registry = Asset_registry.create ~scope ~wake:(fun () -> Inbox.wake inbox)
        ; document_registry =
            Document_registry.create ~scope ~wake:(fun () -> Inbox.wake inbox)
        ; assets = Int64.Map.empty
        ; documents = Int64.Map.empty
        ; correlation = 0L
        ; motion = Some motion
        ; welcomed = false
        ; stopping = false
        ; stopped = false
        ; stats =
            { turns = 0; clock_ticks = 0; commits = 0; rendered = 0; completed_jobs = 0 }
        }
      in
      Exn.protect
        ~finally:(fun () ->
          Asset_registry.close app.asset_registry;
          Document_registry.close app.document_registry;
          Scope.cancel scope;
          Map.iter app.windows ~f:release_window;
          app.assets <- Int64.Map.empty;
          app.documents <- Int64.Map.empty;
          app.frames <- Int64.Map.empty;
          app.closes <- Int64.Map.empty;
          app.opens <- Int64.Map.empty;
          Queue.clear app.commands;
          Inbox.close inbox)
        ~f:(fun () ->
          Eio.Fiber.fork_daemon ~sw (fun () ->
            let buffer = Cstruct.create 4096 in
            while not app.stopped do
              ignore (Eio.Flow.single_read read buffer : int);
              Inbox.wake inbox
            done;
            `Stop_daemon);
          Eio.Fiber.fork_daemon ~sw (fun () ->
            let clock = Eio.Stdenv.mono_clock env in
            let period = Mtime.Span.of_float_ns (1e9 /. tick_hz) |> Option.value_exn in
            let next time = Mtime.add_span time period |> Option.value_exn in
            let rec tick deadline =
              if app.stopped
              then `Stop_daemon
              else (
                Eio.Time.Mono.sleep_until clock deadline;
                app.stats <- { app.stats with clock_ticks = app.stats.clock_ticks + 1 };
                Inbox.wake inbox;
                let candidate = next deadline
                and now = Eio.Time.Mono.now clock in
                tick (if Mtime.compare candidate now <= 0 then next now else candidate))
            in
            tick (next (Eio.Time.Mono.now clock)));
          Gpuio_native.submit native (Hello (Wire.version, Wire.capabilities))
          |> Result.map_error ~f:native_error
          |> Or_error.ok_exn;
          initialize (env :> Eio_unix.Stdenv.base) app;
          while not app.stopped do
            step app;
            if not app.stopped then Inbox.await inbox
          done)))
;;

let capture f =
  try Ok (f ()) with
  | exn -> Error (exn, Stdlib.Printexc.get_raw_backtrace ())
;;

let reraise_result = function
  | Ok value -> value
  | Error (exn, bt) -> Stdlib.Printexc.raise_with_backtrace exn bt
;;

let run
      ?(tick_hz = 60.)
      ?(max_tasks = 1024)
      ?(exit_on_last_window = true)
      ?(motion = Gpuio.Animation.Preference.System)
      initialize
  =
  if (not (Float.is_finite tick_hz)) || Float.(tick_hz < 0.01 || tick_hz > 240.)
  then invalid_arg "tick_hz must be in [0.01,240]";
  if max_tasks < 1 || max_tasks > 65536 then invalid_arg "max_tasks must be in 1..65536";
  if not (Stdlib.Domain.is_main_domain ())
  then invalid_arg "GPUIO App.run must run on the main domain";
  Eio_main.run (fun _ ->
    Eio.Switch.run (fun sw ->
      let read, write = Eio_unix.pipe sw in
      let native =
        Eio_unix.Fd.use_exn
          "GPUIO runtime"
          (Eio_unix.Resource.fd write)
          (Gpuio_native.create_with_options ~exit_on_last_window)
      in
      Eio.Flow.close write;
      let domain =
        Domain.spawn (fun () ->
          try worker native read ~tick_hz ~max_tasks ~motion initialize with
          | exn ->
            let bt = Stdlib.Printexc.get_raw_backtrace () in
            Gpuio_native.abort native;
            Stdlib.Printexc.raise_with_backtrace exn bt)
      in
      let native_result = capture (fun () -> Gpuio_native.run native) in
      if Result.is_error native_result then Gpuio_native.abort native;
      let worker_result = capture (fun () -> Domain.join domain) in
      Gpuio_native.dispose native;
      reraise_result worker_result;
      reraise_result native_result))
;;
