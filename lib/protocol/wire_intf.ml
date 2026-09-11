open Core

module type S = sig
  val version : int64
  val capabilities : int64
  val max_message_bytes : int

  module Kind : sig
    type t =
      | Container
      | Text
      | Button
    [@@deriving bin_io, equal, sexp_of]
  end

  module Length : sig
    type t =
      | Px of float
      | Percent of float
      | Auto
    [@@deriving bin_io, equal, sexp_of]
  end

  module Color : sig
    type t =
      | Rgba of int64
      | Token of int64
    [@@deriving bin_io, equal, sexp_of]
  end

  module Fill : sig
    type t = Solid of Color.t | Linear_gradient of float * Color.t * float * Color.t * float
    [@@deriving bin_io, equal, sexp_of]
  end

  module Shadow : sig
    type t = { color:Color.t; offset_x:float; offset_y:float; blur:float; spread:float; inset:bool }
    [@@deriving bin_io, equal, sexp_of]
  end

  module Field : sig
    type t =
      | Display of int64
      | Visibility of int64
      | Direction of int64
      | Wrap of int64
      | Grow of float
      | Shrink of float
      | Basis of Length.t
      | Align_items of int64
      | Align_self of int64
      | Align_content of int64
      | Justify_content of int64
      | Row_gap of Length.t
      | Column_gap of Length.t
      | Grid_columns of int64
      | Grid_rows of int64
      | Grid_column_minimum of int64
      | Grid_row_minimum of int64
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
      | Position of int64
      | Top of Length.t
      | Right of Length.t
      | Bottom of Length.t
      | Left of Length.t
      | Background of Fill.t
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
      | Font_weight of int64
      | Text_align of int64
      | Line_height of Length.t
      | White_space of int64
      | Text_overflow of int64
      | Line_clamp of int64
      | Text_decoration of int64
      | Overflow_x of int64
      | Overflow_y of int64
      | Cursor of int64
      | Pointer_events of bool
    [@@deriving bin_io, equal, sexp_of]
  end

  module Style : sig
    type t =
      | Width of Length.t
      | Height of Length.t
      | Min_width of Length.t
      | Min_height of Length.t
      | Max_width of Length.t
      | Max_height of Length.t
      | Padding of float
      | Gap of float
      | Grow of float
      | Shrink of float
      | Direction of int64
      | Background of Color.t
      | Foreground of Color.t
      | Font_size of float
      | Radius of float
      | Opacity of float
      | Hover_background of Color.t
      | Pressed_background of Color.t
      | Focus_background of Color.t
      | Fields of Field.t list
      | State of int64 * Field.t list
    [@@deriving bin_io, equal, sexp_of]
  end

  module Op : sig
    type t =
      | Create of Node_id.t * Kind.t * string * Handler_id.t option
      | Remove of Node_id.t
      | Set_text of Node_id.t * string
      | Set_style of Node_id.t * Style.t list
      | Bind of Node_id.t * Handler_id.t option
      | Splice of Node_id.t * int64 * int64 * Node_id.t list
      | Set_root of Node_id.t option
    [@@deriving bin_io, equal, sexp_of]
  end

  module Transaction : sig
    type t =
      { window : Window_id.t
      ; base : int64
      ; revision : int64
      ; operations : Op.t list
      }
    [@@deriving bin_io, equal, sexp_of]
  end

  module Message : sig
    type t =
      | Hello of int64 * int64
      | Open of int64 * Window_id.t * string * float * float
      | Close of int64 * Window_id.t
      | Apply of Transaction.t
      | Request_frame of int64 * Window_id.t
      | Shutdown
    [@@deriving bin_io, equal, sexp_of]

    (** Bounded outgoing encoding. Native decoding additionally validates all
        domain invariants before accepting the message. *)
    val encode : t -> string Or_error.t
  end

  module Error_code : sig
    type t =
      | Unsupported_version
      | Unsupported_capability
      | Malformed
      | Limit_exceeded
      | Not_ready
      | Stale_handle
      | Invalid_revision
      | Invalid_tree
      | Busy
      | Closed
      | Overloaded
      | Native_failure
    [@@deriving bin_io, equal, sexp_of]
  end

  module Event : sig
    type t =
      | Welcome of int64 * int64
      | Opened of int64 * Window_id.t
      | Closed of int64 * Window_id.t
      | Accepted of Window_id.t * int64
      | Rejected of Window_id.t * int64 * Error_code.t
      | Rendered of Window_id.t * int64
      | Frame_requested of int64 * Window_id.t * int64
      | Press of Window_id.t * Node_id.t * Handler_id.t * int64
      | Failed of int64 * Error_code.t
      | Stopped
      | Overloaded of Window_id.t
    [@@deriving bin_io, equal, sexp_of]

    (** Decode one bounded event envelope, requiring full byte consumption and
        validating handle representations. This performs no callback dispatch. *)
    val decode : string -> t list Or_error.t
  end
end
