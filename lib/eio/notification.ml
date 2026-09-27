open Core
module Content = Gpuio.Notification
module Tag = Content.Tag
module Receipt = Content.Receipt
module Authorization = Content.Authorization
module Capabilities = Content.Capabilities
module Error = Content.Error
module Event = Content.Event
module Wire = Gpuio_protocol.Notification_wire
module Delivery = Gpuio_runtime_core.Notification_delivery
module E = Bonsai.Effect

type t =
  { mutable app : App.t option
  ; delivery : Delivery.t
  ; mutable unregister : unit -> unit
  ; mutable unregister_cleanup : unit -> unit
  }

let close t =
  Delivery.close t.delivery;
  t.unregister ();
  t.unregister_cleanup ();
  t.unregister <- Fn.id;
  t.unregister_cleanup <- Fn.id;
  let app = t.app in
  t.app <- None;
  Option.iter app ~f:(fun app ->
    E.Expert.handle (E.map (App.Expert.notification app Close) ~f:(fun _ -> ())))
;;

let attach app ~on_event =
  match App.desktop_identity app with
  | None -> Error Error.Not_ready
  | Some _ ->
    let scope = App.scope app in
    if not (Scope.is_active scope)
    then Error Error.Closed
    else (
      let delivery =
        Delivery.create
          ~schedule:(Scope.Expert.enqueue scope)
          ~take:(fun () -> App.Expert.notification app Take_events)
          ~on_event
      in
      match
        App.Expert.on_notification_pending app (fun () ->
          E.of_thunk (fun () -> Delivery.available delivery))
      with
      | Error error ->
        Delivery.close delivery;
        Error error
      | Ok unregister ->
        let t = { app = Some app; delivery; unregister; unregister_cleanup = Fn.id } in
        (match Scope.on_cancel scope (fun () -> close t) with
         | Error _ ->
           close t;
           Error Error.Busy
         | Ok unregister_cleanup ->
           t.unregister_cleanup <- unregister_cleanup;
           Ok t))
;;

let ready t = Delivery.ready t.delivery
let retry t = Delivery.retry t.delivery
let is_closed t = Delivery.is_closed t.delivery

let request t value =
  E.lazy_
    (lazy
      (if is_closed t
       then E.return (Wire.Response.Failed Closed)
       else (
         match t.app with
         | Some app -> App.Expert.notification app value
         | None -> E.return (Wire.Response.Failed Closed))))
;;

let capabilities t =
  E.map (request t Capabilities) ~f:(function
    | Wire.Response.Capabilities value -> Ok (Content.Expert.capabilities_of_wire value)
    | Failed error -> Error (Content.Expert.error_of_wire error)
    | Authorization _ | Posted _ | Replaced | Dismiss_requested | Events _ | Closed ->
      Error Error.Native_failure)
;;

let authorization_request t command =
  E.map (request t command) ~f:(function
    | Wire.Response.Authorization value -> Ok (Content.Expert.authorization_of_wire value)
    | Failed error -> Error (Content.Expert.error_of_wire error)
    | Capabilities _ | Posted _ | Replaced | Dismiss_requested | Events _ | Closed ->
      Error Error.Native_failure)
;;

let authorization t = authorization_request t Authorization
let request_authorization t = authorization_request t Request_authorization

let post t ~tag content =
  E.map
    (request t (Post (Tag.to_string tag, Content.Expert.to_wire content)))
    ~f:(function
      | Wire.Response.Posted receipt ->
        Result.map_error (Content.Expert.receipt_of_wire receipt) ~f:(fun _ ->
          Error.Native_failure)
      | Failed error -> Error (Content.Expert.error_of_wire error)
      | Capabilities _
      | Authorization _
      | Replaced
      | Dismiss_requested
      | Events _
      | Closed -> Error Error.Native_failure)
;;

let replace t receipt content =
  E.map
    (request
       t
       (Replace (Content.Expert.receipt_to_wire receipt, Content.Expert.to_wire content)))
    ~f:(function
      | Wire.Response.Replaced -> Ok ()
      | Failed error -> Error (Content.Expert.error_of_wire error)
      | Capabilities _
      | Authorization _
      | Posted _
      | Dismiss_requested
      | Events _
      | Closed -> Error Error.Native_failure)
;;

let dismiss t receipt =
  E.map
    (request t (Dismiss (Content.Expert.receipt_to_wire receipt)))
    ~f:(function
      | Wire.Response.Dismiss_requested -> Ok ()
      | Failed error -> Error (Content.Expert.error_of_wire error)
      | Capabilities _ | Authorization _ | Posted _ | Replaced | Events _ | Closed ->
        Error Error.Native_failure)
;;
