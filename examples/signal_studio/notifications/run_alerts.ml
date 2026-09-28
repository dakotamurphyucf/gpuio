open Core
module N = Gpuio.Notification
module E = Bonsai.Effect

module Backend = struct
  type t =
    { authorization : unit -> (N.Authorization.t, N.Error.t) Result.t E.t
    ; authorize : unit -> (N.Authorization.t, N.Error.t) Result.t E.t
    ; capabilities : unit -> (N.Capabilities.t, N.Error.t) Result.t E.t
    ; post : N.t -> (N.Receipt.t, N.Error.t) Result.t E.t
    ; replace : N.Receipt.t -> N.t -> (unit, N.Error.t) Result.t E.t
    ; dismiss : N.Receipt.t -> (unit, N.Error.t) Result.t E.t
    ; close : unit -> unit
    }
end

module State = struct
  type t =
    { enabled : bool
    ; busy : bool
    ; has_notification : bool
    ; authorization : (N.Authorization.t, N.Error.t) Result.t option
    ; message : string
    }

  let initial =
    { enabled = false
    ; busy = false
    ; has_notification = false
    ; authorization = None
    ; message = "Enable alerts to hear when a run finishes."
    }
  ;;
end

type t =
  { backend : Backend.t
  ; activate : unit -> unit E.t
  ; on_state : State.t -> unit
  ; log : string -> unit
  ; mutable state : State.t
  ; mutable receipt : N.Receipt.t option
  ; mutable pending_run : int option
  ; mutable event_waiter : (unit -> unit) option
  ; mutable closed : bool
  }

let create backend ~activate ~on_state ~log =
  { backend
  ; activate
  ; on_state
  ; log
  ; state = State.initial
  ; receipt = None
  ; pending_run = None
  ; event_waiter = None
  ; closed = false
  }
;;

let state t = t.state

let publish t =
  t.state <- { t.state with has_notification = Option.is_some t.receipt };
  t.on_state t.state
;;

let message t message =
  t.state <- { t.state with message };
  t.log message;
  publish t
;;

let error_message = function
  | N.Error.Unavailable ->
    "Desktop alerts are unavailable; run results stay in this window."
  | Denied -> "Notifications are denied. Run results stay in this window."
  | Unsupported -> "This desktop does not support the requested alert update."
  | Not_ready -> "Enable alerts before sending a notification."
  | Busy -> "An alert operation is already in progress."
  | Closed -> "The alert service is closed."
  | Stale -> "That notification has already finished."
  | Native_failure ->
    "The desktop could not process this alert. Run results are still available."
  | Invalid_request -> "This alert request is invalid."
;;

let report_error t error =
  (match error with
   | N.Error.Denied | Unavailable | Closed | Not_ready | Native_failure ->
     t.state <- { t.state with enabled = false }
   | Invalid_request | Unsupported | Busy | Stale -> ());
  t.log ("alert error " ^ Sexp.to_string ([%sexp_of: N.Error.t] error));
  message t (error_message error)
;;

let open_action = N.Action_id.of_string "open-workspace" |> Or_error.ok_exn

let content (capabilities : N.Capabilities.t) run =
  N.create
    ~title:(sprintf "Signal Studio · Run %02d complete" run)
    ~body:(if capabilities.body then "Your model evaluation workspace is ready." else "")
    ~actions:
      (if capabilities.actions
       then [ N.Action.create open_action ~label:"Open workspace" |> Or_error.ok_exn ]
       else [])
    ()
  |> Or_error.ok_exn
;;

let release_waiter t =
  let waiter = t.event_waiter in
  t.event_waiter <- None;
  Option.iter waiter ~f:(fun wake -> wake ())
;;

let rec finish t =
  if t.closed
  then E.Ignore
  else (
    t.state <- { t.state with busy = false };
    publish t;
    release_waiter t;
    pump t)

and pump t =
  if t.closed || t.state.busy
  then E.Ignore
  else (
    match t.pending_run with
    | None -> E.Ignore
    | Some run ->
      t.pending_run <- None;
      if not t.state.enabled
      then (
        let reason =
          match t.state.authorization with
          | Some (Error error) -> error_message error
          | Some (Ok Denied) -> error_message Denied
          | None | Some (Ok (Authorized | Provisional | Not_required | Not_determined)) ->
            "Alerts are off."
        in
        message t (sprintf "Run %02d complete. %s" run reason);
        E.Ignore)
      else (
        t.state <- { t.state with busy = true };
        publish t;
        E.bind (t.backend.capabilities ()) ~f:(function
          | Error error ->
            if not t.closed then report_error t error;
            finish t
          | Ok capabilities ->
            if t.closed
            then finish t
            else (
              let content = content capabilities run in
              let post () =
                E.map (t.backend.post content) ~f:(function
                  | Ok receipt ->
                    if not t.closed
                    then (
                      t.receipt <- Some receipt;
                      message t (sprintf "Run %02d notification submitted." run))
                  | Error error -> if not t.closed then report_error t error)
              in
              let operation =
                match t.receipt with
                | None -> post ()
                | Some receipt when capabilities.replacement ->
                  E.map (t.backend.replace receipt content) ~f:(function
                    | Ok () ->
                      if not t.closed
                      then message t (sprintf "Run %02d notification updated." run)
                    | Error error ->
                      if not t.closed
                      then (
                        if N.Error.equal error Stale then t.receipt <- None;
                        report_error t error))
                | Some receipt when capabilities.dismissal ->
                  E.bind (t.backend.dismiss receipt) ~f:(function
                    | Error error ->
                      if not t.closed then report_error t error;
                      E.Ignore
                    | Ok () ->
                      t.receipt <- None;
                      if t.closed then E.Ignore else post ())
                | Some _ ->
                  report_error t Unsupported;
                  E.Ignore
              in
              E.bind operation ~f:(fun () -> finish t)))))
;;

let authorization t ~enable operation =
  E.bind
    (E.of_thunk (fun () ->
       if t.closed || t.state.busy
       then false
       else (
         t.state <- { t.state with busy = true };
         publish t;
         true)))
    ~f:(fun admitted ->
      if not admitted
      then E.Ignore
      else
        E.bind (operation ()) ~f:(fun result ->
          if not t.closed
          then (
            t.state <- { t.state with authorization = Some result };
            t.log
              ("alert authorization "
               ^ Sexp.to_string
                   (Result.sexp_of_t N.Authorization.sexp_of_t N.Error.sexp_of_t result));
            match result with
            | Error error -> report_error t error
            | Ok (Authorized | Provisional | Not_required) ->
              if enable then t.state <- { t.state with enabled = true };
              message
                t
                (if t.state.enabled
                 then "Run completion alerts are enabled."
                 else "Desktop alerts are available. Enable them to opt in.")
            | Ok Denied ->
              t.state <- { t.state with enabled = false };
              message t (error_message Denied)
            | Ok Not_determined ->
              t.state <- { t.state with enabled = false };
              message t "Enable alerts to request notification permission.");
          finish t))
;;

let probe t = authorization t ~enable:false t.backend.authorization
let enable t = authorization t ~enable:true t.backend.authorize

let notify t ~run =
  E.bind
    (E.of_thunk (fun () ->
       if not t.closed
       then
         if run < 0 || run > 100
         then report_error t Invalid_request
         else t.pending_run <- Some run))
    ~f:(fun () -> pump t)
;;

let dismiss t =
  E.bind
    (E.of_thunk (fun () ->
       if t.closed || t.state.busy
       then None
       else (
         match t.receipt with
         | None ->
           message t "There is no active notification.";
           None
         | Some receipt ->
           t.state <- { t.state with busy = true };
           publish t;
           Some receipt)))
    ~f:(function
      | None -> E.Ignore
      | Some receipt ->
        E.bind (t.backend.dismiss receipt) ~f:(fun result ->
          if not t.closed
          then (
            match result with
            | Ok () ->
              t.receipt <- None;
              message t "Notification dismissal requested."
            | Error error -> report_error t error);
          finish t))
;;

let handle_event t event =
  let await_idle =
    E.Expert.of_fun ~f:(fun ~callback ->
      if t.closed || not t.state.busy
      then callback ()
      else (
        (* Notification delivery awaits each handler; one waiter is sufficient. *)
        assert (Option.is_none t.event_waiter);
        t.event_waiter <- Some callback))
  in
  E.bind await_idle ~f:(fun () ->
    if t.closed
    then E.Ignore
    else (
      match event with
      | N.Event.Failed error ->
        t.receipt <- None;
        t.state <- { t.state with enabled = false };
        report_error t error;
        E.Ignore
      | Activated receipt | Action (receipt, _) | Closed (receipt, _) ->
        if not (Option.equal N.Receipt.equal t.receipt (Some receipt))
        then E.Ignore
        else (
          t.receipt <- None;
          publish t;
          match event with
          | Activated _ ->
            message t "Opening the current workspace.";
            t.activate ()
          | Action (_, action) when N.Action_id.equal action open_action ->
            message t "Opening the current workspace.";
            t.activate ()
          | Action _ ->
            message t "Unknown notification action ignored.";
            E.Ignore
          | Closed _ ->
            message t "Notification closed.";
            E.Ignore
          | Failed _ -> E.Ignore)))
;;

let close t =
  if not t.closed
  then (
    t.closed <- true;
    t.pending_run <- None;
    t.receipt <- None;
    t.state
    <- { t.state with
         enabled = false
       ; busy = false
       ; has_notification = false
       ; message = "The alert service is closed."
       };
    t.backend.close ();
    release_waiter t)
;;
