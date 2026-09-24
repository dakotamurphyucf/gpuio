open Core
module Direction = Gpuio.List_paging.Direction
module Boundary = Gpuio.List_paging.Boundary
module Request = Gpuio.List_paging.Request
module Status = Gpuio.List_paging.Status

module Page : sig
  type ('key, 'data) t =
    { rows : ('key * 'data) list
    ; next : Boundary.t
    }
end

(** A scoped, UI-domain-owned paged collection. [load] runs as an Eio producer;
    capture the explicit filesystem/network capabilities it needs. [on_change]
    executes on the UI loop for status or data changes. It should publish a
    snapshot to the application's Bonsai state, not perform blocking I/O.

    Give this controller a conversation/application scope when loads should
    survive row deactivation. Viewport changes do not cancel tasks. [reset],
    [cancel], [close] and parent-scope cancellation cancel affected producers
    and suppress their queued completions. At most two producers are active.
    Do not create or mutate controllers during Incremental graph evaluation. *)
type ('key, 'data, 'cmp) t

val create
  :  scope:Scope.t
  -> ('key, 'data, 'cmp) Gpuio.List_collection.t
  -> before:Boundary.t
  -> after:Boundary.t
  -> load:(Request.t -> ('key, 'data) Page.t Or_error.t)
  -> on_change:(unit -> unit Bonsai.Effect.t)
  -> ('key, 'data, 'cmp) t Or_error.t

val items : ('key, 'data, 'cmp) t -> ('key, 'data, 'cmp) Gpuio.List_collection.t
val status : (_, _, _) t -> Direction.t -> Status.t
val request : (_, _, _) t -> Direction.t -> unit Or_error.t
val retry : (_, _, _) t -> Direction.t -> unit Or_error.t
val cancel : (_, _, _) t -> Direction.t -> unit

val reset
  :  ('key, 'data, 'cmp) t
  -> ('key, 'data, 'cmp) Gpuio.List_collection.t
  -> before:Boundary.t
  -> after:Boundary.t
  -> unit Or_error.t

val set : ('key, 'data, _) t -> key:'key -> data:'data -> unit Or_error.t
val close : (_, _, _) t -> unit
