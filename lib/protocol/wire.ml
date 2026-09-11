open Core

let version = 1L
let capabilities = 7L
let max_message_bytes = 1_048_576

module Kind = struct
  type t =
    | Container
    | Text
    | Button
  [@@deriving bin_io, equal, sexp_of]
end

module Length = struct
  type t =
    | Px of float
    | Percent of float
    | Auto
  [@@deriving bin_io, equal, sexp_of]
end

module Color = struct
  type t =
    | Rgba of int64
    | Token of int64
  [@@deriving bin_io, equal, sexp_of]
end

module Fill = struct
  type t = Solid of Color.t | Linear_gradient of float * Color.t * float * Color.t * float
  [@@deriving bin_io, equal, sexp_of]
end

module Shadow = struct
  type t = { color:Color.t; offset_x:float; offset_y:float; blur:float; spread:float; inset:bool }
  [@@deriving bin_io, equal, sexp_of]
end

module Field = struct
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

module Style = struct
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

module Op = struct
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

module Transaction = struct
  type t =
    { window : Window_id.t
    ; base : int64
    ; revision : int64
    ; operations : Op.t list
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Message = struct
  type t =
    | Hello of int64 * int64
    | Open of int64 * Window_id.t * string * float * float
    | Close of int64 * Window_id.t
    | Apply of Transaction.t
    | Request_frame of int64 * Window_id.t
    | Shutdown
  [@@deriving bin_io, equal, sexp_of]

  let encode t =
    if bin_size_t t > max_message_bytes
    then Or_error.error_string "message exceeds byte limit"
    else Ok (Bin_prot.Utils.bin_dump bin_writer_t t |> Bigstring.to_string)
  ;;
end

module Error_code = struct
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

module Event = struct
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

  let decode bytes =
    if String.length bytes > max_message_bytes
    then Or_error.error_string "event envelope exceeds byte limit"
    else (
      try
        let buffer = Bigstring.of_string bytes in
        let pos_ref = ref 0 in
        let count = (Bin_prot.Read.bin_read_nat0 buffer ~pos_ref :> int) in
        if count < 0 || count > 256
        then Or_error.error_string "event envelope exceeds count limit"
        else (
          (* Core.List.init invokes its function in descending index order. Reading
           a mutable cursor requires explicit wire order, then one reversal. *)
          let rec read remaining reversed =
            if remaining = 0
            then List.rev reversed
            else (
              let event = bin_read_t buffer ~pos_ref in
              read (remaining - 1) (event :: reversed))
          in
          let events = read count [] in
          if !pos_ref = String.length bytes
          then Ok events
          else Or_error.error_string "trailing event bytes")
      with
      | Bin_prot.Common.Buffer_short
      | Bin_prot.Common.Read_error _
      | Generational_id.Invalid_wire_handle ->
        Or_error.error_string "malformed event envelope")
  ;;
end
