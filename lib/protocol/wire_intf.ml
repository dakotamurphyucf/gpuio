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
      | Input
      | Textarea
      | Checkbox
      | Switch
      | Radio_group
      | Select
      | Combobox
      | Focus_scope
      | Tooltip
    [@@deriving bin_io, equal, sexp_of]
  end

  module Tooltip_open_state : sig
    type t =
      | Managed of bool
      | Controlled of bool
    [@@deriving bin_io, equal, sexp_of]
  end

  module Tooltip : sig
    type t =
      { label : string
      ; width : float
      ; open_state : Tooltip_open_state.t
      ; disabled : bool
      ; hoverable : bool
      ; show_delay_ns : int64
      ; hide_delay_ns : int64
      ; skip_delay_ns : int64
      }
    [@@deriving bin_io, equal, sexp_of]
  end

  module Side : sig
    type t =
      | Top
      | Right
      | Bottom
      | Left
    [@@deriving bin_io, equal, sexp_of]
  end

  module Align : sig
    type t =
      | Start
      | Center
      | End
    [@@deriving bin_io, equal, sexp_of]
  end

  module Placement : sig
    type t =
      { side : Side.t
      ; align : Align.t
      ; offset : float
      }
    [@@deriving bin_io, equal, sexp_of]
  end

  module Overlay_kind : sig
    type t =
      | Dialog
      | Popover
    [@@deriving bin_io, equal, sexp_of]
  end

  module Dismissal : sig
    type t =
      | Escape
      | Outside_pointer
    [@@deriving bin_io, equal, sexp_of]
  end

  module Overlay : sig
    type t =
      { kind : Overlay_kind.t
      ; label : string
      ; width : float
      ; dismiss_on_escape : bool
      ; dismiss_on_outside_pointer : bool
      }
    [@@deriving bin_io, equal, sexp_of]
  end

  module Focus_scope : sig
    type t =
      { trap : bool
      ; auto_focus : bool
      ; restore_focus : bool
      }
    [@@deriving bin_io, equal, sexp_of]
  end

  module Combobox_filter : sig
    type t =
      | Substring
      | Unfiltered
    [@@deriving bin_io, equal, sexp_of]
  end

  module Check_state : sig
    type t =
      | Unchecked
      | Checked
      | Indeterminate
    [@@deriving bin_io, equal, sexp_of]
  end

  module Control : sig
    (** The final Boolean in each case is [disabled]. *)
    type t =
      | Button of bool
      | Checkbox of Check_state.t * bool
      | Switch of bool * bool
    [@@deriving bin_io, equal, sexp_of]
  end

  module Choice : sig
    module Item : sig
      type t =
        { id : string
        ; label : string
        ; disabled : bool
        }
      [@@deriving bin_io, equal, sexp_of]
    end

    module Config : sig
      type t =
        { label : string
        ; items : Item.t list
        ; selected : string option
        ; disabled : bool
        }
      [@@deriving bin_io, equal, sexp_of]
    end
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
    type t =
      | Solid of Color.t
      | Linear_gradient of float * Color.t * float * Color.t * float
    [@@deriving bin_io, equal, sexp_of]
  end

  module Shadow : sig
    type t =
      { color : Color.t
      ; offset_x : float
      ; offset_y : float
      ; blur : float
      ; spread : float
      ; inset : bool
      }
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
      | User_select of bool
      | Selection_color of Color.t
      | Accessible_name of string
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

  module Choice_appearance : sig
    type t =
      { popup_width : float
      ; row_height : float
      ; max_visible_rows : int64
      ; empty_label : string
      ; popup_style : Style.t list
      ; option_style : Style.t list
      ; empty_style : Style.t list
      }
    [@@deriving bin_io, equal, sexp_of]
  end

  module Editor : sig
    module Selection : sig
      type t =
        { anchor : int64
        ; head : int64
        }
      [@@deriving bin_io, equal, sexp_of]
    end

    module Config : sig
      type t =
        { label : string
        ; placeholder : string
        ; read_only : bool
        ; disabled : bool
        ; submit_on_enter : bool
        ; auto_focus : bool
        ; min_rows : int64
        ; max_rows : int64
        }
      [@@deriving bin_io, equal, sexp_of]
    end

    module Snapshot : sig
      type t =
        { revision : int64
        ; text : string
        ; selection : Selection.t
        ; composition : Selection.t option
        ; focused : bool
        }
      [@@deriving bin_io, equal, sexp_of]
    end

    module Selection_policy : sig
      type t =
        | Start
        | End
        | Preserve
        | Select of Selection.t
      [@@deriving bin_io, equal, sexp_of]
    end

    module Undo_policy : sig
      type t =
        | Record
        | Reset
      [@@deriving bin_io, equal, sexp_of]
    end

    module Command : sig
      type t =
        | Replace of string * Selection_policy.t * Undo_policy.t * int64 option
        | Select of Selection.t
        | Focus
        | Undo
        | Redo
      [@@deriving bin_io, equal, sexp_of]
    end

    module Error : sig
      type t =
        | Not_mounted
        | Closed
        | Stale_editor
        | Stale_revision
        | Composing
        | Invalid_selection
        | Limit_exceeded
        | Busy
        | Native_failure
        | Invalid_text
        | Focus_blocked
      [@@deriving bin_io, equal, sexp_of]
    end

    module Result : sig
      type t =
        | Applied of Snapshot.t
        | Failed of Error.t
      [@@deriving bin_io, equal, sexp_of]
    end

    module Event_kind : sig
      type t =
        | Changed
        | Submitted
      [@@deriving bin_io, equal, sexp_of]
    end
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
      | Set_editor of Node_id.t * Editor.Config.t
      | Set_control of Node_id.t * Control.t
      | Set_choice of Node_id.t * Choice.Config.t
      | Set_choice_appearance of Node_id.t * Choice_appearance.t
      | Set_combobox_filter of Node_id.t * Combobox_filter.t
      | Set_focus_scope of Node_id.t * Focus_scope.t
      | Set_overlay of Node_id.t * Overlay.t option
      | Set_placement of Node_id.t * Placement.t option
      | Set_tooltip of Node_id.t * Tooltip.t
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
      | Editor_command of int64 * Window_id.t * Node_id.t * Editor.Command.t
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
      | Editor_event of
          Window_id.t
          * Node_id.t
          * Handler_id.t
          * int64
          * Editor.Event_kind.t
          * Editor.Snapshot.t
      | Editor_result of int64 * Window_id.t * Node_id.t * Editor.Result.t
      | Choice of Window_id.t * Node_id.t * Handler_id.t * int64 * string
      | Combobox_selected of
          Window_id.t * Node_id.t * Handler_id.t * int64 * string * Editor.Snapshot.t
      | Overlay_dismissed of Window_id.t * Node_id.t * Handler_id.t * int64 * Dismissal.t
      | Tooltip_open_changed of Window_id.t * Node_id.t * Handler_id.t * int64 * bool
    [@@deriving bin_io, equal, sexp_of]

    (** Decode one bounded event envelope, requiring full byte consumption and
        validating handle representations. This performs no callback dispatch. *)
    val decode : string -> t list Or_error.t
  end
end
