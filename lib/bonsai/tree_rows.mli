open Core
module B = Bonsai.Cont
module Rows = Gpuio.Tree_rows
module Loading_model = Gpuio.Tree_loading

module Loading : sig
  type t

  (** Handlers must check the supplied token against their current loader on
      delivery. Request/retry must respect queue capacity; stale/closed delivery
      is ignored. The Eio loader's [controls] supplies this implementation. *)
  val create
    :  request:(Loading_model.Target.t -> unit Bonsai.Effect.t)
    -> retry:(Loading_model.Target.t -> unit Bonsai.Effect.t)
    -> cancel:(Loading_model.Target.t -> unit Bonsai.Effect.t)
    -> cancel_hidden:(Loading_model.Lease.t -> Gpuio.Tree_state.t -> unit Bonsai.Effect.t)
    -> t

  val request : t -> Loading_model.Target.t -> unit Bonsai.Effect.t
  val retry : t -> Loading_model.Target.t -> unit Bonsai.Effect.t
  val cancel : t -> Loading_model.Target.t -> unit Bonsai.Effect.t

  val cancel_hidden
    :  t
    -> Loading_model.Lease.t
    -> Gpuio.Tree_state.t
    -> unit Bonsai.Effect.t
end

module Controller : sig
  (** Belongs to one mounted source generation. Targets are captured by their
      current identity when the effect is constructed, then rechecked at delivery.
      Absent/hidden/reincarnated items and inactive generations ignore commands.
      These operations do not mutate expansion or selection preferences. *)
  type t

  (** Reveals an already visible item; expanding ancestors is a higher-level
      tree operation. This scroll request does not claim OS keyboard focus. *)
  val reveal : t -> Rows.Key.t -> unit Bonsai.Effect.t

  (** Request/retry requires a current visible expanded branch. Cancel may target
      any current branch. Build a lightweight token with [Snapshot.target]. *)
  val request : t -> Loading_model.Target.t -> unit Bonsai.Effect.t

  val retry : t -> Loading_model.Target.t -> unit Bonsai.Effect.t
  val cancel : t -> Loading_model.Target.t -> unit Bonsai.Effect.t
end

module Output : sig
  type 'data t

  val view : _ t -> unit Bonsai.Effect.t Gpuio.View.t
  val projection : 'data t -> 'data Rows.t
  val controller : _ t -> Controller.t
  val viewport : _ t -> Gpuio.Virtual_list.Viewport.t option
  val active_rows : _ t -> int
  val budget_exhausted : _ t -> bool
end

(** Managed row primitive for tree adapters. This is not yet the native tree
    widget: renderers provide row presentation and actions; keyboard traversal
    and OS focus belong to the higher-level adapter. [accessibility] decorates the
    actual native list root, so [Role.Tree multiple] can give it tree semantics.
    A renderer can annotate its container with [Rows.Item.accessibility]; the
    native adapter exposes one TreeItem per row, with no duplicate list item.

    Holds one accepted incremental projection; source/preference changes that
    coalesce before display are compared with that accepted baseline. A different
    loader instance or reset generation resets mounted rows even when numeric
    source generations coincide. Provide fresh preferences for a fresh source.
    Selection is never converted into retention pins. [pinned] counts toward the
    shared active budget; otherwise only viewport/native interaction demand mounts
    row computations. [render_row] follows [Managed_rows.assoc]'s reset contract.

    With [loading], visible Ready boundaries request pages after display, limited
    by available queue slots and [config]'s active budget. Failed boundaries never
    retry automatically. [auto_load=false] stops new automatic requests without
    cancelling accepted data work. Collapse cancels hidden work by default;
    [cancel_hidden=false] permits deliberate background prefetch. Unmount retires
    row/controller effects but does not close the application-owned data loader.

    Supply bounded viewport geometry through [style] or the parent. No callbacks
    enter OCaml synchronously from native layout and no I/O occurs in evaluation. *)
val component
  :  'data Loading_model.Snapshot.t B.t
  -> state:Gpuio.Tree_state.t B.t
  -> config:Gpuio.Virtual_list.Config.t
  -> ?key:Gpuio.Key.t
  -> ?style:Gpuio.Style.t B.t
  -> ?accessibility:Gpuio.Accessibility.t B.t
  -> ?pinned:Gpuio.Tree.Id.t list B.t
  -> ?loading:Loading.t B.t
  -> ?auto_load:bool B.t
  -> ?cancel_hidden:bool B.t
  -> render_row:
       (key:Rows.Key.t B.t
        -> data:'data Rows.Row.t B.t
        -> lifetime:Managed_rows.Lifetime.t B.t
        -> B.graph
        -> unit Bonsai.Effect.t Gpuio.View.t B.t)
  -> B.graph
  -> 'data Output.t Or_error.t B.t
