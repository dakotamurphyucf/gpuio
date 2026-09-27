open Core
module Identity = Gpuio.Desktop.Identity
module Capabilities = Gpuio.Desktop.Capabilities
module Error = Gpuio.Desktop.Error
module Event = Gpuio.Desktop.Event
module Document = Gpuio.Window.Document

(** One application-scoped incoming-link subscription, owned by the UI domain.
    Its lifetime is independent of all windows. *)
type t

(** Requires [App.run ~desktop:identity] or [App.run_desktop identity].
    A second live receiver returns [Busy].
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

(** Ask the file manager to reveal a native path. macOS's API has no completion
    result: [Ok ()] means the request was submitted, not that a selection appeared.
    Linux awaits the OpenURI portal's OpenDirectory response (version 3+), which
    may open the containing folder without selecting the item. Linux needs an
    accessible regular file/directory; missing portal service returns [Unavailable]
    and an older portal returns [Unsupported]. Neither backend certifies visible
    presentation. *)
val reveal_file : App.t -> Gpuio.File_path.t -> (unit, Error.t) Result.t Bonsai.Effect.t

(** Open with the OS-selected application. macOS waits for the asynchronous
    workspace completion; [Ok ()] does not prove the target application rendered
    or consumed the document. Native errors distinguish known denial/missing
    resource cases (including wrapped native errors); unclassified errors return
    [Native_failure]. Does not add a recent item or show an application-choice
    prompt on macOS when no handler is available.

    Linux uses OpenURI.OpenFile (version 2+), passing an owned descriptor and
    requesting writable access for a sandboxed target. It uses the portal's
    default/last application choice where available; the portal may still prompt.
    User cancellation maps to [Denied]. No native window is borrowed for parenting.
    Linux shutdown requests portal cancellation and waits for worker cleanup. *)
val open_file : App.t -> Gpuio.File_path.t -> (unit, Error.t) Result.t Bonsai.Effect.t

(** Explicitly request this packaged app as the default handler for a scheme
    declared in both its identity and bundle metadata. This changes OS registration and may prompt the
    user. Never called automatically. The running bundle identifier must match
    the configured identity; unbundled execution returns [Unavailable].
    Linux currently reports [Unsupported]; packaging declarations are separate.
    Shutdown finishes pending requests with [Closed]; late OS callbacks are ignored,
    although already-submitted OS work may still take effect. *)
val register_scheme
  :  App.t
  -> Gpuio.Deep_link.Scheme.t
  -> (unit, Error.t) Result.t Bonsai.Effect.t
