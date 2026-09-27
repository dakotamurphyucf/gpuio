open Core
module Identity = Gpuio.Desktop.Identity
module Capabilities = Gpuio.Desktop.Capabilities
module Error = Gpuio.Desktop.Error
module Event = Gpuio.Desktop.Event
module Document = Gpuio.Window.Document

(** One application-scoped incoming-link subscription, owned by the UI domain.
    Its lifetime is independent of all windows. *)
type t

(** Requires [App.run ~desktop:identity]. A second live receiver returns [Busy].
    No links are delivered until [ready]. Application shutdown closes the receiver.
    Events are delivered in FIFO order, waiting for each effect to complete before
    starting the next, with a scheduler yield between callbacks. Invalid links
    become [Rejected_link]; native drops become a separate [Overflow] event after
    accepted entries in that batch. No links are automatically opened or routed.

    A handler may use current application data to select/open windows and perform
    scoped Eio work. Long effects intentionally backpressure link delivery;
    bounded native overflow is reported instead of accumulating callbacks. *)
val attach : App.t -> on_event:(Event.t -> unit Bonsai.Effect.t) -> (t, Error.t) Result.t

(** Idempotent. A ready application may have no windows. *)
val ready : t -> unit

(** Retry intake after a reported transient [Failed] event. No automatic timer
    polling or retry is installed. Coalesces with an already-pending request. *)
val retry : t -> unit

(** Idempotent; releases queued links, callback captures and subscription. An
    already-running application effect can finish, but cannot restart delivery.
    A retired native response is discarded rather than routed to a new receiver. *)
val close : t -> unit

val is_closed : t -> bool

(** Set represented-file and edited metadata on this exact window generation.
    Clearing the path supports untitled documents. Does no file I/O and does not
    replace close/quit handlers. macOS returns the observed native state; the
    pinned X11/Wayland backends return [Unsupported]. AppKit may normalize paths
    in its observation; non-UTF8 bytes are never replaced with Unicode markers. *)
val set_document
  :  App.Window.t
  -> Document.t
  -> (Gpuio.Window.Snapshot.t, Gpuio.Window.Error.t) Result.t Bonsai.Effect.t

val capabilities : App.t -> (Capabilities.t, Error.t) Result.t Bonsai.Effect.t

(** Request process-level activation. Linux reports [Unsupported] in the pinned
    backend; window activation remains [App.Window.command]. Success means the
    request was accepted, not proof that the OS foregrounded the application. *)
val activate
  :  App.t
  -> ?ignoring_other_apps:bool
  -> unit
  -> (unit, Error.t) Result.t Bonsai.Effect.t
