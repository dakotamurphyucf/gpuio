open Core

module Id : sig
  type t [@@deriving compare, equal, sexp_of]

  val of_string : string -> t Or_error.t
  val to_string : t -> string
end

module Item : sig
  type t [@@deriving equal, sexp_of]

  (** A destination may also have children. Navigation and expansion are separate
      requests, so clicking an expansion control never navigates accidentally.
      Labels and [compact_label] require 1..4096 UTF-8 bytes without NUL; compact
      fallback defaults to a bullet when no icon is supplied. Children are ordered.
      Each subtree is bounded to 4096 items, depth 16 and 256 KiB of text/IDs. *)
  val create
    :  id:Id.t
    -> label:string
    -> ?compact_label:string
    -> ?disabled:bool
    -> ?children:t list
    -> unit
    -> t Or_error.t

  val id : t -> Id.t
  val label : t -> string
  val compact_label : t -> string
  val is_disabled : t -> bool
  val children : t -> t list
end

module Group : sig
  type t [@@deriving equal, sexp_of]

  (** Group IDs occupy a separate namespace from item IDs. Optional labels have
      the same text bounds as items. Empty groups are permitted. *)
  val create : id:Id.t -> ?label:string -> Item.t list -> t Or_error.t

  val id : t -> Id.t
  val label : t -> string option
  val items : t -> Item.t list
end

module Collapse : sig
  type t =
    | Icon
    | Offcanvas
    | Never
  [@@deriving equal, sexp_of]
end

module Side : sig
  type t =
    | Left
    | Right
  [@@deriving equal, sexp_of]
end

module Request : sig
  type t =
    | Select of Id.t
    | Toggle of Id.t
    | Toggle_collapsed
  [@@deriving equal, sexp_of]
end

type t [@@deriving equal, sexp_of]

val max_items : int
val max_depth : int
val max_groups : int
val max_text_bytes : int

(** At most 128 groups and the item/text/depth bounds above across all groups.
    Reject duplicate group/item IDs, absent selection, duplicate expansion IDs and
    expansion of absent/leaf items. Disabled/hidden destinations may remain selected.
    [Never] ignores the requested collapse state without discarding the preference.
    No route payload, asynchronous task or native resource is owned by this model. *)
val create
  :  groups:Group.t list
  -> selected:Id.t option
  -> ?expanded:Id.t list
  -> ?collapse:Collapse.t
  -> ?collapsed:bool
  -> ?disabled:bool
  -> unit
  -> t Or_error.t

val groups : t -> Group.t list
val selected : t -> Id.t option
val expanded : t -> Id.t list
val collapse : t -> Collapse.t
val is_collapsed : t -> bool
val is_disabled : t -> bool
val is_expanded : t -> Id.t -> bool
val is_visible : t -> Id.t -> bool
val find : t -> Id.t -> Item.t option
val with_collapsed : t -> bool -> t
val with_collapse : t -> Collapse.t -> t
val with_disabled : t -> bool -> t
val select : t -> Id.t option -> t Or_error.t

(** Preserve surviving selection and expansion; clear missing selection and drop
    absent/now-leaf expansion. Descendant expansion survives ancestor collapse. *)
val with_groups : t -> Group.t list -> t Or_error.t

(** Reduce against the latest model. Stale, hidden, disabled and leaf-toggle
    requests do nothing. Selection does not implicitly change expansion. Programmatic
    [select]/[with_collapsed] remain available while user requests are disabled.
    This governs built-in requests; custom slots/commands own their enabled policy. *)
val apply_request : t -> Request.t -> t

module Labels : sig
  type t

  val english : t

  val create
    :  navigation:string
    -> current:string
    -> expand_sidebar:string
    -> collapse_sidebar:string
    -> toggle_item:(label:string -> expanded:bool -> string)
    -> t Or_error.t
end

module Motion : sig
  type t

  (** Native width transitions, with no callback per frame. The first mount is
      immediate; updates start at the last painted width. Duration defaults to
      200 ms, is nonnegative and at most 10 seconds. Reduced motion settles the
      current target. This does not control application selection or lifetimes. *)
  val create
    :  ?duration:Time_ns.Span.t
    -> ?easing:Animation.Easing.t
    -> unit
    -> t Or_error.t

  val default : t
  val immediate : t
end

module Appearance : sig
  type t

  (** Widths are finite logical pixels, 32..4096; compact width <= expanded width.
      Existing theme tokens supply colors. Styles refine sidebar/item/current/group
      boxes. Width arguments own the sidebar's geometry over style width/shrink
      overrides. Item styles preserve the native state-style vocabulary. *)
  val create
    :  ?width:float
    -> ?compact_width:float
    -> ?motion:Motion.t
    -> ?style:Style.t
    -> ?item_style:Style.t
    -> ?current_style:Style.t
    -> ?group_style:Style.t
    -> unit
    -> t Or_error.t

  val default : t
end

module Decoration : sig
  type 'action t

  (** Slots are ordinary views; icons use existing scoped SVG registrations and
      context menus resolve commands from the enclosing command scope. Suffix is
      hidden in icon mode. Decoration callbacks run only during OCaml construction. *)
  val create
    :  ?icon:Icon.Decoration.t
    -> ?suffix:'action View.t
    -> ?context_menu:Menu.t
    -> unit
    -> 'action t
end

(** An explicit external collapse control, placed in the application toolbar.
    It uses the same request reducer; [Never] and disabled state make it inert. *)
val toggle
  :  t
  -> ?key:Key.t
  -> ?style:Style.t
  -> ?labels:Labels.t
  -> on_request:(Request.t -> 'action)
  -> unit
  -> 'action View.t

(** Native links, independent expansion buttons and retained disclosure regions.
    [hidden] applies to nested/offcanvas content; it does not deactivate Bonsai or
    cancel Eio tasks. Icon mode keeps top-level links and their tooltip names,
    hides group headings/suffixes/nested contents, and preserves link identity.
    Header/footer receive the effective compact state and own their own responsive
    presentation. [Side] determines the inner border; place the view accordingly.
    Default keyboard behavior is ordinary Tab/Shift-Tab and native activation.

    Invalid dynamic toggle labels return an error. Current and expanded state stay
    application-owned. Replacing decoration wrappers (such as adding a context menu)
    may replace their native descendants; ordinary collapse/selection does not.
    A stable native wrapper animates allocated width while content uses its target
    width (no text reflow per frame). Retained offcanvas contents slide out while
    becoming inert immediately: no focus/input/accessibility or nested animation
    work. Their native resources survive, clipped at zero width after completion.
    [Unmount] removes children immediately; only allocation animates in that mode.
    Neither mode delays removal when the sidebar itself is unmounted. *)
val view
  :  t
  -> ?key:Key.t
  -> ?side:Side.t
  -> ?appearance:Appearance.t
  -> ?labels:Labels.t
  -> hidden:Content_policy.t
  -> ?header:(compact:bool -> 'action View.t)
  -> ?footer:(compact:bool -> 'action View.t)
  -> ?decorate:(Item.t -> 'action Decoration.t)
  -> on_request:(Request.t -> 'action)
  -> unit
  -> 'action View.t Or_error.t
