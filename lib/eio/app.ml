open Core
open Gpuio_protocol
module Guard = Gpuio_runtime_core.Domain_guard
module Driver = Gpuio_runtime_core.Window_driver
module Input = Gpuio.Text_input

type editor_result = (Input.Snapshot.t, Input.Command_error.t) Result.t

type editor_request =
  { window : Window_id.t
  ; node : Node_id.t
  ; complete : editor_result -> unit
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
  ; mutable correlation : int64
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

let release_window window =
  match window.phase with
  | Closed -> ()
  | Opening | Open | Closing_before_open | Closing ->
    window.phase <- Closed;
    let cancelled, remaining =
      Map.partition_tf window.app.editors ~f:(fun request ->
        Window_id.equal request.window window.id)
    in
    window.app.editors <- remaining;
    Map.iter cancelled ~f:(fun request -> request.complete (Error Closed));
    Scope.cancel window.scope;
    Option.iter window.driver ~f:Driver.close;
    window.driver <- None;
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
      Scope.cancel t.scope;
      Inbox.wake t.app.inbox
    | Open ->
      t.phase <- Closing;
      Scope.cancel t.scope;
      let request = correlation t.app in
      t.app.closes <- Map.set t.app.closes ~key:request ~data:t.id;
      queue t.app (Close (request, t.id))
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

let open_window t ?(theme = Gpuio.Theme.default) ~title ~width ~height component =
  check t;
  if t.stopping
  then Or_error.error_string "application stopping"
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
      let window = { app = t; id; scope; phase = Opening; driver = None } in
      let driver =
        try Driver.create id ~start:(t.now ()) ~theme (component window) with
        | exn ->
          Scope.cancel scope;
          raise exn
      in
      window.driver <- Some driver;
      t.generations <- Map.set t.generations ~key:slot ~data:generation;
      t.windows <- Map.set t.windows ~key:slot ~data:window;
      let request = correlation t in
      let message = Wire.Message.Open (request, id, title, width, height) in
      t.opens <- Map.set t.opens ~key:request ~data:message;
      queue t message;
      Ok window)
;;

let find_window t id =
  Map.find t.windows (Window_id.slot id |> Int64.to_int_exn)
  |> Option.filter ~f:(fun window -> Window_id.equal window.id id)
;;

let native_error code = Error.create_s (Wire.Error_code.sexp_of_t code)

let process t = function
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
  | Accepted (id, revision) ->
    Option.iter (find_window t id) ~f:(fun window ->
      if not (Window.is_closed window)
      then
        Option.iter window.driver ~f:(fun driver ->
          Driver.acknowledge driver ~revision |> Or_error.ok_exn));
    Inbox.wake t.inbox
  | ( Press (id, _, _, _)
    | Editor_event (id, _, _, _, _, _)
    | Choice (id, _, _, _, _)
    | Combobox_selected (id, _, _, _, _, _) ) as event ->
    Option.iter (find_window t id) ~f:(fun window ->
      if not (Window.is_closed window)
      then Option.iter window.driver ~f:(fun driver -> Driver.dispatch driver event))
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
  | Failed (_, code) | Rejected (_, _, code) -> Error.raise (native_error code)
  | Overloaded _ -> failwith "native input mailbox overloaded"
  | Stopped ->
    t.stopped <- true;
    t.stopping <- true
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
  if t.welcomed then loop 64
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

let worker native read ~tick_hz ~max_tasks initialize =
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
        ; correlation = 0L
        ; welcomed = false
        ; stopping = false
        ; stopped = false
        ; stats =
            { turns = 0; clock_ticks = 0; commits = 0; rendered = 0; completed_jobs = 0 }
        }
      in
      Exn.protect
        ~finally:(fun () ->
          Scope.cancel scope;
          Map.iter app.windows ~f:release_window;
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

let run ?(tick_hz = 60.) ?(max_tasks = 1024) ?(exit_on_last_window = true) initialize =
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
          try worker native read ~tick_hz ~max_tasks initialize with
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
