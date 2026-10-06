open Core

(** Attach one controller to one context menu using [key] and [observe] as its
    [on_change]. Both drawn and platform context menus are supported. *)
type t

val create : App.Window.t -> Bonsai.Cont.graph -> t Bonsai.Cont.t
val key : t -> Gpuio.Key.t
val snapshot : t -> Gpuio.Menu.Snapshot.t option
val observe : t -> Gpuio.Menu.Snapshot.t -> unit Bonsai.Effect.t
val reset : t -> unit Bonsai.Effect.t

(** Capture the observed window/node/subscription. A delayed effect cannot
    retarget a replacement definition or mount. [Show] requires an active,
    interactive owner and returns [Busy] if it is already open. AppKit also
    permits only one pending/tracking popup across the application. [Close] is
    idempotent for a current subscription, including an unavailable owner.
    Success means native admission, not paint or user selection. Registry
    command invocation is separate. Clear local observation with [reset] when
    removing the view; native identity checks still protect stale snapshots. *)
val command
  :  t
  -> Gpuio.Menu.Command.t
  -> (unit, Gpuio.Menu.Command_error.t) Result.t Bonsai.Effect.t
