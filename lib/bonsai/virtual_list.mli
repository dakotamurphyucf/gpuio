open Core
module B = Bonsai.Cont
module Config = Gpuio.Virtual_list.Config
module Viewport = Gpuio.Virtual_list.Viewport

module Controller : sig
  (** Effects belong to this mounted list generation. An absent target or an
      inactive generation ignores a delayed command. New commands receive
      monotonically increasing serials on the OCaml UI domain. *)
  type 'key t

  val scroll_to : 'key t -> ?offset:float -> 'key -> unit Bonsai.Effect.t Or_error.t
  val reveal : 'key t -> 'key -> unit Bonsai.Effect.t
  val jump_to_latest : _ t -> unit Bonsai.Effect.t
end

module Output : sig
  type 'key t

  val view : _ t -> unit Bonsai.Effect.t Gpuio.View.t
  val controller : 'key t -> 'key Controller.t

  (** [None] until native layout describes the current geometry revision. *)
  val viewport : _ t -> Viewport.t option

  val active_rows : _ t -> int
  val budget_exhausted : _ t -> bool
end

(** A bounded transient row computation for an immutable keyed collection.
    [row_key] must be stable and injective; collisions are reported as errors.
    Share collections through [List_collection.set] for streamed value updates.
    [generation] defaults to zero; change it when replacing a conversation whose
    row keys could be reused. This releases the old native list and resets its
    transient Bonsai model after acceptance. It does not cancel application work.

    [render_row] receives a lifetime for guarding asynchronous completions. Rows
    must follow [Managed_rows.assoc]'s default-reset contract. Keep preferences,
    messages and conversation tasks outside row computations. [pinned] adds
    application-owned retention to the native focus/composition/selection pins.
    Pins count toward [Config.max_active]; excess pins produce an error.

    The viewport must have a bounded height, supplied by [style] or its parent.
    The list fills its assigned area. Initial layout uses native placeholders,
    then asynchronously mounts the requested rows. No OCaml code runs in native
    layout callbacks. [on_viewport] is optional application observation, not a
    requirement to manage the active set.

    Collection/order metadata and one accepted immutable collection snapshot are
    O(logical rows). Only the requested/pinned subset creates row computations.
    Height invalidations compare against the accepted snapshot, including when
    streaming updates coalesce while native acceptance is pending. *)
val component
  :  ('key, 'cmp) B.comparator
  -> ('key, 'data, 'cmp) Gpuio.List_collection.t B.t
  -> row_key:('key -> Gpuio.Key.t)
  -> config:Config.t
  -> ?key:Gpuio.Key.t
  -> ?style:Gpuio.Style.t B.t
  -> ?generation:int64 B.t
  -> ?pinned:'key list B.t
  -> ?on_viewport:(Viewport.t -> unit Bonsai.Effect.t) B.t
  -> render_row:
       (key:'key B.t
        -> data:'data B.t
        -> lifetime:Managed_rows.Lifetime.t B.t
        -> B.graph
        -> unit Bonsai.Effect.t Gpuio.View.t B.t)
  -> B.graph
  -> 'key Output.t Or_error.t B.t

module Paging : sig
  module Direction = Gpuio.List_paging.Direction

  type t

  (** Handlers must ignore a generation that no longer matches their source.
      The Eio pager's [controls] adapter supplies this check. *)
  val create
    :  request:(generation:int64 -> Direction.t -> unit Bonsai.Effect.t)
    -> retry:(generation:int64 -> Direction.t -> unit Bonsai.Effect.t)
    -> cancel:(generation:int64 -> Direction.t -> unit Bonsai.Effect.t)
    -> t

  val request : t -> generation:int64 -> Direction.t -> unit Bonsai.Effect.t
  val retry : t -> generation:int64 -> Direction.t -> unit Bonsai.Effect.t
  val cancel : t -> generation:int64 -> Direction.t -> unit Bonsai.Effect.t
end

(** Drives ready boundaries from the native viewport, including empty or short
    pages. Failed boundaries require explicit [Paging.retry]; leaving the viewport
    never cancels a request. [auto_load=false] suspends new automatic requests;
    use it alongside explicit cancellation when loading should remain paused.
    Generation comes from the snapshot, so replacing a conversation resets rows.
    Read loading/failure/end states from that same snapshot to render controls. *)
val paged
  :  ('key, 'cmp) B.comparator
  -> ('key, 'data, 'cmp) Gpuio.List_paging.Snapshot.t B.t
  -> paging:Paging.t B.t
  -> row_key:('key -> Gpuio.Key.t)
  -> config:Config.t
  -> ?key:Gpuio.Key.t
  -> ?style:Gpuio.Style.t B.t
  -> ?pinned:'key list B.t
  -> ?auto_load:bool B.t
  -> ?on_viewport:(Viewport.t -> unit Bonsai.Effect.t) B.t
  -> render_row:
       (key:'key B.t
        -> data:'data B.t
        -> lifetime:Managed_rows.Lifetime.t B.t
        -> B.graph
        -> unit Bonsai.Effect.t Gpuio.View.t B.t)
  -> B.graph
  -> 'key Output.t Or_error.t B.t
