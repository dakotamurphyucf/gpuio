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
    resolve it against current application state before navigating. *)
val breadcrumbs
  :  Choice.Collection.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> ?appearance:Appearance.t
  -> label:string
  -> current_description:string
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

(** At most 17 children regardless of total page count: four boundary controls
    and at most 13 pages/gaps. Current pages remain actionable native buttons
    with distinct current-page metadata, readable description and visual style.
    Gaps have range descriptions and never register handlers. Boundary controls
    are disabled when their request cannot move; all controls are disabled when
    the model is disabled. Uses Tab/Shift-Tab and native Enter/Space/AX activation.

    Apply [on_request] to the latest model with [Pagination.apply_request]. Relative
    requests remain relative across queued activations; stale page requests after
    shrink are harmless. Stable page keys preserve native identity across updates.
    The current page is independent of focus. No data is fetched by this helper. *)
val pagination
  :  Pagination.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> ?appearance:Appearance.t
  -> ?labels:Pagination_labels.t
  -> on_request:(Pagination.Request.t -> 'action)
  -> unit
  -> 'action View.t Or_error.t
