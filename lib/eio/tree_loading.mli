open Core
module Request = Gpuio.Tree_loading.Request
module Status = Gpuio.Tree_loading.Status
module Page = Gpuio.Tree_loading.Page
module Snapshot = Gpuio.Tree_loading.Snapshot

(** Scoped tree loading on the UI domain. Up to four lazily allocated Eio workers
    remain idle without polling between requests. A cancelling worker is not reused
    until it leaves its old producer, so cancellation cannot multiply concurrency.

    [load] captures explicit Eio filesystem/network capabilities. Results, failures
    and [on_change] are delivered on the UI loop. Do not create/mutate this
    controller during Incremental graph evaluation. Parent scope/window ownership
    is independent of transient row lifetimes. *)
type 'data t

val create
  :  ?on_change:('data Snapshot.t -> unit Bonsai.Effect.t)
  -> scope:Scope.t
  -> 'data Gpuio.Tree.t
  -> load:(Request.t -> 'data Page.t Or_error.t)
  -> 'data t Or_error.t

val snapshot : 'data t -> 'data Snapshot.t
val value : 'data t -> 'data Snapshot.t Bonsai.Cont.t

(** Queue/closed-controller errors return directly. Admitted requests whose worker
    cannot start publish a Failed snapshot and require an explicit retry. *)
val request : _ t -> Gpuio.Tree.Id.t -> unit Or_error.t

val retry : _ t -> Gpuio.Tree.Id.t -> unit Or_error.t
val cancel : _ t -> Gpuio.Tree.Id.t -> unit
val cancel_subtree : _ t -> Gpuio.Tree.Id.t -> unit
val cancel_hidden : _ t -> Gpuio.Tree_state.t -> unit
val invalidate : _ t -> Gpuio.Tree.Id.t -> unit
val update : 'data t -> 'data Gpuio.Tree.t -> unit Or_error.t
val reset : 'data t -> 'data Gpuio.Tree.t -> unit Or_error.t
val close : _ t -> unit
