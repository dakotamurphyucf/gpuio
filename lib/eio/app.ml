open Core
open Gpuio_protocol
module Guard = Gpuio_runtime_core.Domain_guard
module Driver = Gpuio_runtime_core.Window_driver
module Input = Gpuio.Text_input
module Palette = Gpuio.Command_palette
module Slider = Gpuio.Slider
module Number_input = Gpuio.Number_input
module Otp_input = Gpuio.Otp_input
module Calendar = Gpuio.Calendar
module Color_input = Gpuio.Color_input
module Dialog = Gpuio.File_dialog
module Native_window = Gpuio.Window

type editor_request =
  { window : Window_id.t
  ; node : Node_id.t
  ; complete : Wire.Editor.Result.t -> unit
  }

type palette_request =
  { expected : Palette.Snapshot.t
  ; complete : (Palette.Snapshot.t, Palette.Command_error.t) Result.t -> unit
  }

type slider_request =
  { window : Window_id.t
  ; node : Node_id.t
  ; complete : (Slider.Snapshot.t, Slider.Command_error.t) Result.t -> unit
  }

type number_input_request =
  { window : Window_id.t
  ; node : Node_id.t
  ; complete : (Number_input.Snapshot.t, Number_input.Command_error.t) Result.t -> unit
  }

type otp_input_request =
  { window : Window_id.t
  ; node : Node_id.t
  ; policy : Otp_input.Policy.t
  ; minimum_revision : Otp_input.Revision.t
  ; complete : (Otp_input.Snapshot.t, Otp_input.Command_error.t) Result.t -> unit
  }

type calendar_request =
  { window : Window_id.t
  ; node : Node_id.t
  ; mode : Calendar.Mode.t
  ; minimum_revision : Calendar.Revision.t
  ; complete : (Calendar.Snapshot.t, Calendar.Command_error.t) Result.t -> unit
  }

type color_input_request =
  { window : Window_id.t
  ; node : Node_id.t
  ; minimum_revision : Color_input.Revision.t
  ; complete : (Color_input.Snapshot.t, Color_input.Command_error.t) Result.t -> unit
  }

type window_result = Wire.Window.Response.t

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
  ; document_defaults : unit Bonsai.Effect.t Gpuio.Document.Defaults.t
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
  ; mutable palettes : palette_request Int64.Map.t
  ; mutable sliders : slider_request Int64.Map.t
  ; mutable number_inputs : number_input_request Int64.Map.t
  ; mutable otp_inputs : otp_input_request Int64.Map.t
  ; mutable calendars : calendar_request Int64.Map.t
  ; mutable color_inputs : color_input_request Int64.Map.t
  ; mutable dialogs : dialog_request Int64.Map.t
  ; mutable window_requests : window_request Int64.Map.t
  ; mutable window_capabilities : Native_window.Capabilities.t option
  ; mutable quit_pending : int64 option
  ; mutable on_reopen : unit -> unit Bonsai.Effect.t
  ; asset_registry : Asset_registry.t
  ; document_registry : Document_registry.t
  ; chart_registry : Chart_registry.t
  ; canvas_registry : Canvas_registry.t
  ; mutable assets : (Wire.Asset.Response.t -> unit) Int64.Map.t
  ; mutable documents : (Wire.Document.Response.t -> unit) Int64.Map.t
  ; mutable charts : (Wire.Chart.Response.t -> unit) Int64.Map.t
  ; mutable canvases : (Wire.Canvas.Response.t -> unit) Int64.Map.t
  ; mutable desktop_requests : (Wire.Desktop.Response.t -> unit) Int64.Map.t
  ; mutable desktop_pending : bool
  ; mutable desktop_subscription : int64 option
  ; mutable on_desktop_pending : unit -> unit Bonsai.Effect.t
  ; mutable notification_requests : (Wire.Notification.Response.t -> unit) Int64.Map.t
  ; mutable notification_pending : bool
  ; mutable notification_subscription : int64 option
  ; mutable on_notification_pending : unit -> unit Bonsai.Effect.t
  ; mutable notification_closed : bool
  ; desktop_identity : Gpuio.Desktop.Identity.t option
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

let desktop_identity t =
  check t;
  t.desktop_identity
;;

let scope t =
  check t;
  t.scope
;;

let stats t =
  check t;
  t.stats
;;

module Diagnostics = struct
  type t =
    { runtime : Stats.t
    ; traffic : Gpuio_native.Traffic.t
    ; native_command_queue : Gpuio_native.Command_queue.t
    ; scopes : Scope.Stats.t
    ; windows : int
    ; queued_jobs : int
    ; queued_commands : int
    ; pending_requests : int
    ; assets : int
    ; asset_uploads : int
    ; asset_source_bytes : int
    ; documents : int
    ; document_source_bytes : int
    ; charts : int
    ; chart_data_bytes : int
    ; canvases : int
    ; canvas_scene_bytes : int
    }
  [@@deriving sexp_of]
end

let diagnostics t : Diagnostics.t =
  check t;
  let assets, asset_uploads, asset_source_bytes =
    Asset_registry.Expert.counts t.asset_registry
  in
  let documents, document_source_bytes =
    Document_registry.Expert.counts t.document_registry
  in
  let charts, chart_data_bytes = Chart_registry.Expert.counts t.chart_registry in
  let canvases, canvas_scene_bytes = Canvas_registry.Expert.counts t.canvas_registry in
  { runtime = t.stats
  ; traffic = Gpuio_native.traffic t.native
  ; native_command_queue = Gpuio_native.command_queue t.native
  ; scopes = Scope.stats t.scope
  ; windows =
      Map.count t.windows ~f:(fun window ->
        match window.phase with
        | Closed -> false
        | Opening | Open | Closing_before_open | Closing -> true)
  ; queued_jobs = Inbox.length t.inbox
  ; queued_commands = Queue.length t.commands
  ; pending_requests =
      Map.length t.opens
      + Map.length t.closes
      + Map.length t.frames
      + Map.length t.editors
      + Map.length t.palettes
      + Map.length t.sliders
      + Map.length t.number_inputs
      + Map.length t.otp_inputs
      + Map.length t.calendars
      + Map.length t.color_inputs
      + Map.length t.dialogs
      + Map.length t.window_requests
      + Map.length t.assets
      + Map.length t.documents
      + Map.length t.charts
      + Map.length t.canvases
      + Map.length t.desktop_requests
      + Map.length t.notification_requests
  ; assets
  ; asset_uploads
  ; asset_source_bytes
  ; documents
  ; document_source_bytes
  ; charts
  ; chart_data_bytes
  ; canvases
  ; canvas_scene_bytes
  }
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
  let desktop t request =
    Bonsai.Effect.Expert.of_fun ~f:(fun ~callback ->
      check t;
      let fail error = callback (Wire.Desktop.Response.Failed error) in
      if t.stopping
      then fail Closed
      else if not (Wire.Desktop.Request.valid request)
      then fail Invalid_request
      else if Map.length t.desktop_requests >= 16
      then fail Busy
      else (
        let id = correlation t in
        t.desktop_requests <- Map.set t.desktop_requests ~key:id ~data:callback;
        queue t (Desktop (id, request))))
  ;;

  let on_desktop_pending t callback =
    check t;
    if t.stopping
    then Error Gpuio.Desktop.Error.Closed
    else if Option.is_some t.desktop_subscription
    then Error Gpuio.Desktop.Error.Busy
    else (
      let token = correlation t in
      let current () = Option.equal Int64.equal t.desktop_subscription (Some token) in
      t.desktop_subscription <- Some token;
      t.on_desktop_pending <- callback;
      if t.desktop_pending
      then
        Scope.Expert.enqueue t.scope (fun () ->
          if (not t.stopping) && current ()
          then Bonsai.Effect.Expert.handle (t.on_desktop_pending ()));
      Ok
        (fun () ->
          check t;
          if current ()
          then (
            t.desktop_subscription <- None;
            t.on_desktop_pending <- (fun () -> Bonsai.Effect.Ignore))))
  ;;

  let notification t request =
    Bonsai.Effect.Expert.of_fun ~f:(fun ~callback ->
      check t;
      let fail error = callback (Wire.Notification.Response.Failed error) in
      if t.stopping || t.notification_closed
      then fail Closed
      else if not (Wire.Notification.Request.valid request)
      then fail Invalid_request
      else if
        Map.length t.notification_requests >= 16
        && not (Wire.Notification.Request.equal request Close)
      then fail Busy
      else (
        if Wire.Notification.Request.equal request Close
        then t.notification_closed <- true;
        let id = correlation t in
        t.notification_requests <- Map.set t.notification_requests ~key:id ~data:callback;
        queue t (Notification (id, request))))
  ;;

  let on_notification_pending t callback =
    check t;
    if t.stopping || t.notification_closed
    then Error Gpuio.Notification.Error.Closed
    else if Option.is_some t.notification_subscription
    then Error Gpuio.Notification.Error.Busy
    else (
      let token = correlation t in
      let current () =
        Option.equal Int64.equal t.notification_subscription (Some token)
      in
      t.notification_subscription <- Some token;
      t.on_notification_pending <- callback;
      if t.notification_pending
      then
        Scope.Expert.enqueue t.scope (fun () ->
          if (not t.stopping) && current ()
          then Bonsai.Effect.Expert.handle (t.on_notification_pending ()));
      Ok
        (fun () ->
          check t;
          if current ()
          then (
            t.notification_subscription <- None;
            t.on_notification_pending <- (fun () -> Bonsai.Effect.Ignore))))
  ;;

  let chart_request t ~limit request =
    Bonsai.Effect.Expert.of_fun ~f:(fun ~callback ->
      check t;
      let fail error = callback (Wire.Chart.Response.Failed error) in
      if t.stopping
      then fail Closed
      else if not t.welcomed
      then fail Not_ready
      else if Map.length t.charts >= limit
      then fail Resource_limit
      else (
        let oversized =
          match request with
          | Wire.Chart.Request.Chunk (_, _, _, data) ->
            String.length data > Wire.Chart.max_chunk_bytes
          | Create | Begin _ | Publish _ | Abort _ | Release _ -> false
        in
        if oversized
        then fail Invalid_range
        else (
          let id = correlation t in
          t.charts <- Map.set t.charts ~key:id ~data:callback;
          queue t (Chart (id, request)))))
  ;;

  let chart t request = chart_request t ~limit:63 request

  let canvas_request t ~limit request =
    Bonsai.Effect.Expert.of_fun ~f:(fun ~callback ->
      check t;
      let fail error = callback (Wire.Canvas.Response.Failed error) in
      if t.stopping
      then fail Closed
      else if not t.welcomed
      then fail Not_ready
      else if Map.length t.canvases >= limit
      then fail Resource_limit
      else (
        let oversized =
          match request with
          | Wire.Canvas.Request.Chunk (_, _, _, data) ->
            String.length data > Wire.Canvas.max_chunk_bytes
          | Create | Begin _ | Publish _ | Abort _ | Release _ -> false
        in
        if oversized
        then fail Invalid_range
        else (
          let id = correlation t in
          t.canvases <- Map.set t.canvases ~key:id ~data:callback;
          queue t (Canvas (id, request)))))
  ;;

  let canvas t request = canvas_request t ~limit:63 request

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

  let register_chart t ~scope data =
    Bonsai.Effect.Expert.of_fun ~f:(fun ~callback ->
      check t;
      if t.stopping
      then callback (Error (Chart_registry.Error.Native Closed))
      else Chart_registry.register t.chart_registry ~scope data ~on_result:callback)
  ;;

  let register_canvas t ~scope scene =
    Bonsai.Effect.Expert.of_fun ~f:(fun ~callback ->
      check t;
      if t.stopping
      then callback (Error (Canvas_registry.Error.Native Closed))
      else Canvas_registry.register t.canvas_registry ~scope scene ~on_result:callback)
  ;;

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

(* User completion/lifecycle effects can raise during teardown. Finish every
   independent cleanup, then propagate the first failure with its backtrace. *)
let finish_cleanup f =
  let failure = ref None in
  let attempt f =
    try f () with
    | exn ->
      let backtrace = Stdlib.Printexc.get_raw_backtrace () in
      if Option.is_none !failure then failure := Some (exn, backtrace)
  in
  f attempt;
  match !failure with
  | None -> ()
  | Some (exn, backtrace) -> Stdlib.Printexc.raise_with_backtrace exn backtrace
;;

let complete_close window decision =
  let waiters = window.close_waiters in
  window.close_waiters <- [];
  window.close_pending <- None;
  window.close_requested <- false;
  finish_cleanup (fun attempt ->
    List.iter waiters ~f:(fun (_, complete) -> attempt (fun () -> complete decision)))
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
    finish_cleanup (fun attempt ->
      attempt (fun () -> complete_close window Allow);
      let cancelled_windows, remaining_windows =
        Map.partition_tf window.app.window_requests ~f:(fun request ->
          Window_id.equal request.window window.id)
      in
      window.app.window_requests <- remaining_windows;
      Map.iter cancelled_windows ~f:(fun request ->
        attempt (fun () ->
          request.complete (Wire.Window.Response.Failed Native_window.Error.Closed)));
      let cancelled, remaining =
        Map.partition_tf window.app.editors ~f:(fun request ->
          Window_id.equal request.window window.id)
      in
      window.app.editors <- remaining;
      let cancelled_sliders, remaining_sliders =
        Map.partition_tf window.app.sliders ~f:(fun request ->
          Window_id.equal request.window window.id)
      in
      window.app.sliders <- remaining_sliders;
      Map.iter cancelled_sliders ~f:(fun request ->
        attempt (fun () -> request.complete (Error Closed)));
      let cancelled_number_inputs, remaining_number_inputs =
        Map.partition_tf window.app.number_inputs ~f:(fun request ->
          Window_id.equal request.window window.id)
      in
      window.app.number_inputs <- remaining_number_inputs;
      Map.iter cancelled_number_inputs ~f:(fun request ->
        attempt (fun () -> request.complete (Error Closed)));
      let cancelled_otp_inputs, remaining_otp_inputs =
        Map.partition_tf window.app.otp_inputs ~f:(fun request ->
          Window_id.equal request.window window.id)
      in
      window.app.otp_inputs <- remaining_otp_inputs;
      Map.iter cancelled_otp_inputs ~f:(fun request ->
        attempt (fun () -> request.complete (Error Closed)));
      let cancelled_calendars, remaining_calendars =
        Map.partition_tf window.app.calendars ~f:(fun request ->
          Window_id.equal request.window window.id)
      in
      window.app.calendars <- remaining_calendars;
      Map.iter cancelled_calendars ~f:(fun request ->
        attempt (fun () -> request.complete (Error Closed)));
      let cancelled_palettes, remaining_palettes =
        Map.partition_tf window.app.palettes ~f:(fun request ->
          Window_id.equal (Palette.Expert.window request.expected) window.id)
      in
      window.app.palettes <- remaining_palettes;
      Map.iter cancelled_palettes ~f:(fun request ->
        attempt (fun () -> request.complete (Error Closed)));
      let cancelled_color_inputs, remaining_color_inputs =
        Map.partition_tf window.app.color_inputs ~f:(fun request ->
          Window_id.equal request.window window.id)
      in
      window.app.color_inputs <- remaining_color_inputs;
      Map.iter cancelled_color_inputs ~f:(fun request ->
        attempt (fun () -> request.complete (Error Closed)));
      Map.iter cancelled ~f:(fun request ->
        attempt (fun () -> request.complete (Wire.Editor.Result.Failed Closed)));
      let cancelled_dialogs, remaining_dialogs =
        Map.partition_tf window.app.dialogs ~f:(fun request ->
          Window_id.equal request.window window.id)
      in
      window.app.dialogs <- remaining_dialogs;
      Map.iter cancelled_dialogs ~f:(fun request ->
        attempt (fun () -> request.complete (Wire.File_dialog.Result.Failed Closed)));
      attempt (fun () -> Scope.cancel window.scope);
      let driver = window.driver in
      window.driver <- None;
      Option.iter driver ~f:(fun driver -> attempt (fun () -> Driver.close driver));
      window.on_change <- (fun _ -> Bonsai.Effect.Ignore);
      window.on_close
      <- (fun _ -> Bonsai.Effect.return Native_window.Close_decision.Allow);
      window.app.frames
      <- Map.filter window.app.frames ~f:(fun (id, _) ->
           not (Window_id.equal id window.id));
      window.app.windows
      <- Map.remove window.app.windows (Window_id.slot window.id |> Int64.to_int_exn))
;;

let shutdown t =
  check t;
  if not t.stopping
  then (
    t.stopping <- true;
    t.on_desktop_pending <- (fun () -> Bonsai.Effect.Ignore);
    t.desktop_subscription <- None;
    t.desktop_pending <- false;
    t.on_notification_pending <- (fun () -> Bonsai.Effect.Ignore);
    t.notification_subscription <- None;
    t.notification_pending <- false;
    t.notification_closed <- true;
    Asset_registry.close t.asset_registry;
    Document_registry.close t.document_registry;
    Chart_registry.close t.chart_registry;
    Canvas_registry.close t.canvas_registry;
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

  let window_request t command =
    Bonsai.Effect.Expert.of_fun ~f:(fun ~callback ->
      check t.app;
      let fail error = callback (Wire.Window.Response.Failed error) in
      if is_closed t || t.app.stopping
      then fail Native_window.Error.Closed
      else if Result.is_error (Wire.Window.Command.validate command)
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

  let command t command =
    let open Bonsai.Effect.Let_syntax in
    let%map response = window_request t (Native_window.Command.Expert.to_wire command) in
    match response with
    | Wire.Window.Response.Observed snapshot -> Ok snapshot
    | Failed error -> Error error
    | Focused_input _ | Selection_present _ | Selected_text _ | Selection_updated ->
      Error Native_window.Error.Native_failure
  ;;

  let focused_input t =
    let open Bonsai.Effect.Let_syntax in
    let%map response = window_request t Wire.Window.Command.Focused_input in
    match response with
    | Wire.Window.Response.Focused_input input ->
      Ok (Option.map input ~f:(Native_window.Input.Expert.of_wire ~window:t.id))
    | Failed error -> Error error
    | Observed _ | Selection_present _ | Selected_text _ | Selection_updated ->
      Error Native_window.Error.Native_failure
  ;;

  let has_text_selection t =
    let open Bonsai.Effect.Let_syntax in
    let%map response = window_request t Wire.Window.Command.Has_text_selection in
    match response with
    | Wire.Window.Response.Selection_present present -> Ok present
    | Failed error -> Error error
    | Observed _ | Focused_input _ | Selected_text _ | Selection_updated ->
      Error Native_window.Error.Native_failure
  ;;

  let selected_text t ?(max_bytes = Wire.Window.default_selection_bytes) () =
    let open Bonsai.Effect.Let_syntax in
    let%map response =
      window_request t (Wire.Window.Command.Selected_text (Int64.of_int max_bytes))
    in
    match response with
    | Wire.Window.Response.Selected_text text when String.length text <= max_bytes ->
      Ok text
    | Failed error -> Error error
    | Observed _
    | Focused_input _
    | Selection_present _
    | Selected_text _
    | Selection_updated -> Error Native_window.Error.Native_failure
  ;;

  let selection_action t command =
    let open Bonsai.Effect.Let_syntax in
    let%map response = window_request t command in
    match response with
    | Wire.Window.Response.Selection_updated -> Ok ()
    | Failed error -> Error error
    | Observed _ | Focused_input _ | Selection_present _ | Selected_text _ ->
      Error Native_window.Error.Native_failure
  ;;

  let clear_text_selection t = selection_action t Wire.Window.Command.Clear_text_selection
  let end_text_selection t = selection_action t Wire.Window.Command.End_text_selection

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

    let slider_command t snapshot command =
      Bonsai.Effect.Expert.of_fun ~f:(fun ~callback ->
        check t.app;
        if is_closed t || t.app.stopping
        then callback (Error Slider.Command_error.Closed)
        else if not (Window_id.equal t.id (Slider.Expert.window snapshot))
        then callback (Error Stale_slider)
        else if Map.length t.app.sliders >= 64
        then callback (Error Busy)
        else (
          let request = correlation t.app in
          let node = Slider.Expert.node snapshot in
          t.app.sliders
          <- Map.set
               t.app.sliders
               ~key:request
               ~data:{ window = t.id; node; complete = callback };
          queue
            t.app
            (Slider_command (request, t.id, node, Slider.Expert.command_to_wire command))))
    ;;

    let decline_number_step t request =
      check t.app;
      let expected = Number_input.Step_request.snapshot request in
      if
        (not (is_closed t))
        && (not t.app.stopping)
        && Window_id.equal t.id (Number_input.Expert.window expected)
      then
        if
          (* Cleanup has no callback or pending-map slot. The native reply is
           intentionally ignored; token/revision/generation guards make a late
           decline harmless. Saturated ordinary commands cannot strand it. *)
          Int64.equal t.app.correlation Int64.max_value
        then (
          t.app.stopping <- true;
          Inbox.wake t.app.inbox)
        else
          queue
            t.app
            (Number_input_command
               ( correlation t.app
               , t.id
               , Number_input.Expert.node expected
               , Number_input.Expert.command_to_wire (Resolve_step (request, Decline)) ))
    ;;

    let number_input_command t snapshot command =
      Bonsai.Effect.Expert.of_fun ~f:(fun ~callback ->
        check t.app;
        let wrong_owner =
          Option.exists (Number_input.Expert.command_snapshot command) ~f:(fun expected ->
            not
              (Window_id.equal
                 (Number_input.Expert.window expected)
                 (Number_input.Expert.window snapshot)
               && Node_id.equal
                    (Number_input.Expert.node expected)
                    (Number_input.Expert.node snapshot)))
        in
        let command = Number_input.Expert.command_to_wire command in
        let invalid : Number_input.Command_error.t option =
          let module W = Gpuio_protocol.Number_input_wire in
          match command with
          | W.Command.Replace_draft { text; selection; _ } ->
            if String.length text > Number_input.max_draft_bytes
            then Some Limit_exceeded
            else if not (W.valid_text text)
            then Some Invalid_text
            else if not (W.Selection_policy.within selection text)
            then Some Invalid_selection
            else None
          | Replace_value { value; selection; _ } ->
            if not (W.Value.valid value)
            then Some Invalid_value
            else if not (W.Selection_policy.valid selection)
            then Some Invalid_selection
            else None
          | Resolve_step { value; _ } ->
            if Option.for_all value ~f:W.Value.valid then None else Some Invalid_value
          | Select selection ->
            if W.Selection.valid selection then None else Some Invalid_selection
          | Focus | Undo | Redo | Commit | Cancel | Step _ | Read_snapshot -> None
        in
        if is_closed t || t.app.stopping
        then callback (Error Number_input.Command_error.Closed)
        else if wrong_owner
        then callback (Error Number_input.Command_error.Stale_input)
        else if Option.is_some invalid
        then callback (Error (Option.value_exn invalid))
        else if not (Window_id.equal t.id (Number_input.Expert.window snapshot))
        then callback (Error Stale_input)
        else if Map.length t.app.number_inputs >= 64
        then callback (Error Busy)
        else (
          let request = correlation t.app in
          let node = Number_input.Expert.node snapshot in
          t.app.number_inputs
          <- Map.set
               t.app.number_inputs
               ~key:request
               ~data:{ window = t.id; node; complete = callback };
          queue t.app (Number_input_command (request, t.id, node, command))))
    ;;

    let otp_input_command t snapshot command =
      Bonsai.Effect.Expert.of_fun ~f:(fun ~callback ->
        check t.app;
        let command = Otp_input.Expert.command_to_wire command in
        let invalid : Otp_input.Command_error.t option =
          let module W = Gpuio_protocol.Otp_wire in
          match command with
          | W.Command.Replace { value; selection; _ } ->
            if not (W.Selection_policy.within selection value)
            then Some Invalid_selection
            else None
          | Select selection ->
            if W.Selection.valid selection then None else Some Invalid_selection
          | Clear _ | Focus | Undo | Redo | Cancel_composition | Read_snapshot -> None
        in
        if is_closed t || t.app.stopping
        then callback (Error Otp_input.Command_error.Closed)
        else if Option.is_some invalid
        then callback (Error (Option.value_exn invalid))
        else if not (Window_id.equal t.id (Otp_input.Expert.window snapshot))
        then callback (Error Stale_input)
        else if Map.length t.app.otp_inputs >= 64
        then callback (Error Busy)
        else (
          let request = correlation t.app in
          let node = Otp_input.Expert.node snapshot in
          t.app.otp_inputs
          <- Map.set
               t.app.otp_inputs
               ~key:request
               ~data:
                 { window = t.id
                 ; node
                 ; policy = Otp_input.Snapshot.policy snapshot
                 ; minimum_revision = Otp_input.Snapshot.revision snapshot
                 ; complete = callback
                 };
          queue t.app (Otp_input_command (request, t.id, node, command))))
    ;;

    let calendar_command t snapshot command =
      Bonsai.Effect.Expert.of_fun ~f:(fun ~callback ->
        check t.app;
        if is_closed t || t.app.stopping
        then callback (Error Calendar.Command_error.Closed)
        else if not (Window_id.equal t.id (Calendar.Expert.window snapshot))
        then callback (Error Stale_input)
        else (
          match Calendar.Expert.command_to_wire command with
          | Error _ -> callback (Error Invalid_value)
          | Ok command ->
            if Map.length t.app.calendars >= 64
            then callback (Error Busy)
            else (
              let request = correlation t.app in
              let node = Calendar.Expert.node snapshot in
              t.app.calendars
              <- Map.set
                   t.app.calendars
                   ~key:request
                   ~data:
                     { window = t.id
                     ; node
                     ; mode = Calendar.Snapshot.mode snapshot
                     ; minimum_revision = Calendar.Snapshot.revision snapshot
                     ; complete = callback
                     };
              queue t.app (Calendar_command (request, t.id, node, command)))))
    ;;

    let color_input_command t snapshot command =
      Bonsai.Effect.Expert.of_fun ~f:(fun ~callback ->
        check t.app;
        if is_closed t || t.app.stopping
        then callback (Error Color_input.Command_error.Closed)
        else if not (Window_id.equal t.id (Color_input.Expert.window snapshot))
        then callback (Error Stale_color_input)
        else if Map.length t.app.color_inputs >= 64
        then callback (Error Busy)
        else (
          let command = Color_input.Expert.command_to_wire command in
          let request = correlation t.app in
          let node = Color_input.Expert.node snapshot in
          t.app.color_inputs
          <- Map.set
               t.app.color_inputs
               ~key:request
               ~data:
                 { window = t.id
                 ; node
                 ; minimum_revision = Color_input.Snapshot.revision snapshot
                 ; complete = callback
                 };
          queue t.app (Color_input_command (request, t.id, node, command))))
    ;;

    let palette_command t snapshot ?(if_query_unchanged = false) command =
      Bonsai.Effect.Expert.of_fun ~f:(fun ~callback ->
        check t.app;
        let command = Palette.Expert.command_to_wire command in
        if is_closed t || t.app.stopping
        then callback (Error Palette.Command_error.Closed)
        else if not (Window_id.equal t.id (Palette.Expert.window snapshot))
        then callback (Error Stale_palette)
        else if not (Palette_command_wire.Command.valid command)
        then callback (Error Invalid_query)
        else if Map.length t.app.palettes >= 64
        then callback (Error Busy)
        else (
          let request = correlation t.app in
          t.app.palettes
          <- Map.set
               t.app.palettes
               ~key:request
               ~data:{ expected = snapshot; complete = callback };
          queue
            t.app
            (Palette_command
               ( request
               , t.id
               , Palette.Expert.node snapshot
               , Palette.Expert.observer snapshot
               , (if if_query_unchanged
                  then Some (Palette.Expert.query_revision snapshot)
                  else None)
               , command ))))
    ;;

    let request_editor t snapshot ?invalid_text command =
      Bonsai.Effect.Expert.of_fun ~f:(fun ~callback:complete ->
        check t.app;
        if is_closed t || t.app.stopping
        then complete (Wire.Editor.Result.Failed Closed)
        else if Option.is_some invalid_text
        then complete (Failed (Option.value_exn invalid_text))
        else if not (Window_id.equal t.id (Input.Expert.window snapshot))
        then complete (Failed Stale_editor)
        else if Map.length t.app.editors >= 64
        then complete (Failed Busy)
        else (
          let request = correlation t.app in
          let node = Input.Expert.node snapshot in
          t.app.editors
          <- Map.set t.app.editors ~key:request ~data:{ window = t.id; node; complete };
          queue t.app (Editor_command (request, t.id, node, command))))
    ;;

    let editor_command t snapshot command =
      let invalid_text : Wire.Editor.Error.t option =
        match command with
        | Input.Command.Replace { text; _ } ->
          if String.length text > Input.max_text_bytes
          then Some Limit_exceeded
          else if Result.is_error (Input.validate_text ~mode:Multiline text)
          then Some Invalid_text
          else None
        | Select _ | Focus | Undo | Redo | Submit | Read_snapshot -> None
      in
      Bonsai.Effect.map
        (request_editor t snapshot ?invalid_text (Input.Expert.command_to_wire command))
        ~f:(function
          | Failed error -> Error (Input.Expert.error_of_wire error)
          | Applied value ->
            Input.Expert.snapshot_of_wire
              ~window:t.id
              ~node:(Input.Expert.node snapshot)
              value
            |> Result.map_error ~f:(fun _ -> Input.Command_error.Native_failure)
          | Content_hint_status _
          | Viewport _
          | Viewport_scroll_accepted
          | Search_observed _
          | Search_replaced _
          | Range_bounds _ -> Error Input.Command_error.Native_failure)
    ;;

    let editor_content_hint_status t snapshot =
      Bonsai.Effect.map (request_editor t snapshot Read_content_hint_status) ~f:(function
        | Failed error -> Error (Input.Expert.error_of_wire error)
        | Content_hint_status status ->
          Ok (Input.Content_hint.Status.Expert.of_wire status)
        | Applied _
        | Viewport _
        | Viewport_scroll_accepted
        | Search_observed _
        | Search_replaced _
        | Range_bounds _ -> Error Input.Command_error.Native_failure)
    ;;

    let editor_range_bounds t snapshot range =
      let expected = Input.Revision.to_int64 (Input.Snapshot.revision snapshot) in
      let invalid_text =
        if
          Result.is_error
            (Input.Selection.validate range ~text:(Input.Snapshot.text snapshot))
        then Some Wire.Editor.Error.Invalid_selection
        else None
      in
      let range : Wire.Editor.Selection.t =
        { anchor = Int64.of_int (Input.Selection.anchor range)
        ; head = Int64.of_int (Input.Selection.head range)
        }
      in
      Bonsai.Effect.map
        (request_editor t snapshot ?invalid_text (Read_range_bounds (expected, range)))
        ~f:(function
          | Failed error -> Error (Input.Expert.error_of_wire error)
          | Range_bounds None -> Ok None
          | Range_bounds (Some geometry) ->
            if not (Int64.equal geometry.revision expected)
            then Error Input.Command_error.Native_failure
            else
              Gpuio.Editor_geometry.Expert.of_wire geometry
              |> Result.map ~f:Option.some
              |> Result.map_error ~f:(fun _ -> Input.Command_error.Native_failure)
          | Applied _
          | Content_hint_status _
          | Viewport _
          | Viewport_scroll_accepted
          | Search_observed _
          | Search_replaced _ -> Error Input.Command_error.Native_failure)
    ;;

    let editor_viewport t snapshot =
      Bonsai.Effect.map (request_editor t snapshot Read_viewport) ~f:(function
        | Failed error -> Error (Input.Expert.error_of_wire error)
        | Viewport viewport ->
          Option.value_map viewport ~default:(Ok None) ~f:(fun value ->
            Gpuio.Editor_viewport.Expert.of_wire value |> Result.map ~f:Option.some)
          |> Result.map_error ~f:(fun _ -> Input.Command_error.Native_failure)
        | Applied _
        | Content_hint_status _
        | Viewport_scroll_accepted
        | Search_observed _
        | Search_replaced _
        | Range_bounds _ -> Error Input.Command_error.Native_failure)
    ;;

    let editor_search t snapshot command =
      let module S = Input.Search in
      let invalid_text =
        match S.Expert.expected_stamp command with
        | Some stamp
          when (not (Window_id.equal t.id (S.Expert.stamp_window stamp)))
               || not
                    (Node_id.equal
                       (Input.Expert.node snapshot)
                       (S.Expert.stamp_node stamp)) -> Some Wire.Editor.Error.Stale_editor
        | Some _ | None ->
          (match command with
           | S.Command.Replace_current { replacement; _ } | Replace_all { replacement; _ }
             ->
             if String.length replacement > Input.max_text_bytes
             then Some Limit_exceeded
             else if Result.is_error (Input.validate_text ~mode:Multiline replacement)
             then Some Invalid_text
             else None
           | Read
           | Open _
           | Close
           | Close_and_focus _
           | Set_query _
           | Set_query_text _
           | Set_case _
           | Toggle_case
           | Next
           | Previous -> None)
      in
      let decode_search value =
        S.Expert.snapshot_of_wire ~window:t.id ~node:(Input.Expert.node snapshot) value
      in
      Bonsai.Effect.map
        (request_editor
           t
           snapshot
           ?invalid_text
           (Search (S.Expert.command_to_wire command)))
        ~f:(function
          | Failed error -> Error (Input.Expert.error_of_wire error)
          | Search_observed value ->
            (match command with
             | Replace_current _ | Replace_all _ ->
               Error Input.Command_error.Native_failure
             | Close_and_focus expected
               when (not (Gpuio_protocol.Editor_search_wire.Mode.equal value.mode Closed))
                    || not
                         (Int64.equal
                            value.activation_revision
                            (S.Snapshot.activation_revision expected)) ->
               Error Input.Command_error.Native_failure
             | Read
             | Open _
             | Close
             | Close_and_focus _
             | Set_query _
             | Set_query_text _
             | Set_case _
             | Toggle_case
             | Next
             | Previous ->
               decode_search value
               |> Result.map ~f:(fun value -> S.Response.Observed value, None)
               |> Result.map_error ~f:(fun _ -> Input.Command_error.Native_failure))
          | Search_replaced (editor, value, count) ->
            (match command with
             | Read
             | Open _
             | Close
             | Close_and_focus _
             | Set_query _
             | Set_query_text _
             | Set_case _
             | Toggle_case
             | Next
             | Previous -> Error Input.Command_error.Native_failure
             | Replace_current _ | Replace_all _ ->
               let converted =
                 let open Or_error.Let_syntax in
                 let%bind () =
                   if
                     Int64.(count >= 0L && count <= 262_144L)
                     && Int64.equal editor.revision value.stamp.editor_revision
                     && Int64.equal
                          value.text_bytes
                          (Int64.of_int (String.length editor.text))
                   then Ok ()
                   else Or_error.error_string "inconsistent search replacement reply"
                 in
                 let%bind search = decode_search value in
                 let%map editor =
                   Input.Expert.snapshot_of_wire
                     ~window:t.id
                     ~node:(Input.Expert.node snapshot)
                     editor
                 in
                 ( S.Response.Replaced
                     { snapshot = search; count = Int64.to_int_exn count }
                 , Some editor )
               in
               Result.map_error converted ~f:(fun _ -> Input.Command_error.Native_failure))
          | Applied _
          | Content_hint_status _
          | Viewport _
          | Viewport_scroll_accepted
          | Range_bounds _ -> Error Input.Command_error.Native_failure)
    ;;

    let editor_scroll_to t snapshot offset =
      Bonsai.Effect.map
        (request_editor
           t
           snapshot
           (Scroll_viewport (Gpuio.Editor_viewport.Expert.offset_to_wire offset)))
        ~f:(function
          | Failed error -> Error (Input.Expert.error_of_wire error)
          | Viewport_scroll_accepted -> Ok ()
          | Applied _
          | Content_hint_status _
          | Viewport _
          | Search_observed _
          | Search_replaced _
          | Range_bounds _ -> Error Input.Command_error.Native_failure)
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
  let { Wire.Window.Config.title
      ; width
      ; height
      ; focus = _
      ; chrome = _
      ; resizable = _
      ; frame = _
      }
    =
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
            ~document_defaults:t.document_defaults
            ~asset_owner:(Asset_registry.Expert.owner t.asset_registry)
            ~document_owner:(Document_registry.Expert.owner t.document_registry)
            ~canvas_owner:(Canvas_registry.Expert.owner t.canvas_registry)
            ~chart_owner:(Chart_registry.Expert.owner t.chart_registry)
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
      ?frame
      ~title
      ~width
      ~height
      component
  =
  Result.bind
    (Native_window.Config.create
       ~focus
       ~chrome
       ~resizable
       ?frame
       ~title
       ~width
       ~height
       ())
    ~f:(fun config -> open_window_config t ?theme config component)
;;

let find_window t id =
  Map.find t.windows (Window_id.slot id |> Int64.to_int_exn)
  |> Option.filter ~f:(fun window -> Window_id.equal window.id id)
;;

let native_error code = Error.create_s (Wire.Error_code.sexp_of_t code)

let process t = function
  | Wire.Event.Desktop_pending ->
    if not t.stopping
    then (
      t.desktop_pending <- true;
      let token = t.desktop_subscription in
      enqueue t (fun () ->
        if
          (not t.stopping)
          && Option.is_some token
          && Option.equal Int64.equal token t.desktop_subscription
        then Bonsai.Effect.Expert.handle (t.on_desktop_pending ())))
  | Desktop_response (request, response) ->
    (match Map.find t.desktop_requests request with
     | None -> ()
     | Some complete ->
       t.desktop_requests <- Map.remove t.desktop_requests request;
       (match response with
        | Links _ -> t.desktop_pending <- false
        | Configured
        | Capabilities _
        | Requested
        | Registered
        | Failed _
        | Scrollbar_preference _ -> ());
       complete (if t.stopping then Wire.Desktop.Response.Failed Closed else response))
  | Wire.Event.Notification_pending ->
    if (not t.stopping) && not t.notification_closed
    then (
      t.notification_pending <- true;
      let token = t.notification_subscription in
      enqueue t (fun () ->
        if
          (not t.stopping)
          && (not t.notification_closed)
          && Option.is_some token
          && Option.equal Int64.equal token t.notification_subscription
        then Bonsai.Effect.Expert.handle (t.on_notification_pending ())))
  | Notification_response (request, response) ->
    (match Map.find t.notification_requests request with
     | None -> ()
     | Some complete ->
       t.notification_requests <- Map.remove t.notification_requests request;
       (match response with
        | Events _ -> t.notification_pending <- false
        | Capabilities _
        | Authorization _
        | Posted _
        | Replaced
        | Dismiss_requested
        | Closed
        | Failed _ -> ());
       complete
         (if
            t.stopping
            || (t.notification_closed
                && not (Wire.Notification.Response.equal response Closed))
          then Wire.Notification.Response.Failed Closed
          else response))
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
       pending.complete response
     | Some _ | None -> ())
  | Wire.Event.Welcome (protocol_version, available_capabilities) ->
    Wire.validate_welcome ~protocol_version ~available_capabilities |> Or_error.ok_exn;
    t.welcomed <- true
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
  | Canvas_event (id, _, _, _, source, scene_revision, scene_generation, observation) as
    event ->
    if
      Canvas_registry.accepts_event
        t.canvas_registry
        source
        ~scene_revision
        ~scene_generation
        observation
    then
      Option.iter (find_window t id) ~f:(fun window ->
        if not (Window.is_closed window)
        then Option.iter window.driver ~f:(fun driver -> Driver.dispatch driver event))
  | Chart_event (id, _, _, _, source, data_revision, data_generation, observation) as
    event ->
    if
      Chart_registry.accepts_event
        t.chart_registry
        source
        ~data_revision
        ~data_generation
        observation
    then
      Option.iter (find_window t id) ~f:(fun window ->
        if not (Window.is_closed window)
        then Option.iter window.driver ~f:(fun driver -> Driver.dispatch driver event))
  | Document_profile_event (id, _, _, _, source, observation) as event ->
    if
      Document_registry.accepts_event
        t.document_registry
        source
        ~generation:observation.source_generation
        ~revision:observation.source_revision
    then
      Option.iter (find_window t id) ~f:(fun window ->
        if not (Window.is_closed window)
        then Option.iter window.driver ~f:(fun driver -> Driver.dispatch driver event))
  | Document_action (id, _, _, _, source, observation) as event ->
    if
      Document_registry.accepts_event
        t.document_registry
        source
        ~generation:observation.source_generation
        ~revision:observation.source_revision
    then
      Option.iter (find_window t id) ~f:(fun window ->
        if not (Window.is_closed window)
        then Option.iter window.driver ~f:(fun driver -> Driver.dispatch driver event))
  | Document_preview_observed (id, _, _, _, source, observation) as event ->
    if
      Document_registry.accepts_event
        t.document_registry
        source
        ~generation:observation.source_generation
        ~revision:observation.source_revision
    then
      Option.iter (find_window t id) ~f:(fun window ->
        if not (Window.is_closed window)
        then Option.iter window.driver ~f:(fun driver -> Driver.dispatch driver event))
  | Document_diff_event (id, _, _, _, source, observation) as event ->
    if
      Document_registry.accepts_event
        t.document_registry
        source
        ~generation:observation.source_generation
        ~revision:observation.source_revision
    then
      Option.iter (find_window t id) ~f:(fun window ->
        if not (Window.is_closed window)
        then Option.iter window.driver ~f:(fun driver -> Driver.dispatch driver event))
  | Document_navigation (id, _, _, _, source, generation, _) as event ->
    if Document_registry.accepts_navigation t.document_registry source ~generation
    then
      Option.iter (find_window t id) ~f:(fun window ->
        if not (Window.is_closed window)
        then Option.iter window.driver ~f:(fun driver -> Driver.dispatch driver event))
  | ( Extension_event (id, _, _, _, _, _)
    | Split_resized (id, _, _, _, _, _)
    | Split_group_resized (id, _, _, _, _, _)
    | Press (id, _, _, _)
    | Editor_search_observed (id, _, _, _, _)
    | Editor_event (id, _, _, _, _, _)
    | Slider_event (id, _, _, _, _)
    | Number_input_event (id, _, _, _, _)
    | Otp_input_event (id, _, _, _, _)
    | Color_input_event (id, _, _, _, _)
    | Calendar_viewport_changed (id, _, _, _, _)
    | Calendar_event (id, _, _, _, _)
    | Rating_requested (id, _, _, _, _)
    | Table_input (id, _, _, _, _)
    | Table_columns_observed (id, _, _, _, _)
    | Tree_input (id, _, _, _, _)
    | List_input (id, _, _, _, _, _)
    | Carousel_requested (id, _, _, _, _)
    | Carousel_track_requested (id, _, _, _, _)
    | Choice (id, _, _, _, _)
    | Combobox_selected (id, _, _, _, _, _)
    | Choice_picker_event (id, _, _, _, _)
    | Palette_observed (id, _, _, _, _)
    | Palette_dismissed (id, _, _, _, _)
    | Toast_dismissed (id, _, _, _, _)
    | Drag_source_event (id, _, _, _, _)
    | Image_state (id, _, _, _, _)
    | Animation_endpoint (id, _, _, _, _)
    | Container_selected (id, _, _, _, _)
    | Animation_program_event (id, _, _, _, _)
    | List_viewport (id, _, _, _, _)
    | Drop_target_event (id, _, _, _, _)
    | Pointer_event (id, _, _, _, _)
    | Input_observed (id, _, _, _, _)
    | Highlight_observed (id, _, _, _, _)
    | Hover_changed (id, _, _, _, _)
    | Menu_open_changed (id, _, _, _, _)
    | Command_binding_observed (id, _, _, _, _)
    | Overlay_dismissed (id, _, _, _, _)
    | Tooltip_open_changed (id, _, _, _, _)
    | Command_invoked (id, _, _, _, _, _, _) ) as event ->
    Option.iter (find_window t id) ~f:(fun window ->
      if not (Window.is_closed window)
      then Option.iter window.driver ~f:(fun driver -> Driver.dispatch driver event))
  | Chart_response (request, response) ->
    (match Map.find t.charts request with
     | None -> ()
     | Some complete ->
       t.charts <- Map.remove t.charts request;
       complete (if t.stopping then Wire.Chart.Response.Failed Closed else response))
  | Canvas_response (request, response) ->
    (match Map.find t.canvases request with
     | None -> ()
     | Some complete ->
       t.canvases <- Map.remove t.canvases request;
       complete (if t.stopping then Wire.Canvas.Response.Failed Closed else response))
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
  | Slider_result (request, id, node, result) ->
    (match Map.find t.sliders request with
     | Some pending
       when Window_id.equal pending.window id && Node_id.equal pending.node node ->
       t.sliders <- Map.remove t.sliders request;
       let result =
         match result with
         | Failed error -> Error (Slider.Expert.error_of_wire error)
         | Applied snapshot ->
           Slider.Expert.snapshot_of_wire ~window:id ~node snapshot
           |> Result.map_error ~f:(fun _ -> Slider.Command_error.Native_failure)
       in
       pending.complete result
     | Some _ | None -> ())
  | Number_input_result (request, id, node, result) ->
    (match Map.find t.number_inputs request with
     | Some pending
       when Window_id.equal pending.window id && Node_id.equal pending.node node ->
       t.number_inputs <- Map.remove t.number_inputs request;
       let result =
         match result with
         | Failed error -> Error (Number_input.Expert.error_of_wire error)
         | Applied snapshot ->
           Number_input.Expert.snapshot_of_wire ~window:id ~node snapshot
           |> Result.map_error ~f:(fun _ -> Number_input.Command_error.Native_failure)
       in
       pending.complete result
     | Some _ | None -> ())
  | Otp_input_result (request, id, node, result) ->
    (match Map.find t.otp_inputs request with
     | Some pending
       when Window_id.equal pending.window id && Node_id.equal pending.node node ->
       t.otp_inputs <- Map.remove t.otp_inputs request;
       let result =
         match result with
         | Failed error -> Error (Otp_input.Expert.error_of_wire error)
         | Applied snapshot ->
           (match Otp_input.Expert.snapshot_of_wire ~window:id ~node snapshot with
            | Ok snapshot
              when Otp_input.Policy.equal
                     pending.policy
                     (Otp_input.Snapshot.policy snapshot)
                   && Otp_input.Revision.compare
                        (Otp_input.Snapshot.revision snapshot)
                        pending.minimum_revision
                      >= 0 -> Ok snapshot
            | Ok _ | Error _ -> Error Otp_input.Command_error.Native_failure)
       in
       pending.complete result
     | Some _ | None -> ())
  | Calendar_result (request, id, node, result) ->
    (match Map.find t.calendars request with
     | Some pending
       when Window_id.equal pending.window id && Node_id.equal pending.node node ->
       t.calendars <- Map.remove t.calendars request;
       let result =
         match result with
         | Failed error -> Error (Calendar.Expert.error_of_wire error)
         | Applied snapshot ->
           (match Calendar.Expert.snapshot_of_wire ~window:id ~node snapshot with
            | Ok snapshot
              when Calendar.Mode.equal pending.mode (Calendar.Snapshot.mode snapshot)
                   && Calendar.Revision.compare
                        (Calendar.Snapshot.revision snapshot)
                        pending.minimum_revision
                      >= 0 -> Ok snapshot
            | Ok _ | Error _ -> Error Calendar.Command_error.Native_failure)
       in
       pending.complete result
     | Some _ | None -> ())
  | Color_input_result (request, id, node, result) ->
    (match Map.find t.color_inputs request with
     | Some pending
       when Window_id.equal pending.window id && Node_id.equal pending.node node ->
       t.color_inputs <- Map.remove t.color_inputs request;
       let result =
         match result with
         | Failed error -> Error (Color_input.Expert.error_of_wire error)
         | Applied snapshot ->
           (match Color_input.Expert.snapshot_of_wire ~window:id ~node snapshot with
            | Ok snapshot
              when Color_input.Revision.compare
                     (Color_input.Snapshot.revision snapshot)
                     pending.minimum_revision
                   >= 0 -> Ok snapshot
            | Ok _ | Error _ -> Error Color_input.Command_error.Native_failure)
       in
       pending.complete result
     | Some _ | None -> ())
  | Palette_result (request, id, node, observer, result) ->
    (match Map.find t.palettes request with
     | Some pending
       when Window_id.equal (Palette.Expert.window pending.expected) id
            && Node_id.equal (Palette.Expert.node pending.expected) node
            && Handler_id.equal (Palette.Expert.observer pending.expected) observer ->
       t.palettes <- Map.remove t.palettes request;
       let result =
         match result with
         | Palette_command_wire.Response.Failed error -> Error error
         | Applied wire ->
           (match Palette.Expert.snapshot_of_wire ~window:id ~node ~observer wire with
            | Ok snapshot
              when Int64.(
                     Palette.Expert.sequence snapshot
                     >= Palette.Expert.sequence pending.expected
                     && Palette.Expert.query_revision snapshot
                        >= Palette.Expert.query_revision pending.expected) -> Ok snapshot
            | Ok _ | Error _ -> Error Palette.Command_error.Native_failure)
       in
       pending.complete result
     | Some _ | None -> ())
  | Editor_result (request, id, node, result) ->
    (match Map.find t.editors request with
     | Some pending
       when Window_id.equal pending.window id && Node_id.equal pending.node node ->
       t.editors <- Map.remove t.editors request;
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
  | Failed (request, _) when Map.mem t.desktop_requests request ->
    let complete = Map.find_exn t.desktop_requests request in
    t.desktop_requests <- Map.remove t.desktop_requests request;
    complete (Wire.Desktop.Response.Failed Native_failure)
  | Failed (request, _) when Map.mem t.notification_requests request ->
    let complete = Map.find_exn t.notification_requests request in
    t.notification_requests <- Map.remove t.notification_requests request;
    complete (Wire.Notification.Response.Failed Native_failure)
  | Failed (request, _) when Map.mem t.window_requests request ->
    let pending = Map.find_exn t.window_requests request in
    t.window_requests <- Map.remove t.window_requests request;
    pending.complete (Wire.Window.Response.Failed Native_window.Error.Native_failure)
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
  | Failed (request, code) when Map.mem t.sliders request ->
    let pending = Map.find_exn t.sliders request in
    t.sliders <- Map.remove t.sliders request;
    let error : Slider.Command_error.t =
      match code with
      | Closed -> Closed
      | Stale_handle -> Stale_slider
      | Busy | Overloaded -> Busy
      | Limit_exceeded -> Limit_exceeded
      | Unsupported_version
      | Unsupported_capability
      | Malformed
      | Not_ready
      | Invalid_revision
      | Invalid_tree
      | Native_failure -> Native_failure
    in
    pending.complete (Error error)
  | Failed (request, code) when Map.mem t.number_inputs request ->
    let pending = Map.find_exn t.number_inputs request in
    t.number_inputs <- Map.remove t.number_inputs request;
    let error : Number_input.Command_error.t =
      match code with
      | Closed -> Closed
      | Stale_handle -> Stale_input
      | Busy | Overloaded -> Busy
      | Limit_exceeded -> Limit_exceeded
      | Unsupported_version
      | Unsupported_capability
      | Malformed
      | Not_ready
      | Invalid_revision
      | Invalid_tree
      | Native_failure -> Native_failure
    in
    pending.complete (Error error)
  | Failed (request, code) when Map.mem t.otp_inputs request ->
    let pending = Map.find_exn t.otp_inputs request in
    t.otp_inputs <- Map.remove t.otp_inputs request;
    let error : Otp_input.Command_error.t =
      match code with
      | Closed -> Closed
      | Stale_handle -> Stale_input
      | Busy | Overloaded -> Busy
      | Limit_exceeded -> Limit_exceeded
      | Unsupported_version
      | Unsupported_capability
      | Malformed
      | Not_ready
      | Invalid_revision
      | Invalid_tree
      | Native_failure -> Native_failure
    in
    pending.complete (Error error)
  | Failed (request, code) when Map.mem t.calendars request ->
    let pending = Map.find_exn t.calendars request in
    t.calendars <- Map.remove t.calendars request;
    let error : Calendar.Command_error.t =
      match code with
      | Closed -> Closed
      | Stale_handle -> Stale_input
      | Busy | Overloaded -> Busy
      | Limit_exceeded -> Limit_exceeded
      | Unsupported_version
      | Unsupported_capability
      | Malformed
      | Not_ready
      | Invalid_revision
      | Invalid_tree
      | Native_failure -> Native_failure
    in
    pending.complete (Error error)
  | Failed (request, code) when Map.mem t.color_inputs request ->
    let pending = Map.find_exn t.color_inputs request in
    t.color_inputs <- Map.remove t.color_inputs request;
    let error : Color_input.Command_error.t =
      match code with
      | Closed -> Closed
      | Stale_handle -> Stale_color_input
      | Busy | Overloaded -> Busy
      | Limit_exceeded -> Limit_exceeded
      | Unsupported_version
      | Unsupported_capability
      | Malformed
      | Not_ready
      | Invalid_revision
      | Invalid_tree
      | Native_failure -> Native_failure
    in
    pending.complete (Error error)
  | Failed (request, code) when Map.mem t.palettes request ->
    let pending = Map.find_exn t.palettes request in
    t.palettes <- Map.remove t.palettes request;
    let error : Palette.Command_error.t =
      match code with
      | Closed -> Closed
      | Stale_handle -> Stale_palette
      | Busy | Overloaded -> Busy
      | Limit_exceeded
      | Unsupported_version
      | Unsupported_capability
      | Malformed
      | Not_ready
      | Invalid_revision
      | Invalid_tree
      | Native_failure -> Native_failure
    in
    pending.complete (Error error)
  | Failed (request, code) when Map.mem t.editors request ->
    let pending = Map.find_exn t.editors request in
    t.editors <- Map.remove t.editors request;
    let error : Wire.Editor.Error.t =
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
    pending.complete (Wire.Editor.Result.Failed error)
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
  | Failed (request, _) when Map.mem t.charts request ->
    let complete = Map.find_exn t.charts request in
    t.charts <- Map.remove t.charts request;
    complete (Wire.Chart.Response.Failed Native_failure)
  | Failed (request, _) when Map.mem t.canvases request ->
    let complete = Map.find_exn t.canvases request in
    t.canvases <- Map.remove t.canvases request;
    complete (Wire.Canvas.Response.Failed Native_failure)
  | Failed (request, _) when Map.mem t.documents request ->
    let complete = Map.find_exn t.documents request in
    t.documents <- Map.remove t.documents request;
    complete (Wire.Document.Response.Failed Native_failure)
  | Failed (_, code) | Rejected (_, _, code) -> Error.raise (native_error code)
  | Overloaded _ -> failwith "native input mailbox overloaded"
  | Stopped ->
    t.stopped <- true;
    t.stopping <- true;
    t.on_desktop_pending <- (fun () -> Bonsai.Effect.Ignore);
    t.desktop_subscription <- None;
    t.desktop_pending <- false;
    t.on_notification_pending <- (fun () -> Bonsai.Effect.Ignore);
    t.notification_subscription <- None;
    t.notification_pending <- false;
    t.notification_closed <- true;
    let desktop_requests = t.desktop_requests in
    t.desktop_requests <- Int64.Map.empty;
    Map.iter desktop_requests ~f:(fun complete ->
      complete (Wire.Desktop.Response.Failed Closed));
    let notification_requests = t.notification_requests in
    t.notification_requests <- Int64.Map.empty;
    Map.iter notification_requests ~f:(fun complete ->
      complete (Wire.Notification.Response.Failed Closed));
    Asset_registry.close t.asset_registry;
    Document_registry.close t.document_registry;
    Chart_registry.close t.chart_registry;
    Canvas_registry.close t.canvas_registry;
    let assets = t.assets in
    t.assets <- Int64.Map.empty;
    Map.iter assets ~f:(fun complete -> complete (Wire.Asset.Response.Failed Closed));
    let documents = t.documents in
    t.documents <- Int64.Map.empty;
    Map.iter documents ~f:(fun complete ->
      complete (Wire.Document.Response.Failed Closed));
    let charts = t.charts in
    t.charts <- Int64.Map.empty;
    Map.iter charts ~f:(fun complete -> complete (Wire.Chart.Response.Failed Closed));
    let canvases = t.canvases in
    t.canvases <- Int64.Map.empty;
    Map.iter canvases ~f:(fun complete -> complete (Wire.Canvas.Response.Failed Closed))
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
    if t.welcomed && not t.stopping
    then
      Option.iter (Chart_registry.next_request t.chart_registry) ~f:(fun request ->
        Bonsai.Effect.Expert.handle
          (Bonsai.Effect.map
             (Expert.chart_request t ~limit:64 request)
             ~f:(Chart_registry.complete t.chart_registry)));
    if t.welcomed && not t.stopping
    then
      Option.iter (Canvas_registry.next_request t.canvas_registry) ~f:(fun request ->
        Bonsai.Effect.Expert.handle
          (Bonsai.Effect.map
             (Expert.canvas_request t ~limit:64 request)
             ~f:(Canvas_registry.complete t.canvas_registry)));
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

let create_runtime ~document_defaults ~native ~inbox ~scope ~now ~motion ~desktop =
  let asset_registry = Asset_registry.create ~scope ~wake:(fun () -> Inbox.wake inbox) in
  { guard = Guard.create ()
  ; document_defaults
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
  ; sliders = Int64.Map.empty
  ; number_inputs = Int64.Map.empty
  ; otp_inputs = Int64.Map.empty
  ; calendars = Int64.Map.empty
  ; palettes = Int64.Map.empty
  ; color_inputs = Int64.Map.empty
  ; dialogs = Int64.Map.empty
  ; window_requests = Int64.Map.empty
  ; window_capabilities = None
  ; quit_pending = None
  ; on_reopen = (fun () -> Bonsai.Effect.Ignore)
  ; asset_registry
  ; document_registry = Document_registry.create ~scope ~wake:(fun () -> Inbox.wake inbox)
  ; canvas_registry =
      Canvas_registry.create
        ~scope
        ~asset_owner:(Asset_registry.Expert.owner asset_registry)
        ~wake:(fun () -> Inbox.wake inbox)
  ; assets = Int64.Map.empty
  ; documents = Int64.Map.empty
  ; chart_registry = Chart_registry.create ~scope ~wake:(fun () -> Inbox.wake inbox)
  ; charts = Int64.Map.empty
  ; canvases = Int64.Map.empty
  ; desktop_requests = Int64.Map.empty
  ; desktop_pending = false
  ; desktop_subscription = None
  ; on_desktop_pending = (fun () -> Bonsai.Effect.Ignore)
  ; notification_requests = Int64.Map.empty
  ; notification_pending = false
  ; notification_subscription = None
  ; on_notification_pending = (fun () -> Bonsai.Effect.Ignore)
  ; notification_closed = false
  ; desktop_identity = desktop
  ; correlation = 0L
  ; motion = Some motion
  ; welcomed = false
  ; stopping = false
  ; stopped = false
  ; stats = { turns = 0; clock_ticks = 0; commits = 0; rendered = 0; completed_jobs = 0 }
  }
;;

let dispose_runtime app =
  finish_cleanup (fun attempt ->
    attempt (fun () -> Asset_registry.close app.asset_registry);
    attempt (fun () -> Document_registry.close app.document_registry);
    attempt (fun () -> Chart_registry.close app.chart_registry);
    attempt (fun () -> Canvas_registry.close app.canvas_registry);
    attempt (fun () -> Scope.cancel app.scope);
    Map.iter app.windows ~f:(fun window -> attempt (fun () -> release_window window));
    app.assets <- Int64.Map.empty;
    app.documents <- Int64.Map.empty;
    app.charts <- Int64.Map.empty;
    app.canvases <- Int64.Map.empty;
    app.desktop_requests <- Int64.Map.empty;
    app.desktop_pending <- false;
    app.desktop_subscription <- None;
    app.on_desktop_pending <- (fun () -> Bonsai.Effect.Ignore);
    app.notification_requests <- Int64.Map.empty;
    app.notification_pending <- false;
    app.notification_subscription <- None;
    app.on_notification_pending <- (fun () -> Bonsai.Effect.Ignore);
    app.notification_closed <- true;
    app.frames <- Int64.Map.empty;
    app.closes <- Int64.Map.empty;
    app.opens <- Int64.Map.empty;
    Queue.clear app.commands;
    Inbox.close app.inbox)
;;

let worker native read ~document_defaults ~tick_hz ~max_tasks ~motion ~desktop initialize =
  Eio_main.run (fun env ->
    Eio.Switch.run (fun sw ->
      let inbox = Inbox.create ~capacity:1024 () in
      let scope = Scope.Expert.create ~sw ~inbox ~max_tasks in
      let now () =
        Time_ns.of_span_since_epoch
          (Time_ns.Span.of_sec (Eio.Time.now (Eio.Stdenv.clock env)))
      in
      let app =
        create_runtime ~document_defaults ~native ~inbox ~scope ~now ~motion ~desktop
      in
      Exn.protect
        ~finally:(fun () -> dispose_runtime app)
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
          Option.iter desktop ~f:(fun identity ->
            Bonsai.Effect.Expert.handle
              (Bonsai.Effect.map
                 (Expert.desktop
                    app
                    (Configure (Gpuio.Desktop.Expert.identity_to_wire identity)))
                 ~f:(function
                   | Wire.Desktop.Response.Configured -> ()
                   | Failed Closed when app.stopping -> ()
                   | response ->
                     raise_s
                       [%sexp
                         "desktop initialization failed"
                       , (response : Wire.Desktop.Response.t)])));
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

let run_with_preflight
      ?(document_defaults = Gpuio.Document.Defaults.empty)
      ?(tick_hz = 60.)
      ?(max_tasks = 1024)
      ?(exit_on_last_window = true)
      ?(motion = Gpuio.Animation.Preference.System)
      ?desktop
      ~preflight
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
      Exn.protect
        ~finally:(fun () -> Gpuio_native.dispose native)
        ~f:(fun () ->
          match preflight native with
          | Error error -> Error error
          | Ok `Forwarded -> Ok `Forwarded
          | Ok `Primary ->
            let record_backtraces = Stdlib.Printexc.backtrace_status () in
            let domain =
              Domain.spawn (fun () ->
                Stdlib.Printexc.record_backtrace record_backtraces;
                let result =
                  capture (fun () ->
                    worker
                      native
                      read
                      ~document_defaults
                      ~tick_hz
                      ~max_tasks
                      ~motion
                      ~desktop
                      initialize)
                in
                if Result.is_error result then Gpuio_native.abort native;
                (* Carry the worker's backtrace as data. Raising here would make
                   [Domain.join] replace its origin with the join call site. *)
                result)
            in
            let native_result = capture (fun () -> Gpuio_native.run native) in
            if Result.is_error native_result then Gpuio_native.abort native;
            let worker_result = capture (fun () -> Domain.join domain) |> Result.join in
            reraise_result worker_result;
            reraise_result native_result;
            Ok `Exited)))
;;

let document_profile_catalog () =
  let%bind.Or_error schemas = Gpuio_native.document_profile_catalog () in
  List.map schemas ~f:Gpuio.Document.Profile.Schema.Expert.of_wire |> Or_error.all
;;

let extension_catalog () =
  let%bind.Or_error schemas = Gpuio_native.extension_catalog () in
  List.map schemas ~f:Gpuio.Extension.Schema.Expert.of_wire |> Or_error.all
;;

let run
      ?document_defaults
      ?tick_hz
      ?max_tasks
      ?exit_on_last_window
      ?motion
      ?desktop
      initialize
  =
  match
    run_with_preflight
      ?document_defaults
      ?tick_hz
      ?max_tasks
      ?exit_on_last_window
      ?motion
      ?desktop
      ~preflight:(fun _ -> Ok `Primary)
      initialize
  with
  | Ok `Exited -> ()
  | Ok `Forwarded | Error () -> assert false
;;

module Launch_outcome = struct
  type t =
    | Exited
    | Forwarded
  [@@deriving equal, sexp_of]
end

let run_desktop
      ?document_defaults
      ?tick_hz
      ?max_tasks
      ?exit_on_last_window
      ?motion
      identity
      ~startup_links
      initialize
  =
  let preflight native =
    match
      Gpuio_native.prepare_desktop
        native
        { identity = Gpuio.Desktop.Expert.identity_to_wire identity
        ; links = startup_links
        }
    with
    | Primary -> Ok `Primary
    | Forwarded -> Ok `Forwarded
    | Failed error -> Error (Gpuio.Desktop.Expert.error_of_wire error)
  in
  Result.map
    (run_with_preflight
       ?document_defaults
       ?tick_hz
       ?max_tasks
       ?exit_on_last_window
       ?motion
       ~desktop:identity
       ~preflight
       initialize)
    ~f:(function
    | `Exited -> Launch_outcome.Exited
    | `Forwarded -> Forwarded)
;;

let%test_module "pending picker command lifecycle" =
  (module struct
    module E = Bonsai.Effect
    module P = Gpuio.Choice_picker

    let ok = Or_error.ok_exn

    (* Exercise real Eio fibers and native allocation/submission, but inject host
       lifecycle/reply events. Native dispatch/GUI/IME is not simulated here. *)
    let with_runtime ?(document_defaults = Gpuio.Document.Defaults.empty) f =
      Eio_main.run (fun env ->
        Eio.Switch.run (fun sw ->
          let _read, write = Eio_unix.pipe sw in
          let native =
            Eio_unix.Fd.use_exn
              "command lifecycle test"
              (Eio_unix.Resource.fd write)
              Gpuio_native.create
          in
          Eio.Flow.close write;
          Exn.protect
            ~finally:(fun () -> Gpuio_native.dispose native)
            ~f:(fun () ->
              let inbox = Inbox.create ~capacity:1024 () in
              let scope = Scope.Expert.create ~sw ~inbox ~max_tasks:16 in
              let app =
                create_runtime
                  ~document_defaults
                  ~native
                  ~inbox
                  ~scope
                  ~now:(fun () -> Time_ns.epoch)
                  ~motion:Gpuio.Animation.Preference.Reduce
                  ~desktop:None
              in
              Exn.protect
                ~finally:(fun () -> dispose_runtime app)
                ~f:(fun () ->
                  Eio.Time.with_timeout_exn (Eio.Stdenv.clock env) 5. (fun () -> f sw app)))))
    ;;

    let%expect_test "document defaults reach later windows and remain application-local" =
      let source =
        Gpuio.Text_source.Expert.handle
          ~owner:(Gpuio.Text_source.Expert.Owner.create ())
          (Resource_id.create ~slot:0L ~generation:1L |> ok)
      in
      let config ?appearance () =
        Gpuio.Document.Config.create ~source ~mode:Markdown ?appearance () |> ok
      in
      let inspect app config =
        let window =
          open_window
            app
            ~focus:false
            ~title:"Defaults contract"
            ~width:400.
            ~height:300.
            (fun _ _ -> Bonsai.Cont.return (Gpuio.View.document config))
          |> ok
        in
        process app (Opened (app.correlation, window.id));
        let driver = Option.value_exn window.driver in
        Driver.cycle driver ~now:Time_ns.epoch |> ok;
        let tx =
          match Driver.next_message driver with
          | Some (Apply tx) -> tx
          | _ -> assert false
        in
        let dark =
          List.find_map_exn tx.operations ~f:(function
            | Set_document (_, c) -> Some c.dark
            | _ -> None)
        in
        Driver.submitted driver;
        Driver.acknowledge driver ~revision:tx.revision |> ok;
        dark
      in
      let defaults = Gpuio.Document.Defaults.create ~appearance:Dark () |> ok in
      with_runtime ~document_defaults:defaults (fun _ app ->
        assert (inspect app (config ()));
        assert (not (inspect app (config ~appearance:Light ())));
        assert (inspect app (config ())));
      with_runtime (fun _ app -> assert (not (inspect app (config ()))));
      print_endline "inherited, explicit, later-window and separate-application defaults";
      [%expect {| inherited, explicit, later-window and separate-application defaults |}]
    ;;

    let open_picker app =
      let component _window _graph =
        let config =
          P.Config.create
            ~label:"Picker"
            ~options:(P.Collection.flat (Gpuio.Choice.Collection.create [] |> ok))
            ~selected:(P.Selection.single None)
            ~search:Substring
            ()
          |> ok
        in
        let query =
          P.Query.create
            ~controller:(Gpuio.Key.of_string "query" |> ok)
            ~initial_text:""
            ()
          |> ok
        in
        let description = P.Description.create ~config ~query () |> ok in
        Bonsai.Cont.return
          (Gpuio.View.choice_picker ~on_event:(fun _ -> E.Ignore) description |> ok)
      in
      let window =
        open_window
          app
          ~focus:false
          ~title:"Command lifecycle"
          ~width:400.
          ~height:300.
          component
        |> ok
      in
      let request = app.correlation in
      process app (Opened (request, window.id));
      let driver = Option.value_exn window.driver in
      Driver.cycle driver ~now:Time_ns.epoch |> ok;
      let tx =
        match Driver.next_message driver with
        | Some (Apply tx) -> tx
        | Some _ | None -> failwith "expected picker transaction"
      in
      let query =
        List.find_map_exn tx.operations ~f:(function
          | Create (node, Input, _, _) -> Some node
          | _ -> None)
      in
      Driver.submitted driver;
      Driver.acknowledge driver ~revision:tx.revision |> ok;
      let wire : Wire.Editor.Snapshot.t =
        { revision = 1L
        ; text = ""
        ; selection = { anchor = 0L; head = 0L }
        ; composition = None
        ; focused = false
        }
      in
      let snapshot =
        Input.Expert.snapshot_of_wire ~window:window.id ~node:query wire |> ok
      in
      window, snapshot, wire
    ;;

    let%expect_test
        "palette replies are correlated, monotone, bounded and closed exactly once"
      =
      with_runtime (fun _ app ->
        let window, editor, _ = open_picker app in
        let node = Input.Expert.node editor in
        let observer = Handler_id.create ~slot:9L ~generation:1L |> ok in
        let wire : Palette_state_wire.t =
          { sequence = 3L
          ; query_revision = 2L
          ; query = "run"
          ; composing = false
          ; selected = None
          ; matched_count = 0
          }
        in
        let snapshot =
          Palette.Expert.snapshot_of_wire ~window:window.id ~node ~observer wire |> ok
        in
        let start ?(command = Palette.Command.Read_snapshot) () =
          let result = ref None
          and calls = ref 0 in
          E.Expert.handle
            (E.map (Window.Expert.palette_command window snapshot command) ~f:(fun next ->
               incr calls;
               result := Some next));
          app.correlation, result, calls
        in
        let request, result, calls = start () in
        let wrong = Handler_id.create ~slot:9L ~generation:2L |> ok in
        process app (Palette_result (request, window.id, node, wrong, Applied wire));
        assert (Option.is_none !result && !calls = 0);
        process
          app
          (Palette_result
             (request, window.id, node, observer, Applied { wire with sequence = 2L }));
        assert (
          Option.equal
            (Result.equal Palette.Snapshot.equal Palette.Command_error.equal)
            !result
            (Some (Error Native_failure)));
        assert (!calls = 1);
        process app (Palette_result (request, window.id, node, observer, Applied wire));
        assert (!calls = 1);
        let _, invalid, invalid_calls = start ~command:(Set_query "x\n") () in
        assert (
          Option.equal
            (Result.equal Palette.Snapshot.equal Palette.Command_error.equal)
            !invalid
            (Some (Error Invalid_query))
          && !invalid_calls = 1);
        assert (Map.is_empty app.palettes);
        let pending = List.init 64 ~f:(fun _ -> start ()) in
        let _, busy, busy_calls = start () in
        assert (
          Option.equal
            (Result.equal Palette.Snapshot.equal Palette.Command_error.equal)
            !busy
            (Some (Error Busy))
          && !busy_calls = 1);
        let request, result, calls = List.hd_exn pending in
        process app (Palette_result (request, window.id, node, observer, Applied wire));
        assert (
          Option.equal
            (Result.equal Palette.Snapshot.equal Palette.Command_error.equal)
            !result
            (Some (Ok snapshot))
          && !calls = 1);
        release_window window;
        assert (Map.is_empty app.palettes);
        List.iter (List.tl_exn pending) ~f:(fun (_, result, calls) ->
          assert (
            Option.equal
              (Result.equal Palette.Snapshot.equal Palette.Command_error.equal)
              !result
              (Some (Error Closed))
            && !calls = 1));
        let _, closed, closed_calls = start () in
        assert (
          Option.equal
            (Result.equal Palette.Snapshot.equal Palette.Command_error.equal)
            !closed
            (Some (Error Closed))
          && !closed_calls = 1));
      print_endline
        "wrong observer ignored; regression rejected; 64 bounded; closure completes once";
      [%expect
        {| wrong observer ignored; regression rejected; 64 bounded; closure completes once |}]
    ;;

    let start window snapshot =
      let promise, resolver = Eio.Promise.create () in
      let calls = ref 0 in
      E.Expert.handle
        (E.map
           (Window.Expert.editor_command window snapshot Read_snapshot)
           ~f:(fun result ->
             incr calls;
             Eio.Promise.resolve resolver result));
      window.app.correlation, promise, calls
    ;;

    let print_result result =
      print_s
        [%sexp
          (Result.map result ~f:(fun snapshot -> Input.Snapshot.text snapshot)
           : (string, Input.Command_error.t) Result.t)]
    ;;

    let start_metadata window action =
      let promise, resolver = Eio.Promise.create () in
      let calls = ref 0 in
      E.Expert.handle
        (E.map action ~f:(fun result ->
           incr calls;
           Eio.Promise.resolve resolver result));
      window.app.correlation, promise, calls
    ;;

    let%expect_test "search replies correlate and stamps cannot cross editor leases" =
      let module S = Input.Search in
      with_runtime (fun _ app ->
        let window, snapshot, wire = open_picker app in
        let node = Input.Expert.node snapshot in
        let value : Gpuio_protocol.Editor_search_wire.Snapshot.t =
          { stamp = { editor_revision = wire.revision; search_revision = 0L }
          ; activation_revision = 0L
          ; mode = Closed
          ; query = ""
          ; case = Sensitive
          ; text_bytes = 0L
          ; match_count = 0L
          ; current = None
          ; can_replace = false
          }
        in
        let start command =
          start_metadata window (Window.Expert.editor_search window snapshot command)
        in
        let request, result, calls = start Read in
        let wrong_node = Node_id.create ~slot:999L ~generation:1L |> ok in
        process
          app
          (Editor_result (request, window.id, wrong_node, Search_observed value));
        assert (!calls = 0 && Map.length app.editors = 1);
        process app (Editor_result (request, window.id, node, Search_observed value));
        let search =
          match Eio.Promise.await result with
          | Ok (Observed value, None) -> value
          | _ -> assert false
        in
        process app (Editor_result (request, window.id, node, Search_observed value));
        assert (!calls = 1 && Map.is_empty app.editors);
        let wrong =
          S.Expert.snapshot_of_wire ~window:window.id ~node:wrong_node value |> ok
        in
        let _, result, calls =
          start (Replace_all { if_stamp = S.Snapshot.stamp wrong; replacement = "" })
        in
        assert (
          match Eio.Promise.await result with
          | Error Stale_editor -> true
          | _ -> false);
        assert (!calls = 1 && Map.is_empty app.editors);
        let _, foreign, _ = start (Close_and_focus wrong) in
        assert (
          match Eio.Promise.await foreign with
          | Error Stale_editor -> true
          | _ -> false);
        let request, result, _ = start (Close_and_focus search) in
        process
          app
          (Editor_result
             (request, window.id, node, Search_observed { value with mode = Find }));
        assert (
          match Eio.Promise.await result with
          | Error Native_failure -> true
          | _ -> false);
        let request, result, _ = start (Close_and_focus search) in
        process app (Editor_result (request, window.id, node, Search_observed value));
        assert (
          match Eio.Promise.await result with
          | Ok (Observed _, None) -> true
          | _ -> false);
        let request, result, _ =
          start (Replace_all { if_stamp = S.Snapshot.stamp search; replacement = "" })
        in
        process app (Editor_result (request, window.id, node, Search_observed value));
        assert (
          match Eio.Promise.await result with
          | Error Native_failure -> true
          | _ -> false);
        let request, result, _ =
          start (Replace_all { if_stamp = S.Snapshot.stamp search; replacement = "" })
        in
        process
          app
          (Editor_result (request, window.id, node, Search_replaced (wire, value, 0L)));
        assert (
          match Eio.Promise.await result with
          | Ok (Replaced { count = 0; _ }, Some _) -> true
          | _ -> false);
        let request, result, _ =
          start (Replace_all { if_stamp = S.Snapshot.stamp search; replacement = "" })
        in
        process
          app
          (Editor_result
             (request, window.id, node, Search_replaced (wire, value, Int64.max_value)));
        assert (
          match Eio.Promise.await result with
          | Error Native_failure -> true
          | _ -> false);
        let _, pending, calls = start Read in
        Window.close window;
        process app (Closed (app.correlation, window.id));
        assert (
          match Eio.Promise.await pending with
          | Error Closed -> true
          | _ -> false);
        assert (!calls = 1 && Map.is_empty app.editors));
      print_endline "exact correlation, reply kinds, lease stamps and close cleanup";
      [%expect {| exact correlation, reply kinds, lease stamps and close cleanup |}]
    ;;

    let%expect_test
        "range metadata validates source, reply revision, lease correlation and close"
      =
      with_runtime (fun _ app ->
        let window, base, wire = open_picker app in
        let node = Input.Expert.node base in
        let snapshot =
          Input.Expert.snapshot_of_wire
            ~window:window.id
            ~node
            { wire with revision = 3L; text = "A界" }
          |> ok
        in
        let range = Input.Selection.create ~anchor:4 ~head:1 |> ok in
        let geometry : Gpuio_protocol.Editor_geometry_wire.t =
          { revision = 3L; x = -12.5; y = 40.; width = 10.; height = 20. }
        in
        let query () =
          start_metadata window (Window.Expert.editor_range_bounds window snapshot range)
        in
        let request, result, calls = query () in
        let wrong_node = Node_id.create ~slot:999L ~generation:1L |> ok in
        process app (Editor_result (request, window.id, wrong_node, Range_bounds None));
        process
          app
          (Editor_result (Int64.succ request, window.id, node, Range_bounds None));
        assert (!calls = 0 && Map.length app.editors = 1);
        process
          app
          (Editor_result (request, window.id, node, Range_bounds (Some geometry)));
        let value =
          Eio.Promise.await result |> Result.ok |> Option.value_exn |> Option.value_exn
        in
        assert (Float.equal (Gpuio.Editor_geometry.x value) (-12.5));
        assert (
          Input.Revision.equal
            (Gpuio.Editor_geometry.revision value)
            (Input.Snapshot.revision snapshot));
        process app (Editor_result (request, window.id, node, Range_bounds None));
        assert (!calls = 1);
        List.iter
          [ Wire.Editor.Result.Range_bounds (Some { geometry with revision = 4L })
          ; Range_bounds (Some { geometry with height = 0. })
          ; Applied wire
          ; Viewport None
          ]
          ~f:(fun response ->
            let request, result, _ = query () in
            process app (Editor_result (request, window.id, node, response));
            assert (
              match Eio.Promise.await result with
              | Error Native_failure -> true
              | _ -> false));
        let invalid = Input.Selection.create ~anchor:2 ~head:4 |> ok in
        let _, result, _ =
          start_metadata
            window
            (Window.Expert.editor_range_bounds window snapshot invalid)
        in
        assert (
          match Eio.Promise.await result with
          | Error Invalid_selection -> true
          | _ -> false);
        assert (Map.is_empty app.editors);
        let ordinary = List.init 63 ~f:(fun _ -> start window snapshot) in
        let request, pending, calls = query () in
        let _, busy, _ = query () in
        assert (
          match Eio.Promise.await busy with
          | Error Busy -> true
          | _ -> false);
        process app (Closed (app.correlation, window.id));
        assert (
          match Eio.Promise.await pending with
          | Error Closed -> true
          | _ -> false);
        List.iter ordinary ~f:(fun (_, result, _) ->
          assert (Result.is_error (Eio.Promise.await result)));
        process
          app
          (Editor_result (request, window.id, node, Range_bounds (Some geometry)));
        assert (!calls = 1 && Map.is_empty app.editors));
      [%expect {| |}]
    ;;

    let%expect_test
        "viewport replies keep exact correlation and reject wrong metadata kinds"
      =
      with_runtime (fun _ app ->
        let window, snapshot, wire = open_picker app in
        let node = Input.Expert.node snapshot in
        let request, result, calls =
          start_metadata window (Window.Expert.editor_viewport window snapshot)
        in
        let wrong_node = Node_id.create ~slot:999L ~generation:1L |> ok in
        process app (Editor_result (request, window.id, wrong_node, Viewport None));
        process app (Editor_result (Int64.succ request, window.id, node, Viewport None));
        assert (Map.length app.editors = 1 && !calls = 0);
        process app (Editor_result (request, window.id, node, Viewport None));
        assert (
          Result.equal
            (Option.equal Gpuio.Editor_viewport.equal)
            Input.Command_error.equal
            (Eio.Promise.await result)
            (Ok None));
        process app (Editor_result (request, window.id, node, Viewport None));
        assert (!calls = 1);
        let request, result, _ =
          start_metadata window (Window.Expert.editor_viewport window snapshot)
        in
        process app (Editor_result (request, window.id, node, Applied wire));
        assert (Result.is_error (Eio.Promise.await result));
        let request, result, _ =
          start_metadata
            window
            (Window.Expert.editor_scroll_to
               window
               snapshot
               Gpuio.Editor_viewport.Offset.origin)
        in
        process app (Editor_result (request, window.id, node, Viewport None));
        assert (
          Result.equal
            Unit.equal
            Input.Command_error.equal
            (Eio.Promise.await result)
            (Error Native_failure));
        let request, result, _ =
          start_metadata
            window
            (Window.Expert.editor_scroll_to
               window
               snapshot
               Gpuio.Editor_viewport.Offset.origin)
        in
        process app (Editor_result (request, window.id, node, Viewport_scroll_accepted));
        assert (
          Result.equal
            Unit.equal
            Input.Command_error.equal
            (Eio.Promise.await result)
            (Ok ()));
        print_s [%sexp (Map.length app.editors : int)]);
      [%expect {| 0 |}]
    ;;

    let%expect_test
        "viewport reads and scrolls share capacity and release on window close"
      =
      with_runtime (fun _ app ->
        let window, snapshot, _ = open_picker app in
        let ordinary = List.init 62 ~f:(fun _ -> start window snapshot) in
        let _, read, read_calls =
          start_metadata window (Window.Expert.editor_viewport window snapshot)
        in
        let _, scroll, scroll_calls =
          start_metadata
            window
            (Window.Expert.editor_scroll_to
               window
               snapshot
               Gpuio.Editor_viewport.Offset.origin)
        in
        let _, busy, _ =
          start_metadata window (Window.Expert.editor_viewport window snapshot)
        in
        assert (
          Result.equal
            (Option.equal Gpuio.Editor_viewport.equal)
            Input.Command_error.equal
            (Eio.Promise.await busy)
            (Error Busy));
        process app (Closed (app.correlation, window.id));
        assert (
          Result.equal
            (Option.equal Gpuio.Editor_viewport.equal)
            Input.Command_error.equal
            (Eio.Promise.await read)
            (Error Closed));
        assert (
          Result.equal
            Unit.equal
            Input.Command_error.equal
            (Eio.Promise.await scroll)
            (Error Closed));
        List.iter ordinary ~f:(fun (_, result, calls) ->
          assert (
            Result.equal
              Input.Snapshot.equal
              Input.Command_error.equal
              (Eio.Promise.await result)
              (Error Closed));
          assert (!calls = 1));
        print_s
          [%sexp
            (!read_calls : int), (!scroll_calls : int), (Map.length app.editors : int)]);
      [%expect {| (1 1 0) |}]
    ;;

    let start_hint window snapshot =
      let promise, resolver = Eio.Promise.create () in
      let calls = ref 0 in
      E.Expert.handle
        (E.map
           (Window.Expert.editor_content_hint_status window snapshot)
           ~f:(fun result ->
             incr calls;
             Eio.Promise.resolve resolver result));
      window.app.correlation, promise, calls
    ;;

    let print_hint result =
      print_s
        [%sexp (result : (Input.Content_hint.Status.t, Input.Command_error.t) Result.t)]
    ;;

    let%expect_test
        "hint metadata replies preserve correlation and reject wrong result kinds"
      =
      with_runtime (fun _ app ->
        let window, snapshot, wire = open_picker app in
        let node = Input.Expert.node snapshot in
        let request, result, calls = start_hint window snapshot in
        let status = Wire.Editor.Result.Content_hint_status (Exposed Email_address) in
        let wrong_node = Node_id.create ~slot:999L ~generation:1L |> ok in
        let wrong_window = Window_id.create ~slot:999L ~generation:1L |> ok in
        process app (Editor_result (Int64.succ request, window.id, node, status));
        process app (Editor_result (request, window.id, wrong_node, status));
        process app (Editor_result (request, wrong_window, node, status));
        assert (Map.length app.editors = 1 && !calls = 0);
        process app (Editor_result (request, window.id, node, status));
        print_hint (Eio.Promise.await result);
        process app (Editor_result (request, window.id, node, status));
        assert (!calls = 1);
        let request, result, _ = start_hint window snapshot in
        process app (Editor_result (request, window.id, node, Applied wire));
        print_hint (Eio.Promise.await result);
        let request, result, _ = start window snapshot in
        process app (Editor_result (request, window.id, node, status));
        print_result (Eio.Promise.await result);
        print_s [%sexp (Map.length app.editors : int)]);
      [%expect
        {|
        (Ok (Exposed Email_address))
        (Error Native_failure)
        (Error Native_failure)
        0
      |}]
    ;;

    let%expect_test
        "hint requests share capacity and close cleanup with snapshot commands"
      =
      with_runtime (fun _ app ->
        let window, snapshot, _ = open_picker app in
        let snapshots = List.init 63 ~f:(fun _ -> start window snapshot) in
        let _, admitted, admitted_calls = start_hint window snapshot in
        let _, busy, _ = start_hint window snapshot in
        print_hint (Eio.Promise.await busy);
        assert (Map.length app.editors = 64);
        Window.close window;
        let _, closed, _ = start_hint window snapshot in
        print_hint (Eio.Promise.await closed);
        process app (Closed (app.correlation, window.id));
        print_hint (Eio.Promise.await admitted);
        List.iter snapshots ~f:(fun (_, result, calls) ->
          assert (
            Result.equal
              Input.Snapshot.equal
              Input.Command_error.equal
              (Eio.Promise.await result)
              (Error Closed));
          assert (!calls = 1));
        print_s [%sexp (!admitted_calls : int), (Map.length app.editors : int)]);
      [%expect
        {|
        (Error Busy)
        (Error Closed)
        (Error Closed)
        (1 0)
      |}]
    ;;

    let%expect_test
        "hint callback exceptions do not strand later requests and late replies are \
         ignored"
      =
      with_runtime (fun _ app ->
        let window, snapshot, _ = open_picker app in
        E.Expert.handle
          (E.map (Window.Expert.editor_content_hint_status window snapshot) ~f:(fun _ ->
             failwith "hint callback"));
        let request, result, calls = start_hint window snapshot in
        let _, snapshot_result, _ = start window snapshot in
        let failed =
          Result.is_error (Or_error.try_with (fun () -> release_window window))
        in
        assert failed;
        print_hint (Eio.Promise.await result);
        print_result (Eio.Promise.await snapshot_result);
        process
          app
          (Editor_result
             ( request
             , window.id
             , Input.Expert.node snapshot
             , Content_hint_status (Inactive None) ));
        print_s [%sexp (!calls : int), (Map.length app.editors : int)]);
      [%expect
        {|
        (Error Closed)
        (Error Closed)
        (1 0)
      |}]
    ;;

    let%expect_test
        "closing rejects new commands while admitted replies remain correlated"
      =
      with_runtime (fun sw app ->
        let window, snapshot, wire = open_picker app in
        let first, first_result, first_calls = start window snapshot in
        let second, second_result, second_calls = start window snapshot in
        let waiter_done, complete_waiter = Eio.Promise.create () in
        Eio.Fiber.fork ~sw (fun () ->
          let first = Eio.Promise.await first_result in
          let second = Eio.Promise.await second_result in
          Eio.Promise.resolve complete_waiter (first, second));
        process app (Welcome (Wire.version, Wire.capabilities));
        submit_commands app;
        assert ((Gpuio_native.command_queue app.native).commands > 0);
        let wrong = Node_id.create ~slot:999L ~generation:1L |> ok in
        process app (Editor_result (first, window.id, wrong, Applied wire));
        assert (Map.length app.editors = 2);
        Window.close window;
        let _, rejected, _ = start window snapshot in
        print_result (Eio.Promise.await rejected);
        process
          app
          (Editor_result (first, window.id, Input.Expert.node snapshot, Applied wire));
        process app (Closed (app.correlation, window.id));
        let first_result, second_result = Eio.Promise.await waiter_done in
        print_result first_result;
        print_result second_result;
        process
          app
          (Editor_result (second, window.id, Input.Expert.node snapshot, Applied wire));
        process app (Closed (app.correlation, window.id));
        print_s
          [%sexp
            (!first_calls : int)
          , (!second_calls : int)
          , (Map.length app.editors : int)
          , (Map.length app.windows : int)]);
      [%expect
        {|
        (Error Closed)
        (Ok "")
        (Error Closed)
        (1 1 0 0)
      |}]
    ;;

    let%expect_test
        "reused window slots fence old commands and disposal wakes pending fibers"
      =
      with_runtime (fun sw app ->
        let old, old_snapshot, wire = open_picker app in
        let old_request, old_result, _ = start old old_snapshot in
        release_window old;
        print_result (Eio.Promise.await old_result);
        let window, snapshot, _ = open_picker app in
        assert (Int64.equal (Window_id.slot old.id) (Window_id.slot window.id));
        assert (not (Window_id.equal old.id window.id));
        let _, stale, _ = start window old_snapshot in
        print_result (Eio.Promise.await stale);
        let _, pending, calls = start window snapshot in
        let done_, complete = Eio.Promise.create () in
        Eio.Fiber.fork ~sw (fun () ->
          Eio.Promise.resolve complete (Eio.Promise.await pending));
        process
          app
          (Editor_result
             (old_request, old.id, Input.Expert.node old_snapshot, Applied wire));
        assert (Map.length app.editors = 1);
        dispose_runtime app;
        print_result (Eio.Promise.await done_);
        print_s
          [%sexp
            (!calls : int)
          , (Map.length app.editors : int)
          , (Map.length app.windows : int)
          , (Scope.is_active app.scope : bool)]);
      [%expect
        {|
        (Error Closed)
        (Error Stale_editor)
        (Error Closed)
        (1 0 0 false)
      |}]
    ;;

    let%expect_test
        "pending limit is shared and one window closure releases only its commands"
      =
      with_runtime (fun _sw app ->
        let first, a, _ = open_picker app in
        let second, b, _ = open_picker app in
        let results =
          List.init 64 ~f:(fun i -> if i % 2 = 0 then start first a else start second b)
        in
        let _, busy, _ = start second b in
        print_result (Eio.Promise.await busy);
        release_window first;
        print_s [%sexp (Map.length app.editors : int)];
        let _, recovered, recovered_calls = start second b in
        print_s [%sexp (Map.length app.editors : int)];
        dispose_runtime app;
        List.iter results ~f:(fun (_, result, calls) ->
          assert (
            Result.equal
              Input.Snapshot.equal
              Input.Command_error.equal
              (Eio.Promise.await result)
              (Error Closed));
          assert (!calls = 1));
        print_result (Eio.Promise.await recovered);
        print_s [%sexp (!recovered_calls : int), (Map.length app.editors : int)]);
      [%expect
        {|
        (Error Busy)
        32
        33
        (Error Closed)
        (1 0)
      |}]
    ;;

    let%expect_test
        "a failed command completion does not strand other commands or cleanup"
      =
      with_runtime (fun _sw app ->
        let window, snapshot, _ = open_picker app in
        let failed_calls = ref 0 in
        E.Expert.handle
          (E.map (Window.Expert.editor_command window snapshot Read_snapshot) ~f:(fun _ ->
             incr failed_calls;
             failwith "completion failure"));
        let _, remaining, remaining_calls = start window snapshot in
        let failed =
          Result.is_error (Or_error.try_with (fun () -> release_window window))
        in
        print_s
          [%sexp
            (failed : bool)
          , (!failed_calls : int)
          , (!remaining_calls : int)
          , (Map.length app.windows : int)
          , (Scope.is_active window.scope : bool)
          , (Option.is_none window.driver : bool)];
        if !remaining_calls = 1 then print_result (Eio.Promise.await remaining));
      [%expect
        {|
        (true 1 1 0 false true)
        (Error Closed)
      |}]
    ;;

    let%expect_test "runtime disposal finishes other windows after a completion raises" =
      with_runtime (fun sw app ->
        let first, a, _ = open_picker app in
        let second, b, _ = open_picker app in
        E.Expert.handle
          (E.map (Window.Expert.editor_command first a Read_snapshot) ~f:(fun _ ->
             failwith "first window callback"));
        let _, pending, calls = start second b in
        let done_, complete = Eio.Promise.create () in
        Eio.Fiber.fork ~sw (fun () ->
          Eio.Promise.resolve complete (Eio.Promise.await pending));
        let failed =
          Result.is_error (Or_error.try_with (fun () -> dispose_runtime app))
        in
        print_s
          [%sexp
            (failed : bool)
          , (Map.length app.windows : int)
          , (Map.length app.editors : int)
          , (Queue.length app.commands : int)
          , (Scope.is_active app.scope : bool)
          , (Inbox.try_push app.inbox ignore : bool)];
        print_result (Eio.Promise.await done_);
        print_s [%sexp (!calls : int)];
        dispose_runtime app);
      [%expect
        {|
        (true 0 0 0 false false)
        (Error Closed)
        1
      |}]
    ;;

    let%expect_test "numeric cleanup bypasses saturation and keeps request identity" =
      with_runtime (fun _sw app ->
        (* Only bridge routing is under test: no native widget dispatch. *)
        let window, editor, _ = open_picker app in
        let other, _, _ = open_picker app in
        let node = Input.Expert.node editor in
        let wire : Gpuio_protocol.Number_input_wire.Snapshot.t =
          { revision = 7L
          ; domain =
              Gpuio.Numeric.Expert.to_wire
                (Gpuio.Numeric.Domain.create ~min:0. ~max:100. ~step:1. |> ok)
          ; draft = "12"
          ; committed = Number 12.
          ; selection = { anchor = 2L; head = 2L }
          ; composition = None
          ; focused = true
          }
        in
        let snapshot =
          Number_input.Expert.snapshot_of_wire ~window:window.id ~node wire |> ok
        in
        let request =
          match
            Number_input.Expert.event_of_wire
              ~window:window.id
              ~node
              (Step_requested
                 { id = 9L; direction = Increase; source = Keyboard; snapshot = wire })
            |> ok
          with
          | Step_requested request -> request
          | _ -> failwith "expected numeric step request"
        in
        let calls = ref 0 in
        let start () =
          E.Expert.handle
            (E.map
               (Window.Expert.number_input_command window snapshot Read_snapshot)
               ~f:(fun _ -> incr calls))
        in
        List.init 64 ~f:(fun _ -> start ()) |> ignore;
        let ordinary = app.correlation in
        start ();
        assert (!calls = 1 && Map.length app.number_inputs = 64);
        let queued = Queue.length app.commands in
        Window.Expert.decline_number_step other request;
        assert (Queue.length app.commands = queued);
        Window.Expert.decline_number_step window request;
        let cleanup = app.correlation in
        assert (Queue.length app.commands = queued + 1);
        (match List.last_exn (Queue.to_list app.commands) with
         | Number_input_command
             ( correlation
             , owner
             , target
             , Resolve_step { request_id; revision; value = None } ) ->
           assert (
             Int64.equal correlation cleanup
             && Window_id.equal owner window.id
             && Node_id.equal target node
             && Int64.equal request_id 9L
             && Int64.equal revision 7L)
         | _ -> failwith "expected original guarded decline");
        process app (Number_input_result (cleanup, window.id, node, Applied wire));
        assert (!calls = 1 && Map.length app.number_inputs = 64);
        process app (Number_input_result (ordinary, window.id, node, Applied wire));
        assert (!calls = 2 && Map.length app.number_inputs = 63);
        release_window window;
        let queued = Queue.length app.commands in
        Window.Expert.decline_number_step window request;
        assert (Queue.length app.commands = queued);
        assert (!calls = 65 && Map.is_empty app.number_inputs));
      print_endline
        "64 ordinary requests; guarded cleanup admitted; reply ignored; close drains";
      [%expect
        {| 64 ordinary requests; guarded cleanup admitted; reply ignored; close drains |}]
    ;;
  end)
;;
