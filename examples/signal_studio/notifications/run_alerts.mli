open Core
module N = Gpuio.Notification
module E = Bonsai.Effect

module Backend : sig
  (** Explicit asynchronous platform boundary, supplied by Gpuio_eio.Notification.
      Tests can control replies without prompting or changing OS preferences. *)
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

module State : sig
  type t =
    { enabled : bool
    ; busy : bool
    ; has_notification : bool
    ; authorization : (N.Authorization.t, N.Error.t) Result.t option
    ; message : string
    }

  val initial : t
end

type t

(** Owned by the UI domain/application, independent of windows. At most one native
    mutation and one latest pending run are retained. Enable is explicit; probing
    never prompts or opts in. [activate] resolves current application state when
    invoked, rather than capturing a window at posting time. *)
val create
  :  Backend.t
  -> activate:(unit -> unit E.t)
  -> on_state:(State.t -> unit)
  -> log:(string -> unit)
  -> t

val state : t -> State.t
val probe : t -> unit E.t
val enable : t -> unit E.t

(** Runs must be 0..100. While busy, newer requests replace the pending run.
    Existing content is replaced or explicitly dismissed before posting again.
    Denial/unavailability never discards the in-app run result. *)
val notify : t -> run:int -> unit E.t

val dismiss : t -> unit E.t

(** Pass this as the serial Notification service handler. A callback that races
    an operation reply waits asynchronously for that reply; exactly one waiter
    is retained because service event delivery is serial. Foreign/stale receipts
    cannot activate the workspace. Only the declared "open-workspace" action is
    accepted. *)
val handle_event : t -> N.Event.t -> unit E.t

(** Idempotent terminal service cleanup; late replies/events cannot reactivate it.
    Does not call [on_state] or [log], so it is safe in a scope cleanup with a
    nonblocking backend [close]. *)
val close : t -> unit
