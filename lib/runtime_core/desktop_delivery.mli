open Core

(** Single UI-domain application delivery. Native admission and UI scheduling
    are injected by the runner; callbacks never run on an OS delegate thread. *)
type t

val create
  :  identity:Gpuio.Desktop.Identity.t
  -> schedule:((unit -> unit) -> unit)
  -> take:(unit -> Gpuio_protocol.Desktop_wire.Response.t Bonsai.Effect.t)
  -> on_event:(Gpuio.Desktop.Event.t -> unit Bonsai.Effect.t)
  -> t

(** Coalesced hint; no request before readiness and no second concurrent take. *)
val available : t -> unit

(** Monotonic and idempotent. Initial readiness requests one batch even if an
    earlier native availability hint was consumed by a previous subscriber. *)
val ready : t -> unit

(** Explicitly request another batch, e.g. after a transient failure. Repeated
    retry calls coalesce. No timer or automatic busy retry is installed. *)
val retry : t -> unit

(** Release input and handlers. Late responses/completions cannot deliver more
    events. An already-running application effect is not cancelled by this. *)
val close : t -> unit

val is_closed : t -> bool
