open Core

module Part : sig
  type t =
    | Header_background
    | Header_foreground
    | Stripe_background
    | Hover_background
    | Selected_background
    | Selected_border
    | Row_border
    | Column_border
    | Sort_hover_background
    | Sort_pressed_background
    | Sort_foreground
    | Drag_border
    | Context_border
  [@@deriving compare, equal, sexp_of]
end

module Padding : sig
  (** Native leaf-header/body cell padding, in logical pixels. Each edge is finite
      in 0..4096. Large padding can consume the available content area; cells
      retain their configured dimensions and clip content. *)
  type t [@@deriving equal, sexp_of]

  val create : top:float -> right:float -> bottom:float -> left:float -> t Or_error.t
  val all : float -> t Or_error.t
  val zero : t
end

(** Per-table native presentation, with no callbacks or native owners. Root
    background/border/text still use ordinary View styles. Header colors also
    apply to grouped headers and row-header gutters. Stripe parity follows logical
    row order, including native filler rows below a short dataset. *)
type t [@@deriving equal, sexp_of]

(** Defaults: unstriped, inherited colors/padding. Omitted colors inherit the
    native/root-style-derived value; transparent colors are explicit overrides.
    Colors resolve through the submission theme, even for currently hidden parts.
    Duplicate parts and duplicate column IDs reject. At most 64 padding overrides;
    [Table.Config] additionally requires their IDs in the current schema.
    Per-column padding replaces shared padding, then the native default (4 vertical,
    8 horizontal). Padding updates retire old geometry-dependent input; color and
    stripe updates preserve geometry, selection, scroll handles and cell models. *)
val create
  :  ?striped:bool
  -> ?colors:(Part.t * Color.t) list
  -> ?padding:Padding.t
  -> ?column_padding:(Table_column.Id.t * Padding.t) list
  -> unit
  -> t Or_error.t

val default : t
val is_striped : t -> bool

module Expert : sig
  val valid_columns : t -> Table_column.Collection.t -> bool
  val to_wire : t -> theme:Theme.t -> Gpuio_protocol.Table_wire.Appearance.t Or_error.t
end
