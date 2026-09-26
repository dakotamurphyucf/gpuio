open Core
module Direction = Gpuio.Table_paging.Direction
module Boundary = Gpuio.Table_paging.Boundary
module Request = Gpuio.Table_paging.Request
module Status = Gpuio.Table_paging.Status
module Snapshot = Gpuio.Table_paging.Snapshot

module Page : sig
  type 'data t =
    { rows : (Gpuio.Table_data.Id.t * 'data) list
    ; next : Boundary.t
    }
end

(** Application/window-scoped table data, independent of transient cell models.
    Two lazily allocated reusable workers bound concurrent producers, including
    cancellation cleanup. A worker is not reused until it exits its previous
    producer and returns through the UI inbox. At most the latest request per
    boundary waits behind these workers; rapid resets do not grow a queue.

    [load] runs with Eio and captures explicit I/O capabilities. Its request
    contains immutable query settings captured at admission. Results and optional
    [on_change] effects are delivered on the UI loop. Workers block while idle;
    no polling. Never create or mutate a controller during Incremental evaluation.
    Physical cancellation and Core token checks both fence obsolete results. *)
type ('query, 'data) t

val create
  :  ?on_change:(('query, 'data) Snapshot.t -> unit Bonsai.Effect.t)
  -> scope:Scope.t
  -> query:'query
  -> 'data Gpuio.Table_data.t
  -> before:Boundary.t
  -> after:Boundary.t
  -> load:('query Request.t -> 'data Page.t Or_error.t)
  -> ('query, 'data) t Or_error.t

val snapshot : ('query, 'data) t -> ('query, 'data) Snapshot.t
val value : ('query, 'data) t -> ('query, 'data) Snapshot.t Bonsai.Cont.t

(** Request/queue admission errors return directly. A worker-start or producer
    failure publishes Failed and requires explicit retry. Repeated requests
    while loading, failed or at end do not start a producer. *)
val request : (_, _) t -> Direction.t -> unit Or_error.t

val retry : (_, _) t -> Direction.t -> unit Or_error.t
val cancel : (_, _) t -> Direction.t -> unit

(** Atomically validate and accept a new query/source, then cancel old producers.
    Old results already waiting in the UI inbox are also ignored. Invalid reset
    leaves current data, query and producers unchanged. *)
val reset
  :  ('query, 'data) t
  -> query:'query
  -> 'data Gpuio.Table_data.t
  -> before:Boundary.t
  -> after:Boundary.t
  -> unit Or_error.t

val set : (_, 'data) t -> key:Gpuio.Table_data.Id.t -> data:'data -> unit Or_error.t
val append : (_, 'data) t -> (Gpuio.Table_data.Id.t * 'data) list -> unit Or_error.t

(** Idempotently cancel only this controller's workers and queued loads, retire
    completion delivery and unregister its scope hook. Data remains readable.
    Parent-scope cancellation also closes it. Other controllers/tasks in the
    same scope are unaffected by explicit close. *)
val close : (_, _) t -> unit

(** Generation-checked controls for [Gpuio_bonsai.Table.paged]. Delayed actions
    from an obsolete query, a closed controller or a cancelled scope do nothing. *)
val controls : (_, _) t -> Gpuio_bonsai.Table.Paging.t
