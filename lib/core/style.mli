open Core

module Display : sig
  type t = Block | Flex | Grid | Hidden [@@deriving equal, sexp_of]
end

module Visibility : sig
  type t = Visible | Hidden [@@deriving equal, sexp_of]
end

module Direction : sig
  type t = Row | Column | Row_reverse | Column_reverse [@@deriving equal, sexp_of]
end

module Wrap : sig
  type t = No_wrap | Wrap | Wrap_reverse [@@deriving equal, sexp_of]
end

module Align : sig
  type t = Start | End | Flex_start | Flex_end | Center | Baseline | Stretch [@@deriving equal, sexp_of]
end

module Distribution : sig
  type t = Start | End | Flex_start | Flex_end | Center | Stretch | Space_between | Space_evenly | Space_around [@@deriving equal, sexp_of]
end

module Grid_minimum : sig
  type t = Zero | Min_content | Max_content [@@deriving equal, sexp_of]
end

module Position : sig
  type t = Relative | Absolute [@@deriving equal, sexp_of]
end

module Text_align : sig
  type t = Left | Center | Right [@@deriving equal, sexp_of]
end

module White_space : sig
  type t = Normal | No_wrap [@@deriving equal, sexp_of]
end

module Text_overflow : sig
  type t = Clip | Ellipsis [@@deriving equal, sexp_of]
end

module Text_decoration : sig
  type t = None | Underline | Strikethrough | Underline_and_strikethrough [@@deriving equal, sexp_of]
end

module Overflow : sig
  type t = Visible | Clip | Hidden | Scroll [@@deriving equal, sexp_of]
end

module Cursor : sig
  type t = Arrow | Ibeam | Pointer | Crosshair | Move | Not_allowed | Resize_horizontal | Resize_vertical | Grab | Grabbing [@@deriving equal, sexp_of]
end

module State : sig
  type t = Base | Focused | Hovered | Pressed [@@deriving equal, sexp_of]
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
    | Pointer_events of bool
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
      | Pointer_events
    [@@deriving compare, equal, sexp_of]
  end
end

(** Last property wins; shorthands expand to individual sides before composition.
    An unset refinement restores GPUI defaults/inheritance, including when merged
    over a component default. Native precedence is base < focused < hovered < pressed. *)
type t [@@deriving equal, sexp_of]
val empty : t
val create : Property.t list -> t Or_error.t
val create_exn : Property.t list -> t
val merge : t list -> t
val unset : t -> ?state:State.t -> Property.Name.t -> t
val with_state : t -> State.t -> Property.t list -> t Or_error.t
val with_state_exn : t -> State.t -> Property.t list -> t
module Expert : sig
  val to_wire : t -> theme:Theme.t -> Gpuio_protocol.Wire.Style.t list Or_error.t
end
