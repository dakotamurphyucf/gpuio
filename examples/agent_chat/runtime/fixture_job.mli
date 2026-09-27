open Core

(** Window-scoped CPU fixture work: one running producer and at most one pending
    replacement. Superseded/cancelled results are ignored. Cancellation retires
    delivery but lets an already-running pure calculation finish before admitting
    another. Parent cancellation cancels the Eio task and clears pending work. *)
type 'a t

val create : scope:Gpuio_eio.Scope.t -> 'a t Or_error.t

(** Run outside Bonsai evaluation. [f] runs in an Eio task and may use an explicit
    worker-domain capability; [on_result] executes on the UI domain. *)
val submit
  :  'a t
  -> f:(unit -> 'a)
  -> on_result:('a Or_error.t -> unit Bonsai.Effect.t)
  -> unit Bonsai.Effect.t

val cancel : _ t -> unit
