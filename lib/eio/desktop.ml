open Core
module Identity = Gpuio.Desktop.Identity
module Capabilities = Gpuio.Desktop.Capabilities
module Error = Gpuio.Desktop.Error
module Event = Gpuio.Desktop.Event
module Delivery = Gpuio_runtime_core.Desktop_delivery
module Wire = Gpuio_protocol.Desktop_wire
module E = Bonsai.Effect

type t =
  { delivery : Delivery.t
  ; mutable unregister : unit -> unit
  ; mutable unregister_cleanup : unit -> unit
  }

let close t =
  Delivery.close t.delivery;
  t.unregister ();
  t.unregister_cleanup ();
  t.unregister <- Fn.id;
  t.unregister_cleanup <- Fn.id
;;

let attach app ~on_event =
  match App.desktop_identity app with
  | None -> Error Error.Not_ready
  | Some identity ->
    let scope = App.scope app in
    if not (Scope.is_active scope)
    then Error Error.Closed
    else (
      let delivery =
        Delivery.create
          ~identity
          ~schedule:(Scope.Expert.enqueue scope)
          ~take:(fun () -> App.Expert.desktop app Take_links)
          ~on_event
      in
      match
        App.Expert.on_desktop_pending app (fun () ->
          E.of_thunk (fun () -> Delivery.available delivery))
      with
      | Error error ->
        Delivery.close delivery;
        Error error
      | Ok unregister ->
        let t = { delivery; unregister; unregister_cleanup = Fn.id } in
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

let capabilities app =
  E.map (App.Expert.desktop app Capabilities) ~f:(function
    | Wire.Response.Capabilities caps ->
      Ok (Gpuio.Desktop.Expert.capabilities_of_wire caps)
    | Failed error -> Error (Gpuio.Desktop.Expert.error_of_wire error)
    | Configured | Links _ | Requested | Registered -> Error Error.Native_failure)
;;

let activate app ?(ignoring_other_apps = false) () =
  E.map (App.Expert.desktop app (Activate ignoring_other_apps)) ~f:(function
    | Wire.Response.Requested -> Ok ()
    | Failed error -> Error (Gpuio.Desktop.Expert.error_of_wire error)
    | Configured | Links _ | Capabilities _ | Registered -> Error Error.Native_failure)
;;
