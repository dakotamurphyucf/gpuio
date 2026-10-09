open Core

(** Application-owned workflow navigation, distinct from numeric step buttons.
    This model does not validate forms, load pages or own their lifetimes. *)
module Status : sig
  type t =
    | Completed
    | Current
    | Upcoming
  [@@deriving equal, sexp_of]
end

module Request : sig
  type t =
    | Select of Choice.Id.t
    | Previous
    | Next
  [@@deriving equal, sexp_of]
end

type t [@@deriving equal, sexp_of]

val max_steps : int

(** At most 64 ordered steps with unique stable IDs and nonblank labels of at most
    1024 UTF-8 bytes (the native rich-button name limit). Empty models and absent
    current steps are valid. A current ID must exist, even if disabled. Status
    is positional: steps before current are Completed, not a claim of business
    validation. Without current, every step is Upcoming. *)
val create
  :  steps:Choice.Collection.t
  -> current:Choice.Id.t option
  -> ?disabled:bool
  -> unit
  -> t Or_error.t

val steps : t -> Choice.Collection.t
val current : t -> Choice.Id.t option
val status : t -> Choice.Id.t -> Status.t option
val is_disabled : t -> bool
val with_disabled : t -> bool -> t

(** Reordering preserves current identity; removing it clears current. *)
val with_steps : t -> Choice.Collection.t -> t Or_error.t

(** Explicit selection may choose a disabled step; nonexistent IDs fail. *)
val select : t -> Choice.Id.t option -> t Or_error.t

(** Apply requests against the latest application model. Disabled/removed targets
    and a disabled whole model are no-ops. Relative requests skip disabled steps,
    never wrap, and use latest current. Without current, Next chooses the first
    enabled step and Previous the last. No form validation is implicit. *)
val apply_request : t -> Request.t -> t

module Axis : sig
  type t =
    | Horizontal
    | Vertical
  [@@deriving equal, sexp_of]
end

module Appearance : sig
  type t

  val default : t

  (** Finite logical pixels: size/thickness in (0,4096], gap in [0,4096].
      Defaults: 28, 2, 8. Ordinary styles refine the item, indicator and connector.
      Status styles refine the indicator; completed connectors use the accent
      token. Root/item styles can set fonts, padding and native focus/hover states.
      Layout overrides are caller-owned and can change the standard geometry. *)
  val create
    :  ?indicator_size:float
    -> ?connector_thickness:float
    -> ?gap:float
    -> ?item_style:Style.t
    -> ?indicator_style:Style.t
    -> ?completed_style:Style.t
    -> ?current_style:Style.t
    -> ?upcoming_style:Style.t
    -> ?connector_style:Style.t
    -> unit
    -> t Or_error.t
end

module Labels : sig
  type t

  val english : t

  (** Pure OCaml formatters, called for at most 64 displayed steps. [index] is
      one-based. Descriptions should express position and status. Results pass
      accessibility text validation on each [view] call, including when disabled.
      These callbacks never cross FFI. *)
  val create : description:(index:int -> count:int -> Status.t -> string) -> t
end

(** One labelled navigation region, stable keyed native rich buttons and decorative
    connectors. Tab/Shift-Tab, Enter/Space and AX activation use ordinary button
    behavior; disabled steps are skipped. Current is independent of focus, with
    native Current.Step metadata and a localized description.

    Horizontal is the default; [centered] centers labels under indicators (default
    false). Vertical places indicators beside labels. Custom [indicator]/[content]
    are passive ordinary views, subject to rich-button depth/node/interaction
    limits. Defaults are step numbers and Choice labels. A custom icon can be
    returned from [indicator]; interactive/natively owned descendants are rejected.
    [on_request] carries identity, never a captured index. Feed it to the latest
    model's reducer, then decide whether/how to display associated content. *)
val view
  :  t
  -> ?key:Key.t
  -> ?style:Style.t
  -> ?axis:Axis.t
  -> ?centered:bool
  -> ?appearance:Appearance.t
  -> ?labels:Labels.t
  -> ?indicator:(Choice.t -> Status.t -> 'action View.t)
  -> ?content:(Choice.t -> Status.t -> 'action View.t)
  -> label:string
  -> on_request:(Request.t -> 'action)
  -> unit
  -> 'action View.t Or_error.t
