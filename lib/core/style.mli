open Core

module Display : sig
  type t =
    | Block
    | Flex
    | Grid
    | Hidden
  [@@deriving equal, sexp_of]
end

module Visibility : sig
  type t =
    | Visible
    | Hidden
  [@@deriving equal, sexp_of]
end

module Direction : sig
  type t =
    | Row
    | Column
    | Row_reverse
    | Column_reverse
  [@@deriving equal, sexp_of]
end

module Wrap : sig
  type t =
    | No_wrap
    | Wrap
    | Wrap_reverse
  [@@deriving equal, sexp_of]
end

module Align : sig
  type t =
    | Start
    | End
    | Flex_start
    | Flex_end
    | Center
    | Baseline
    | Stretch
  [@@deriving equal, sexp_of]
end

module Distribution : sig
  type t =
    | Start
    | End
    | Flex_start
    | Flex_end
    | Center
    | Stretch
    | Space_between
    | Space_evenly
    | Space_around
  [@@deriving equal, sexp_of]
end

module Grid_minimum : sig
  type t =
    | Zero
    | Min_content
    | Max_content
  [@@deriving equal, sexp_of]
end

module Position : sig
  type t =
    | Relative
    | Absolute
  [@@deriving equal, sexp_of]
end

module Text_align : sig
  type t =
    | Left
    | Center
    | Right
  [@@deriving equal, sexp_of]
end

module White_space : sig
  type t =
    | Normal
    | No_wrap
  [@@deriving equal, sexp_of]
end

module Text_overflow : sig
  type t =
    | Clip
    | Ellipsis
    | Ellipsis_start
  [@@deriving equal, sexp_of]
end

module Text_decoration : sig
  type t =
    | None
    | Underline
    | Strikethrough
    | Underline_and_strikethrough
  [@@deriving equal, sexp_of]
end

(** [Scroll] enables native scrolling on the declared axis. Ordinary containers
    with both axes scrollable preserve diagonal movement; a single-axis
    container does not translate wheel input from the other axis. Consumed
    events stop at that viewport, and events at its boundary can reach an
    ancestor. A partly consumed event does not forward leftover movement.
    Managed lists and native editor/control widgets own their scroll policies. *)
module Overflow : sig
  type t =
    | Visible
    | Clip
    | Hidden
    | Scroll
  [@@deriving equal, sexp_of]
end

(** Controls native hit testing behind this element, independently of whether
    its own listeners are enabled. None uses ordinary native behavior. Pointer
    blocks pointer hitboxes behind it while allowing wheel input through;
    Pointer_and_scroll blocks both. Base style only, not inherited. This does not
    disable children or replace modal/native-control priority. *)
module Pointer_occlusion : sig
  type t =
    | None
    | Pointer
    | Pointer_and_scroll
  [@@deriving equal, sexp_of]
end

module Cursor : sig
  type t =
    | Arrow
    | Ibeam
    | Pointer
    | Crosshair
    | Move
    | Not_allowed
    | Resize_horizontal
    | Resize_vertical
    | Grab
    | Grabbing
    | Ibeam_vertical
    | Resize_column
    | Resize_row
    | Resize_nw_se
    | Resize_ne_sw
    | Resize_left
    | Resize_right
    | Resize_up
    | Resize_down
    | Alias
    | Copy
    | Context_menu
  [@@deriving equal, sexp_of]

  (** Diagonals name their physical directions: [Resize_nw_se] runs top-left to
      bottom-right. Platform cursor artwork may coincide: macOS uses the same
      glyph for [Resize_column]/[Resize_horizontal] and [Resize_row]/
      [Resize_vertical]. [Move] and [Grabbing] both use the closed-hand cursor. *)
end

module State : sig
  type t =
    | Base
    | Focused
    | Hovered
    | Pressed
    | Checked
    | Indeterminate
    | Disabled
    | Selected
  [@@deriving equal, sexp_of]
end

module Property : sig
  type t =
    | Display of Display.t
    | Visibility of Visibility.t
    | Direction of Direction.t
    | Wrap of Wrap.t
    | Grow of float
    | Shrink of float
    | Basis of Length.t
    | Align_items of Align.t
    | Align_self of Align.t
    | Align_content of Distribution.t
    | Justify_content of Distribution.t
    | Row_gap of Length.t
    | Column_gap of Length.t
    | Grid_columns of int
    | Grid_rows of int
    | Grid_column_minimum of Grid_minimum.t
    | Grid_row_minimum of Grid_minimum.t
    | Width of Length.t
    | Height of Length.t
    | Min_width of Length.t
    | Min_height of Length.t
    | Max_width of Length.t
    | Max_height of Length.t
    | Padding_top of Length.t
    | Padding_right of Length.t
    | Padding_bottom of Length.t
    | Padding_left of Length.t
    | Margin_top of Length.t
    | Margin_right of Length.t
    | Margin_bottom of Length.t
    | Margin_left of Length.t
    | Position of Position.t
    | Top of Length.t
    | Right of Length.t
    | Bottom of Length.t
    | Left of Length.t
    | Background of Background.t
    | Foreground of Color.t
    | Opacity of float
    | Border_top_width of float
    | Border_right_width of float
    | Border_bottom_width of float
    | Border_left_width of float
    | Top_left_radius of float
    | Top_right_radius of float
    | Bottom_left_radius of float
    | Bottom_right_radius of float
    | Border_color of Color.t
    | Shadows of Shadow.t list
    | Font_size of float
    | Font_family of string
    | Font_weight of int
    | Text_align of Text_align.t
    | Line_height of Length.t
    | White_space of White_space.t
    | Text_overflow of Text_overflow.t
    | Line_clamp of int
    | Text_decoration of Text_decoration.t
    | Overflow_x of Overflow.t
    | Overflow_y of Overflow.t
    | Cursor of Cursor.t
    | Pointer_occlusion of Pointer_occlusion.t
    | Pointer_events of bool
    | User_select of bool
    | Selection_color of Color.t
    | Accessible_name of string
    | Inert of bool
    (** [Inert true] retains layout and paint while excluding this subtree from
        native focus, keyboard/pointer/IME input and accessibility. Descendants
        cannot override an inert ancestor. It does not deactivate Bonsai or cancel
        application tasks. Base style only; ordinary hidden style still removes
        paint. Intended for outgoing retained visual content. *)
    | Padding of Length.t
    | Margin of Length.t
    | Gap of Length.t
    | Border_width of float
    | Radius of float
    | Overflow of Overflow.t
  [@@deriving equal, sexp_of]

  module Name : sig
    type t =
      | Display
      | Visibility
      | Direction
      | Wrap
      | Grow
      | Shrink
      | Basis
      | Align_items
      | Align_self
      | Align_content
      | Justify_content
      | Row_gap
      | Column_gap
      | Grid_columns
      | Grid_rows
      | Grid_column_minimum
      | Grid_row_minimum
      | Width
      | Height
      | Min_width
      | Min_height
      | Max_width
      | Max_height
      | Padding_top
      | Padding_right
      | Padding_bottom
      | Padding_left
      | Margin_top
      | Margin_right
      | Margin_bottom
      | Margin_left
      | Position
      | Top
      | Right
      | Bottom
      | Left
      | Background
      | Foreground
      | Opacity
      | Border_top_width
      | Border_right_width
      | Border_bottom_width
      | Border_left_width
      | Top_left_radius
      | Top_right_radius
      | Bottom_left_radius
      | Bottom_right_radius
      | Border_color
      | Shadows
      | Font_size
      | Font_family
      | Font_weight
      | Text_align
      | Line_height
      | White_space
      | Text_overflow
      | Line_clamp
      | Text_decoration
      | Overflow_x
      | Overflow_y
      | Cursor
      | Pointer_occlusion
      | Pointer_events
      | User_select
      | Selection_color
      | Accessible_name
      | Inert
    [@@deriving compare, equal, sexp_of]
  end
end

(** Last property wins; shorthands expand to individual sides before composition.
    An unset refinement restores GPUI defaults/inheritance, including when merged
    over a component default. Native precedence is base < focused < hovered < pressed. *)
type t [@@deriving equal, sexp_of]

val empty : t

(** Validate before constructing a style. Numeric values are finite and bounded:
    grow, shrink, border widths and radii are in 0..1,000,000; font size is
    strictly positive and at most 1,000,000; opacity is in 0..1. Grid counts and
    line clamp are integers in 1..1024; font weight is in 1..1000. Font family
    uses 1..256 UTF-8 bytes, accessible name 1..1024 bytes, and shadows at most
    eight entries. Invalid values are rejected rather than clamped.

    Length constructors enforce their own finite magnitude limit. Widths,
    heights and basis allow nonnegative lengths or Auto; padding, gaps and line
    height require nonnegative definite lengths; margins and offsets also allow
    negatives and Auto. Shorthands expand in list order: the last declaration
    of an individual side/axis wins, including when a later shorthand replaces
    an earlier longhand. At most 128 input declarations are accepted per call. *)
val create : Property.t list -> t Or_error.t

val create_exn : Property.t list -> t
val merge : t list -> t

(** Remove the declaration from the selected state, including an earlier value
    in that state when styles are merged. This emits no native reset command.
    Unsetting a hover/pressed declaration leaves the base declaration in effect;
    it does not force the native default while that state is active. Removing a
    base declaration restores the receiving component's defaults/inheritance. *)
val unset : t -> ?state:State.t -> Property.Name.t -> t

val with_state : t -> State.t -> Property.t list -> t Or_error.t
val with_state_exn : t -> State.t -> Property.t list -> t

module Expert : sig
  val declaration_count : t -> int

  val validate_scope
    :  t
    -> states:State.t list
    -> properties:Property.Name.t list
    -> unit Or_error.t

  val to_wire : t -> theme:Theme.t -> Gpuio_protocol.Wire.Style.t list Or_error.t
end
