open Core
module Content = Gpuio.Notification
module Tag = Content.Tag
module Receipt = Content.Receipt
module Authorization = Content.Authorization
module Capabilities = Content.Capabilities
module Error = Content.Error
module Event = Content.Event

(** Application-scoped OS notifications, owned by the UI domain and independent
    of windows. There is one service per application. *)
type t

(** Requires an application desktop identity. A second live receiver is [Busy].
    Does not prompt or submit a notification. Call [ready] after application state
    can route events. Handlers run in order and intake waits for their effects.
    Route through current model state and exact [App.Window.t] handles, never a
    raw window slot stored in a tag. No window is automatically opened/activated. *)
val attach : App.t -> on_event:(Event.t -> unit Bonsai.Effect.t) -> (t, Error.t) Result.t

val ready : t -> unit

(** Retry event intake after an error; no automatic polling is installed.
    This does not reconnect a failed Linux notification session or replay owned
    notifications after daemon loss. See operation errors to choose a fallback. *)
val retry : t -> unit

(** Idempotently disables the service for this application lifetime, drops queued
    callbacks and requests best-effort removal of owned OS notifications. Shutdown
    performs the same cleanup. An already-running handler may finish; late native
    responses cannot restart it. Use [dismiss] for an individual removal result. *)
val close : t -> unit

val is_closed : t -> bool

(** Backend support snapshot, separate from permission and visible presentation.
    macOS requires a matching packaged bundle identity; unbundled development
    returns [Unavailable]. *)
val capabilities : t -> (Capabilities.t, Error.t) Result.t Bonsai.Effect.t

val authorization : t -> (Authorization.t, Error.t) Result.t Bonsai.Effect.t

(** Explicit permission request; may show an OS prompt. Posting never implicitly
    prompts. Before a decision, posting returns [Not_ready]; denial is [Denied]. *)
val request_authorization : t -> (Authorization.t, Error.t) Result.t Bonsai.Effect.t

(** Submit plaintext content; success is OS acceptance, not proof of presentation.
    A duplicate live tag is [Busy]. There are 128 live/queued native lifetimes and
    16 concurrent operations. Capability/permission failures allow the application
    to choose a fallback such as an in-app toast. *)
val post : t -> tag:Tag.t -> Content.t -> (Receipt.t, Error.t) Result.t Bonsai.Effect.t

(** Keeps the same logical receipt/tag. Old custom action revisions are rejected;
    default activation identifies the lifetime, not a painted content revision. *)
val replace : t -> Receipt.t -> Content.t -> (unit, Error.t) Result.t Bonsai.Effect.t

(** Immediately disables this receipt's action routing after native admission.
    Failed removal retains a bounded retired slot so dismissal can be retried;
    replacement is then stale. macOS reports submission of its void removal API,
    not verified disappearance. Already-admitted events retain their receipt. *)
val dismiss : t -> Receipt.t -> (unit, Error.t) Result.t Bonsai.Effect.t
