open Core
module B = Bonsai.Cont
module Target = Gpuio.Tree_interaction.Target

module Controller : sig
  (** One mounted source generation, holding no payload-bearing snapshot. Capture
      a target with [Output.target] before constructing delayed commands. Removed,
      reincarnated or foreign targets and retired mounts ignore commands. *)
  type t

  val dispatch : t -> Gpuio.Tree_interaction.Request.t -> unit Bonsai.Effect.t

  val select
    :  t
    -> ?gesture:Gpuio.Tree_state.Selection.t
    -> Target.t
    -> unit Bonsai.Effect.t

  val set_selected : t -> Target.t -> bool -> unit Bonsai.Effect.t
  val set_expanded : t -> Target.t -> bool -> unit Bonsai.Effect.t
  val toggle_expanded : t -> Target.t -> unit Bonsai.Effect.t

  (** Opens loaded ancestors, then issues native reveal after the new projection
      is displayed. With [focus=true], the same row receives focus once mounted.
      Later commands supersede pending reveal. Never loads an unknown path. *)
  val reveal : t -> ?focus:bool -> Target.t -> unit Bonsai.Effect.t

  val activate : t -> Target.t -> unit Bonsai.Effect.t

  (** Keyboard/menu alternative to native drag. [allow_moves] controls native
      dragging only; explicit proposals still validate both current endpoints.
      [on_action] owns approval; recheck [Tree_interaction.Move.is_current] before
      applying a delayed approval. No hierarchy is mutated by this command. *)
  val propose_move
    :  t
    -> source:Target.t
    -> destination:Target.t
    -> Gpuio.Tree_interaction.Placement.t
    -> unit Bonsai.Effect.t

  (** Requests/retries require a visible expanded branch and [loading] controls.
      Cancel allows any current branch. Data work belongs to the application's
      loading scope; unmount does not close that scope. *)
  val request : t -> Target.t -> unit Bonsai.Effect.t

  val retry : t -> Target.t -> unit Bonsai.Effect.t
  val cancel : t -> Target.t -> unit Bonsai.Effect.t
end

module Output : sig
  type 'data t

  val view : _ t -> unit Bonsai.Effect.t Gpuio.View.t
  val state : _ t -> Gpuio.Tree_state.t
  val controller : _ t -> Controller.t

  (** Captures identity, not payloads. Hidden loaded targets may be revealed;
      an absent ID is an error. Holding the output itself retains its snapshot. *)
  val target : _ t -> Gpuio.Tree.Id.t -> Target.t Or_error.t

  val projection : 'data t -> 'data Gpuio.Tree_rows.t
  val viewport : _ t -> Gpuio.Virtual_list.Viewport.t option
  val active_rows : _ t -> int
  val budget_exhausted : _ t -> bool
end

(** A managed tree with component-owned preferences and native interaction.
    The application owns [source] and optional Eio [loading] controls. Native
    requests reduce in order against current data/state. Selection never pins
    rows. Only activation and application-approved move proposals reach
    [on_action]; reducing a Move never changes application data.

    [allow_moves=false] by default. Opting in enables single-row, same-window,
    same-tree native drag proposals. The top/bottom quarter of a branch requests
    Before/After; its middle requests Inside. Leaves split at the midpoint.
    The application handles Move in [on_action] and supplies the new snapshot
    after approval. Invalid/self/descendant or stale endpoints are ignored.
    Row eviction/collapse, source disable/hiding, handler retirement, Escape,
    window deactivation/close cancel a gesture. Existing native focus pins share
    the active-row budget; dragging adds no separate pin policy and does not
    expand/load a hovered branch automatically. Provide a keyboard/menu move
    alternative with [Controller.propose_move].

    [initial_selected]/[initial_expanded] seed each mounted source generation.
    Later seed changes do not overwrite live preferences; use controllers for
    commands and observe [Output.state] to persist preferences outside the widget.
    Invalid initial IDs return Error. Mode changes reconcile selection using
    [Tree_state.with_mode]. Source replacement/reset retires controllers, effects
    and transient rows and starts fresh preferences. A component unmount likewise
    discards its preferences; persist deliberate preferences in application state.

    [render_item] customizes content inside the themed, indented, accessible row.
    The widget owns selection paint and disclosure; keyboard/AX operate on the
    single native TreeItem. Custom child controls retain their own focus/input.
    The renderer follows [Managed_rows.assoc]'s lifetime/reset rules. The default
    displays the node label. Native loading/retry rows never become tree items.
    Default colors use the background/foreground/accent/muted theme tokens.

    With loading and [auto_load=true], newly expanded eligible branches request
    their first ready page even if its boundary is just below the viewport.
    Admission is bounded by available queue slots and [config]'s active budget;
    viewport demand handles later pages. Failed pages require explicit retry.

    Bounds, cache and fixed/estimated row heights follow [config]. Give the tree
    bounded geometry through [style] or its parent. No synchronous OCaml callback
    runs during native layout/paint, and no I/O runs during Bonsai evaluation. *)
val component
  :  'data Gpuio.Tree_loading.Snapshot.t B.t
  -> config:Gpuio.Virtual_list.Config.t
  -> label:string
  -> ?key:Gpuio.Key.t
  -> ?style:Gpuio.Style.t B.t
  -> ?mode:Gpuio.Tree_state.Mode.t B.t
  -> ?initial_selected:Gpuio.Tree.Id.t list B.t
  -> ?initial_expanded:Gpuio.Tree.Id.t list B.t
  -> ?loading:Tree_rows.Loading.t B.t
  -> ?auto_load:bool B.t
  -> ?cancel_hidden:bool B.t
  -> ?allow_moves:bool B.t
  -> ?on_action:(Gpuio.Tree_interaction.Action.t -> unit Bonsai.Effect.t) B.t
  -> ?render_item:
       (target:Target.t B.t
        -> item:'data Gpuio.Tree_rows.Item.t B.t
        -> controller:Controller.t B.t
        -> lifetime:Managed_rows.Lifetime.t B.t
        -> B.graph
        -> unit Bonsai.Effect.t Gpuio.View.t B.t)
  -> B.graph
  -> 'data Output.t Or_error.t B.t
