open Core

(** One UI-domain owner shared by every gallery window. Attaches desktop and
    notification receivers once; application shutdown disposes them. Retains only
    the latest observations and one notification receipt, never an event history.
    Page departure does not close application services. *)
type t

val identity : Gpuio.Desktop.Identity.t
val scheme : Gpuio.Deep_link.Scheme.t
val create : Gpuio_eio.App.t -> t
val ready : t -> unit

module Snapshot : sig
  type t =
    { desktop_support : string
    ; notification_support : string
    ; authorization : string
    ; link : string
    ; notice : string
    ; busy : bool
    ; has_receipt : bool
    }
end

val snapshot : t -> Snapshot.t Bonsai.Cont.t

(** Explicit operations, serialized at effect execution across all windows.
    Checking support never prompts; permission is requested only by [allow].
    Posting success means OS acceptance, not visible presentation. Notification
    actions only update the shared observation; they never activate a window.
    Receipt matching prevents a delayed event retiring a newer notification. *)
val check : t -> unit Bonsai.Effect.t

val allow : t -> unit Bonsai.Effect.t
val post : t -> unit Bonsai.Effect.t
val replace : t -> unit Bonsai.Effect.t
val dismiss : t -> unit Bonsai.Effect.t
val retry : t -> unit Bonsai.Effect.t
