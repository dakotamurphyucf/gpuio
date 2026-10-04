open Core
module Field = Accessibility.Field

(** Application-owned validation and ordinary view composition. No controller,
    validation runtime, asynchronous work, or editor replacement is introduced. *)
module Layout : sig
  type t =
    | Vertical
    | Horizontal
  [@@deriving equal, sexp_of]
end

(** Display a label, the supplied native control, and optional help/error text.
    Apply semantic associations directly to [control]. Stable internal keys keep
    the control mounted when help/errors appear or disappear. Supported roots
    are those accepted by [View.with_accessibility] for a field.

    Defaults: vertical layout, 8px gap, muted help and red error text. Custom
    styles refine those defaults. The label is static text, not another Tab stop.
    [Horizontal] gives the label 160 logical pixels and lets content flex. *)
val field
  :  Field.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> ?layout:Layout.t
  -> ?label_style:Style.t
  -> ?help_style:Style.t
  -> ?error_style:Style.t
  -> control:'action View.t
  -> unit
  -> 'action View.t Or_error.t

(** Spacing and label typography only; controls retain their caller-owned size. *)
module Size : sig
  type t =
    | XSmall
    | Small
    | Medium
    | Large
  [@@deriving equal, sexp_of]
end

module Item : sig
  type 'action t

  (** Arbitrary content. The caller supplies semantics to interactive descendants.
      Keys are unique within a collection. [column] defaults to automatic placement.
      Unspecified layout/size/label width/indent/alignment inherit the collection.
      [label_indent] reserves width for an absent horizontal label, never hides a
      present label. [required] only draws a marker; use [of_field] for semantics.
      Label widths must be definite and nonnegative. *)
  val create
    :  key:Key.t
    -> ?column:Style.Grid_location.Axis.t
    -> ?layout:Layout.t
    -> ?size:Size.t
    -> ?label_width:Length.t
    -> ?label_indent:bool
    -> ?alignment:Style.Align.t
    -> ?style:Style.t
    -> ?label_style:Style.t
    -> ?description_style:Style.t
    -> ?error_style:Style.t
    -> ?required:bool
    -> ?label:'action View.t
    -> ?description:'action View.t
    -> ?error:'action View.t
    -> 'action View.t list
    -> 'action t Or_error.t

  (** Associate metadata directly with a supported native control. Defaults for
      visible slots come from [Field]; rich overrides change display only.
      Required status always comes from [Field]. Unsupported roots return Error.
      Pass an empty view as a slot to suppress its default visual content without
      discarding the native semantic association. *)
  val of_field
    :  Field.t
    -> key:Key.t
    -> ?column:Style.Grid_location.Axis.t
    -> ?layout:Layout.t
    -> ?size:Size.t
    -> ?label_width:Length.t
    -> ?label_indent:bool
    -> ?alignment:Style.Align.t
    -> ?style:Style.t
    -> ?label_style:Style.t
    -> ?description_style:Style.t
    -> ?error_style:Style.t
    -> ?label:'action View.t
    -> ?description:'action View.t
    -> ?error:'action View.t
    -> control:'action View.t
    -> unit
    -> 'action t Or_error.t
end

(** A native column grid of stable keyed items, followed by an optional full-width
    footer aligned to the trailing edge. Defaults: one column, vertical labels,
    Medium spacing, 160px label width, absent-label indentation, Start alignment.
    Columns are in 1..1024. Reject duplicate keys and placements extending outside
    the explicit columns (including negative-line resolution and equal-line spans).
    Changing columns/orientation/slots does not reparent an item's content.

    Root, grid, item and slot styles refine visual defaults. Base grid display and
    column count, item placement/position/direction and horizontal label width are
    structural and applied after custom styles. Use the typed arguments to change
    them. State styles remain ordinary native refinements; callers must not change
    structural fields in state styles. Removing an item unmounts it; retained
    hiding uses normal View/style visibility rules. No validation scheduler,
    native editor owner or persistence is introduced. *)
val create
  :  ?key:Key.t
  -> ?columns:int
  -> ?layout:Layout.t
  -> ?size:Size.t
  -> ?label_width:Length.t
  -> ?label_indent:bool
  -> ?alignment:Style.Align.t
  -> ?style:Style.t
  -> ?grid_style:Style.t
  -> ?label_style:Style.t
  -> ?description_style:Style.t
  -> ?error_style:Style.t
  -> ?footer_style:Style.t
  -> ?footer:'action View.t
  -> 'action Item.t list
  -> 'action View.t Or_error.t
