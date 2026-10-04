open Core

(** Stateless navigation compositions. Models, route resolution and data loading
    stay in the application; callbacks enqueue actions through ordinary native
    buttons. These helpers allocate no native navigation history or timers. *)
module Appearance : sig
  type t

  (** Uses the existing background/foreground/accent/muted theme tokens. Overrides
      refine ordinary item, current item, gap and separator styles, respectively.
      Native hover/pressed/focus/disabled styles remain available. *)
  val default : t

  val create
    :  ?item_style:Style.t
    -> ?current_style:Style.t
    -> ?gap_style:Style.t
    -> ?separator_style:Style.t
    -> unit
    -> t
end

(** Ordered path with stable [Choice.Id] keys. The final member is current, a
    labelled text node rather than an action. Earlier members are native links;
    disabled members are inert and omitted from Tab traversal. Empty paths are
    valid. The existing collection bounds apply. Separators are decorative.
    Labels/descriptions require 1..4096 UTF-8 bytes without NUL.

    [on_navigate] receives identity, never a captured route payload or index;
    resolve it against current application state before navigating.

    [is_navigable] defaults to true for earlier items; false renders passive text
    without an action or disabled state. The final item always remains passive.
    [item_style] is a pure per-member style override applied after Appearance.
    These callbacks run only during OCaml view construction. *)
val breadcrumbs
  :  Choice.Collection.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> ?appearance:Appearance.t
  -> label:string
  -> current_description:string
  -> ?is_navigable:(Choice.t -> bool)
  -> ?item_style:(Choice.t -> Style.t)
  -> on_navigate:(Choice.Id.t -> 'action)
  -> unit
  -> 'action View.t Or_error.t

module Pagination_labels : sig
  type t

  val english : t

  (** Strings follow the same bounds as breadcrumbs. [page] and [gap] are pure
      OCaml formatting functions called only for the bounded rendered items;
      their results are validated on each construction. They never cross FFI.
      [current] is the accessible description of the current page. *)
  val create
    :  navigation:string
    -> first:string
    -> previous:string
    -> next:string
    -> last:string
    -> current:string
    -> page:(int -> string)
    -> gap:(first:int -> last:int -> string)
    -> t Or_error.t
end

module Pagination_layout : sig
  type t =
    | Full
    | Compact
  [@@deriving equal, sexp_of]
end

(** A popup attached to the gap's own native button. Content is controlled by the
    application; [None] keeps the closed anchor mounted. [style] styles its panel. *)
module Gap_popup : sig
  type 'action t

  val create
    :  ?style:Style.t
    -> config:Overlay.Config.t
    -> on_dismiss:(Overlay.Dismissal.t -> 'action)
    -> 'action View.t option
    -> 'action t
end

(** At most 17 children regardless of total page count: four boundary controls
    and at most 13 pages/gaps. Current pages remain actionable native buttons
    with distinct current-page metadata, readable description and visual style.
    Gaps have range descriptions and are passive unless [on_gap] is supplied. Boundary controls
    are disabled when their request cannot move; all controls are disabled when
    the model is disabled. Uses Tab/Shift-Tab and native Enter/Space/AX activation.

    Apply [on_request] to the latest model with [Pagination.apply_request]. Relative
    requests remain relative across queued activations; stale page requests after
    shrink are harmless. Stable page keys preserve native identity across updates.
    The current page is independent of focus. No data is fetched by this helper.

    [Compact] renders only previous/next arrow buttons with localized accessible
    labels. It preserves their identities and never calls page/gap formatters.
    [on_gap] makes full-layout ellipses buttons and receives the inclusive interval
    from that rendered model. Revalidate it against current state before opening
    application content; never enumerate an unbounded page interval.

    [gap_popup] requires [on_gap], otherwise construction returns an error. In
    full layout it is called once per rendered gap, including while disabled.
    Each gap's native button remains the direct popup anchor, with stable identity
    across open/close updates, native expanded state and focus restoration.
    Disabled models suppress popup content. Compact invokes neither callback
    builder nor page/gap formatter. Popup content is additional to the root's
    bounded children; the caller must keep that content bounded as well. *)
val pagination
  :  Pagination.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> ?appearance:Appearance.t
  -> ?labels:Pagination_labels.t
  -> ?layout:Pagination_layout.t
  -> ?on_gap:(first:int -> last:int -> 'action)
  -> ?gap_popup:(first:int -> last:int -> 'action Gap_popup.t)
  -> on_request:(Pagination.Request.t -> 'action)
  -> unit
  -> 'action View.t Or_error.t
