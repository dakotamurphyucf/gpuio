open Core

(** Application-scoped, UI-domain notification delivery. No callback runs on a
    native delegate thread. Delivery yields between events and awaits each effect. *)
type t

val create
  :  schedule:((unit -> unit) -> unit)
  -> take:(unit -> Gpuio_protocol.Notification_wire.Response.t Bonsai.Effect.t)
  -> on_event:(Gpuio.Notification.Event.t -> unit Bonsai.Effect.t)
  -> t

(** Coalesced availability hint. No intake before [ready]. *)
val available : t -> unit

(** Monotonic and idempotent; requests one initial batch. *)
val ready : t -> unit

(** Explicit retry after an intake error. No automatic polling or retry timer. *)
val retry : t -> unit

(** Drops queued events and callback captures. Late responses and completion of
    an already-running effect cannot resume delivery. Native cleanup is owned by
    the application adapter. *)
val close : t -> unit

val is_closed : t -> bool
