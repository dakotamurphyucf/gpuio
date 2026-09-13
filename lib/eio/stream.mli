open Core

(** Ordered bounded batches. Push from Eio producer fibers on the UI domain.
    Values pushed before the scheduler drains the batch share one effect.
    Capacity counts values, not bytes: callers must bound individual payloads.
    [push] does not yield: a full scheduler queue or batch returns an error
    without accepting the value. Do not push during graph construction. *)
type 'a t

val create
  :  scope:Scope.t
  -> capacity:int
  -> on_batch:('a list -> unit Bonsai.Effect.t)
  -> 'a t Or_error.t

val push : 'a t -> 'a -> unit Or_error.t
val close : _ t -> unit
