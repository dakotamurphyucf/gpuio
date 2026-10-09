open Core
module B = Bonsai.Cont

module Interaction : sig
  (** Query/policy identity is distinct from the selected set and cursor. Change
      [epoch] when accepting a new search query. Busy updates preserve queued
      input; disabled/mode/navigation/boundary changes retire captured input. *)
  type t

  val create
    :  epoch:Gpuio.Key.t
    -> ?mode:Gpuio.List_selection.Mode.t
    -> ?boundary:Gpuio.List_selection.Boundary.t
    -> ?selection_on_navigation:bool
    -> ?disabled:bool
    -> ?busy:bool
    -> unit
    -> t
end

module Action : sig
  (** Intent targets retain membership identity, never a payload snapshot.
      Confirmation/context do not implicitly change committed selection. *)
  type 'key t =
    | Confirm of
        'key Gpuio.List_collection.Item_ref.t * Gpuio.List_selection.Confirmation.t
    | Context of 'key Gpuio.List_collection.Item_ref.t
    | Cancel
  [@@deriving sexp_of]
end

module Controller : sig
  (** Commands use the current catalog. They ignore obsolete/foreign references,
      disabled interaction and an inactive source lifetime. Filtering preserves
      the controller; source replacement or unmount retires it permanently. *)
  type 'key t

  val focus : 'key t -> 'key Gpuio.List_collection.Item_ref.t -> unit Bonsai.Effect.t

  val select
    :  'key t
    -> 'key Gpuio.List_collection.Item_ref.t
    -> Gpuio.List_selection.Gesture.t
    -> unit Bonsai.Effect.t

  val set_selected
    :  'key t
    -> 'key Gpuio.List_collection.Item_ref.t
    -> bool
    -> unit Bonsai.Effect.t

  val navigate
    :  _ t
    -> ?selection:Gpuio.List_selection.Gesture.t
    -> Gpuio.List_selection.Navigation.t
    -> unit Bonsai.Effect.t

  val confirm
    :  'key t
    -> ?target:'key Gpuio.List_collection.Item_ref.t
    -> Gpuio.List_selection.Confirmation.t
    -> unit Bonsai.Effect.t

  val context
    :  'key t
    -> ?target:'key Gpuio.List_collection.Item_ref.t
    -> unit
    -> unit Bonsai.Effect.t

  val cancel : _ t -> unit Bonsai.Effect.t
  val clear_selection : _ t -> unit Bonsai.Effect.t
end

module Row : sig
  (** Computed only for the bounded mounted subset. Decoration rows are never
      options; their content can include independently focusable controls. *)
  type ('key, 'data) t

  val item : ('key, 'data) t -> ('key, 'data) Gpuio.List_rows.Item.t
  val is_selected : (_, _) t -> bool
  val is_cursor : (_, _) t -> bool
end

module Output : sig
  type ('key, 'data, 'cmp) t

  val view : (_, _, _) t -> unit Bonsai.Effect.t Gpuio.View.t
  val state : ('key, _, 'cmp) t -> ('key, 'cmp) Gpuio.List_selection.t
  val controller : ('key, _, _) t -> 'key Controller.t
  val target : ('key, _, 'cmp) t -> 'key -> 'key Gpuio.List_collection.Item_ref.t option
  val viewport : (_, _, _) t -> Gpuio.Virtual_list.Viewport.t option
  val active_rows : (_, _, _) t -> int
  val budget_exhausted : (_, _, _) t -> bool
end

(** A source-scoped selection owner over the bounded managed list. Reuse [layout]
    until source membership/order or query visibility/eligibility changes. Point
    payload updates use persistent-map sharing. Selection/cursor changes touch
    mounted row computations, never create computations for all loaded records.

    [initial_selected] seeds each independent source once; later edits to the seed
    do not overwrite user selection. Hidden committed selections survive filters.
    Query key must name one direct Input in [before]/[after], in either order.
    The query retains editing/IME ownership. Empty/loading/footer content can use
    these slots without replacing the list owner. Query tasks belong outside rows.

    The component supplies ListBox/Option semantics and themed row presentation.
    [render_row] customizes each row's content; [row_style] can override its outer
    presentation. Label callbacks must supply a valid nonempty accessible label
    for each option. Decorations preserve content semantics. Row models follow
    Managed_rows' reset/lifetime contract. The viewport needs a bounded extent.
    Both axes are supported; changing [config] preserves surviving row models.

    [scrollbar] configures the actual native viewport, inside any layout
    wrappers. It defaults to [B.return None], which retains legacy native
    presentation. Changing it preserves the viewport and mounted row state;
    [config]'s scrollbar enable flag still takes precedence. *)
val component
  :  ('key, 'data, 'cmp) Gpuio.List_collection.t B.t
  -> layout:('key, 'cmp) Gpuio.List_rows.Layout.t B.t
  -> config:Gpuio.Virtual_list.Config.t B.t
  -> interaction:Interaction.t B.t
  -> label:string
  -> item_label:(key:'key -> 'data -> string)
  -> ?key:Gpuio.Key.t
  -> ?style:Gpuio.Style.t B.t
  -> ?scrollbar:Gpuio.Scrollbar.t option B.t
  -> ?initial_selected:'key list B.t
  -> ?query:Gpuio.Key.t B.t
  -> ?before:unit Bonsai.Effect.t Gpuio.View.t list B.t
  -> ?after:unit Bonsai.Effect.t Gpuio.View.t list B.t
  -> ?on_action:('key Action.t -> unit Bonsai.Effect.t) B.t
  -> ?row_style:(('key, 'data) Row.t -> Gpuio.Style.t)
  -> ?render_row:
       (row:('key, 'data) Row.t B.t
        -> controller:'key Controller.t B.t
        -> lifetime:Managed_rows.Lifetime.t B.t
        -> B.graph
        -> unit Bonsai.Effect.t Gpuio.View.t B.t)
  -> B.graph
  -> ('key, 'data, 'cmp) Output.t Or_error.t B.t
