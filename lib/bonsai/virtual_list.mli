open Core
module B = Bonsai.Cont
module Config = Gpuio.Virtual_list.Config
module Viewport = Gpuio.Virtual_list.Viewport

module Input : sig
  (** Persistent-list interaction attached to the existing native viewport. Cursor
      and callbacks use collection keys; the adapter maps native row identities.
      An accessibility List_box role is required; tree input is incompatible. *)
  type 'key t

  (** [query] names one direct Input view in [before]/[after]. These are siblings
      of the actual native list, inside its existing public layout wrapper. They
      may also contain loading, empty-state or footer content. Updating slots or
      query input preserves the keyed list owner and its rows/scroll handles.
      Give fixed-height query/status controls nonshrinking styles as appropriate.

      [epoch] follows List_input.Config's source/query policy contract. Callback
      effects are guarded by the list's mounted generation; reducers must also
      reject an obsolete application/query epoch while a commit is in flight.
      Busy/cursor updates do not retire ordered relative navigation. *)
  val create
    :  epoch:Gpuio.Key.t
    -> ?cursor:'key
    -> ?query:Gpuio.Key.t
    -> ?before:unit Bonsai.Effect.t Gpuio.View.t list
    -> ?after:unit Bonsai.Effect.t Gpuio.View.t list
    -> ?selection_on_navigation:bool
    -> ?disabled:bool
    -> ?busy:bool
    -> on_input:('key Gpuio.List_input.t -> unit Bonsai.Effect.t)
    -> unit
    -> 'key t Or_error.t
end

module Controller : sig
  (** Effects belong to this mounted list generation. An absent target or an
      inactive generation ignores a delayed command. Leaving and revisiting the
      same generation creates a new lifetime: old controllers stay invalid.
      New commands receive
      monotonically increasing serials on the OCaml UI domain. *)
  type 'key t

  val scroll_to : 'key t -> ?offset:float -> 'key -> unit Bonsai.Effect.t Or_error.t
  val reveal : 'key t -> 'key -> unit Bonsai.Effect.t

  (** Requires [on_tree_input]. Reveals and requests eventual native row focus;
      the current target must expose enabled TreeItem semantics when mounted. *)
  val focus_tree_row : 'key t -> 'key -> unit Bonsai.Effect.t

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

    [accessibility] annotates the native list root, not its layout wrapper.
    [on_tree_input] opts into native tree input and requires a Tree root role.
    [tree_moves] defaults to false and requires that input callback. It enables
    same-tree native move proposals; changing it retires the handler epoch.
    Events contain collection keys and must be reduced against current data;
    obsolete native row IDs are discarded before delivery.
    A TreeItem-annotated row container transfers its metadata to the native row
    wrapper, preserving one semantic row and its existing focus handle.

    The viewport must have a bounded main-axis extent (height for vertical lists,
    width for horizontal lists), supplied by [style] or its parent.
    The list fills its assigned area. Initial layout uses native placeholders,
    then asynchronously mounts the requested rows. No OCaml code runs in native
    layout callbacks. [on_viewport] is optional application observation, not a
    requirement to manage the active set.

    While native layout reports tail following, the bounded newest rows are
    prefetched in the same update as collection appends. Pins retain priority;
    native layout still controls scrolling. A paused tail uses its requested
    history instead. Prefetch never exceeds [Config.max_active].

    Collection/order metadata and one accepted immutable collection snapshot are
    O(logical rows). Only the requested/pinned/prefetched subset creates row computations.
    Measurement invalidations compare against the accepted snapshot, including when
    streaming updates coalesce while native acceptance is pending.

    [scrollbar] configures the actual native viewport, inside any layout
    wrappers. It defaults to [B.return None], which retains legacy native
    presentation. Changing it preserves the viewport and mounted row state;
    [config]'s scrollbar enable flag still takes precedence. *)
val component
  :  ('key, 'cmp) B.comparator
  -> ('key, 'data, 'cmp) Gpuio.List_collection.t B.t
  -> row_key:('key -> Gpuio.Key.t)
  -> config:Config.t
  -> ?key:Gpuio.Key.t
  -> ?style:Gpuio.Style.t B.t
  -> ?scrollbar:Gpuio.Scrollbar.t option B.t
  -> ?accessibility:Gpuio.Accessibility.t B.t
  -> ?on_tree_input:('key Gpuio.Tree_input.t -> unit Bonsai.Effect.t) B.t
  -> ?input:'key Input.t B.t
  -> ?tree_moves:bool B.t
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

(** Reactive-configuration version of [component]. Changing axis or sizing keeps
    the collection generation, controller lifetime and surviving row models.
    Native measurement/capture state is replaced while retaining a surviving
    logical anchor; viewport observations then reflect the new geometry. *)
val component_with_config
  :  ('key, 'cmp) B.comparator
  -> ('key, 'data, 'cmp) Gpuio.List_collection.t B.t
  -> row_key:('key -> Gpuio.Key.t)
  -> config:Config.t B.t
  -> ?key:Gpuio.Key.t
  -> ?style:Gpuio.Style.t B.t
  -> ?scrollbar:Gpuio.Scrollbar.t option B.t
  -> ?accessibility:Gpuio.Accessibility.t B.t
  -> ?on_tree_input:('key Gpuio.Tree_input.t -> unit Bonsai.Effect.t) B.t
  -> ?input:'key Input.t B.t
  -> ?tree_moves:bool B.t
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
  -> ?scrollbar:Gpuio.Scrollbar.t option B.t
  -> ?accessibility:Gpuio.Accessibility.t B.t
  -> ?on_tree_input:('key Gpuio.Tree_input.t -> unit Bonsai.Effect.t) B.t
  -> ?input:'key Input.t B.t
  -> ?tree_moves:bool B.t
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

(** Reactive-configuration version of [paged], with the same paging lifetime. *)
val paged_with_config
  :  ('key, 'cmp) B.comparator
  -> ('key, 'data, 'cmp) Gpuio.List_paging.Snapshot.t B.t
  -> paging:Paging.t B.t
  -> row_key:('key -> Gpuio.Key.t)
  -> config:Config.t B.t
  -> ?key:Gpuio.Key.t
  -> ?style:Gpuio.Style.t B.t
  -> ?scrollbar:Gpuio.Scrollbar.t option B.t
  -> ?accessibility:Gpuio.Accessibility.t B.t
  -> ?on_tree_input:('key Gpuio.Tree_input.t -> unit Bonsai.Effect.t) B.t
  -> ?input:'key Input.t B.t
  -> ?tree_moves:bool B.t
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
