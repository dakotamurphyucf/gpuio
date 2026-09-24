open Core
module Asset = Asset_wire
module Image = Image_wire
module Animation = Animation_wire

let version = 1L
let capabilities = 67108863L
let max_message_bytes = 1_048_576

module Kind = struct
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
    | Command_scope
    | Command_button
    | Menu
    | Command_palette
    | Progress
    | Toast
    | Toast_stack
    | Pointer_area
    | Drag_source
    | Drop_target
    | Image
    | Icon
    | Animated
    | Virtual_list
  [@@deriving bin_io, equal, sexp_of]
end

module Shortcut_modifier = struct
  type t =
    | Primary
    | Control
    | Alt
    | Shift
    | Super
  [@@deriving bin_io, equal, sexp_of]
end

module Shortcut_priority = struct
  type t =
    | Native_first
    | Override
  [@@deriving bin_io, equal, sexp_of]
end

module Shortcut_text_input = struct
  type t =
    | Modified_only
    | Always
    | Never
  [@@deriving bin_io, equal, sexp_of]
end

module Shortcut = struct
  type t =
    { key : string
    ; modifiers : Shortcut_modifier.t list
    ; priority : Shortcut_priority.t
    ; text_input : Shortcut_text_input.t
    ; during_composition : bool
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Native_command = struct
  type t =
    | Copy
    | Cut
    | Paste
    | Select_all
    | Undo
    | Redo
  [@@deriving bin_io, equal, sexp_of]
end

module Command_target = struct
  type t =
    | Callback
    | Native of Native_command.t
  [@@deriving bin_io, equal, sexp_of]
end

module Command = struct
  type t =
    { id : string
    ; generation : int64
    ; label : string
    ; enabled : bool
    ; checked : bool option
    ; shortcuts : Shortcut.t list
    ; target : Command_target.t
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Command_source = struct
  type t =
    | Button of Node_id.t
    | Shortcut
    | Menu of Node_id.t
    | Palette of Node_id.t
  [@@deriving bin_io, equal, sexp_of]
end

module Tooltip_open_state = struct
  type t =
    | Managed of bool
    | Controlled of bool
  [@@deriving bin_io, equal, sexp_of]
end

module Tooltip = struct
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

module Side = struct
  type t =
    | Top
    | Right
    | Bottom
    | Left
  [@@deriving bin_io, equal, sexp_of]
end

module Align = struct
  type t =
    | Start
    | Center
    | End
  [@@deriving bin_io, equal, sexp_of]
end

module Placement = struct
  type t =
    { side : Side.t
    ; align : Align.t
    ; offset : float
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Overlay_kind = struct
  type t =
    | Dialog
    | Popover
  [@@deriving bin_io, equal, sexp_of]
end

module Dismissal = struct
  type t =
    | Escape
    | Outside_pointer
  [@@deriving bin_io, equal, sexp_of]
end

module Overlay = struct
  type t =
    { kind : Overlay_kind.t
    ; label : string
    ; width : float
    ; dismiss_on_escape : bool
    ; dismiss_on_outside_pointer : bool
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Focus_scope = struct
  type t =
    { trap : bool
    ; auto_focus : bool
    ; restore_focus : bool
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Combobox_filter = struct
  type t =
    | Substring
    | Unfiltered
  [@@deriving bin_io, equal, sexp_of]
end

module Check_state = struct
  type t =
    | Unchecked
    | Checked
    | Indeterminate
  [@@deriving bin_io, equal, sexp_of]
end

module Control = struct
  type t =
    | Button of bool
    | Checkbox of Check_state.t * bool
    | Switch of bool * bool
  [@@deriving bin_io, equal, sexp_of]
end

module Choice = struct
  module Item = struct
    type t =
      { id : string
      ; label : string
      ; disabled : bool
      }
    [@@deriving bin_io, equal, sexp_of]
  end

  module Config = struct
    type t =
      { label : string
      ; items : Item.t list
      ; selected : string option
      ; disabled : bool
      }
    [@@deriving bin_io, equal, sexp_of]
  end
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
  type t =
    | Solid of Color.t
    | Linear_gradient of float * Color.t * float * Color.t * float
  [@@deriving bin_io, equal, sexp_of]
end

module Shadow = struct
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
    | User_select of bool
    | Selection_color of Color.t
    | Accessible_name of string
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

module Choice_appearance = struct
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

module Editor = struct
  module Selection = struct
    type t =
      { anchor : int64
      ; head : int64
      }
    [@@deriving bin_io, equal, sexp_of]
  end

  module Config = struct
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

  module Snapshot = struct
    type t =
      { revision : int64
      ; text : string
      ; selection : Selection.t
      ; composition : Selection.t option
      ; focused : bool
      }
    [@@deriving bin_io, equal, sexp_of]
  end

  module Selection_policy = struct
    type t =
      | Start
      | End
      | Preserve
      | Select of Selection.t
    [@@deriving bin_io, equal, sexp_of]
  end

  module Undo_policy = struct
    type t =
      | Record
      | Reset
    [@@deriving bin_io, equal, sexp_of]
  end

  module Command = struct
    type t =
      | Replace of string * Selection_policy.t * Undo_policy.t * int64 option
      | Select of Selection.t
      | Focus
      | Undo
      | Redo
    [@@deriving bin_io, equal, sexp_of]
  end

  module Error = struct
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

  module Result = struct
    type t =
      | Applied of Snapshot.t
      | Failed of Error.t
    [@@deriving bin_io, equal, sexp_of]
  end

  module Event_kind = struct
    type t =
      | Changed
      | Submitted
    [@@deriving bin_io, equal, sexp_of]
  end
end

module Menu_definition = struct
  type t =
    { label : string
    ; disabled : bool
    ; items : item list
    }

  and item =
    | Command of string
    | Separator
    | Submenu of t
  [@@deriving bin_io, equal, sexp_of]
end

module Menu_presentation = struct
  type t =
    | Button
    | Context
    | Bar
    | Platform_bar
  [@@deriving bin_io, equal, sexp_of]
end

module Menu = struct
  type t =
    { presentation : Menu_presentation.t
    ; menus : Menu_definition.t list
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Palette = struct
  type t =
    { label : string
    ; placeholder : string
    ; commands : string list
    ; dismiss_on_outside_pointer : bool
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Palette_dismissal = struct
  type t =
    | Escape
    | Outside_pointer
    | Selected of string
  [@@deriving bin_io, equal, sexp_of]
end

module Progress = struct
  type t =
    { label : string
    ; fraction : float option
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Toast_politeness = struct
  type t =
    | Polite
    | Assertive
  [@@deriving bin_io, equal, sexp_of]
end

module Toast_corner = struct
  type t =
    | Top_left
    | Top_right
    | Bottom_left
    | Bottom_right
  [@@deriving bin_io, equal, sexp_of]
end

module Toast = struct
  type t =
    { label : string
    ; close_label : string
    ; timeout_ns : int64 option
    ; politeness : Toast_politeness.t
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Toast_stack = struct
  type t =
    { label : string
    ; corner : Toast_corner.t
    ; width : float
    ; max_visible : int64
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Toast_dismissal = struct
  type t =
    | Timeout
    | Close_button
    | Escape
    | Overflow
  [@@deriving bin_io, equal, sexp_of]
end

module Pointer = struct
  module Button = struct
    type t =
      | Left
      | Right
      | Middle
      | Back
      | Forward
    [@@deriving bin_io, equal, sexp_of]
  end

  module Cancel_reason = struct
    type t =
      | Escape
      | Hidden
      | Blocked
      | Disabled
      | Reconfigured
      | Capture_lost
      | Window_inactive
      | Removed
    [@@deriving bin_io, equal, sexp_of]
  end

  module Phase = struct
    type t =
      | Started
      | Moved
      | Released
      | Cancelled of Cancel_reason.t
    [@@deriving bin_io, equal, sexp_of]
  end

  module Modifiers = struct
    type t =
      { shift : bool
      ; control : bool
      ; alt : bool
      ; command : bool
      ; function_ : bool
      }
    [@@deriving bin_io, equal, sexp_of]
  end

  module Config = struct
    type t =
      { label : string
      ; button : Button.t
      ; disabled : bool
      ; prevent_default : bool
      ; stop_propagation : bool
      }
    [@@deriving bin_io, equal, sexp_of]
  end

  module Sample = struct
    type t =
      { gesture : int64
      ; phase : Phase.t
      ; button : Button.t
      ; window_x : float
      ; window_y : float
      ; local_x : float
      ; local_y : float
      ; modifiers : Modifiers.t
      }
    [@@deriving bin_io, equal, sexp_of]
  end
end

module Drag_and_drop = Drag_and_drop_wire

module Op = struct
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
    | Set_commands of Node_id.t * Command.t list
    | Set_command_ref of Node_id.t * string
    | Set_menu of Node_id.t * Menu.t
    | Set_palette of Node_id.t * Palette.t
    | Set_progress of Node_id.t * Progress.t
    | Set_toast of Node_id.t * Toast.t
    | Set_toast_stack of Node_id.t * Toast_stack.t
    | Set_pointer of Node_id.t * Pointer.Config.t
    | Set_drag_source of Node_id.t * Drag_and_drop.Source.t
    | Set_drop_target of Node_id.t * Drag_and_drop.Target.t
    | Set_image of Node_id.t * Image.Config.t
    | Set_animation of Node_id.t * Animation.Config.t
    | Set_list_config of Node_id.t * List_wire.Config.t
    | Set_list_order of Node_id.t * List_wire.Order.t
    | Set_list_rows of Node_id.t * List_wire.Row.t list
    | Invalidate_list_rows of Node_id.t * int64 list
    | Scroll_list of Node_id.t * List_wire.Scroll_request.t
  [@@deriving bin_io, equal, sexp_of]
end

module File_dialog = struct
  module Selection = struct
    type t =
      | Files
      | Directories
      | Files_and_directories
    [@@deriving bin_io, equal, sexp_of]
  end

  module Open = struct
    type t =
      { selection : Selection.t
      ; multiple : bool
      ; title : string
      ; accept_label : string
      ; directory : string option
      }
    [@@deriving bin_io, equal, sexp_of]
  end

  module Save = struct
    type t =
      { directory : string
      ; suggested_name : string
      ; title : string
      ; accept_label : string
      }
    [@@deriving bin_io, equal, sexp_of]
  end

  module Selection_support = struct
    type t =
      | Unsupported
      | Single
      | Multiple
    [@@deriving bin_io, equal, sexp_of]
  end

  module Capabilities = struct
    type t =
      { files : Selection_support.t
      ; directories : Selection_support.t
      ; files_and_directories : Selection_support.t
      ; save : bool
      }
    [@@deriving bin_io, equal, sexp_of]
  end

  module Config = struct
    type t =
      | Open of Open.t
      | Save of Save.t
      | Capabilities
    [@@deriving bin_io, equal, sexp_of]
  end

  module Error = struct
    type t =
      | Invalid_request
      | Unsupported
      | Busy
      | Closed
      | Not_ready
      | Native_failure
      | Limit_exceeded
    [@@deriving bin_io, equal, sexp_of]
  end

  exception Invalid_wire_result

  module Paths = struct
    type t = string list [@@deriving bin_io, equal, sexp_of]

    (* Reject declared counts/lengths before allocating lists or strings. Unix
       path bytes intentionally have no UTF-8 requirement. *)
    let bin_read_t buffer ~pos_ref =
      let count = (Bin_prot.Read.bin_read_nat0 buffer ~pos_ref :> int) in
      if count <= 0 || count > 128 then raise Invalid_wire_result;
      let rec read remaining total reversed =
        if remaining = 0
        then List.rev reversed
        else (
          let start = !pos_ref in
          let length = (Bin_prot.Read.bin_read_nat0 buffer ~pos_ref :> int) in
          if length <= 0 || length > 16_384 || total + length > 262_144
          then raise Invalid_wire_result;
          if length > Bigstring.length buffer - !pos_ref
          then raise Bin_prot.Common.Buffer_short;
          pos_ref := start;
          let path = Bin_prot.Read.bin_read_string buffer ~pos_ref in
          if (not (Char.equal path.[0] '/')) || String.contains path '\000'
          then raise Invalid_wire_result;
          read (remaining - 1) (total + length) (path :: reversed))
      in
      read count 0 []
    ;;

    let bin_reader_t = { bin_reader_t with read = bin_read_t }
    let bin_t = { bin_t with reader = bin_reader_t }
  end

  module Result = struct
    type t =
      | Selected of Paths.t
      | Cancelled
      | Failed of Error.t
      | Capabilities of Capabilities.t
    [@@deriving bin_io, equal, sexp_of]
  end
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
    | Editor_command of int64 * Window_id.t * Node_id.t * Editor.Command.t
    | File_dialog of int64 * Window_id.t * File_dialog.Config.t
    | Asset of int64 * Asset.Request.t
    | Set_motion of Animation.Preference.t
  [@@deriving bin_io, equal, sexp_of]

  let encode t =
    let invalid_asset =
      match t with
      | Asset (correlation, request) ->
        Int64.(correlation <= 0L)
        ||
          (match request with
          | Append (_, _, data) -> String.length data > Asset.max_chunk_bytes
          | Begin _ | Finish _ | Release _ -> false)
      | Hello _
      | Open _
      | Close _
      | Apply _
      | Request_frame _
      | Shutdown
      | Editor_command _
      | File_dialog _
      | Set_motion _ -> false
    in
    if invalid_asset
    then Or_error.error_string "invalid asset envelope"
    else if bin_size_t t > max_message_bytes
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
    | Command_invoked of
        Window_id.t * Node_id.t * Handler_id.t * int64 * string * int64 * Command_source.t
    | Palette_dismissed of
        Window_id.t * Node_id.t * Handler_id.t * int64 * Palette_dismissal.t
    | Toast_dismissed of
        Window_id.t * Node_id.t * Handler_id.t * int64 * Toast_dismissal.t
    | Pointer_event of Window_id.t * Node_id.t * Handler_id.t * int64 * Pointer.Sample.t
    | File_dialog_result of int64 * Window_id.t * File_dialog.Result.t
    | Drag_source_event of
        Window_id.t * Node_id.t * Handler_id.t * int64 * Drag_and_drop.Source_sample.t
    | Drop_target_event of
        Window_id.t * Node_id.t * Handler_id.t * int64 * Drag_and_drop.Target_sample.t
    | Asset_response of int64 * Asset.Response.t
    | Image_state of Window_id.t * Node_id.t * Handler_id.t * int64 * Image.State.t
    | Animation_endpoint of
        Window_id.t * Node_id.t * Handler_id.t * int64 * Animation.Endpoint.t
    | List_viewport of
        Window_id.t * Node_id.t * Handler_id.t * int64 * List_wire.Viewport.t
    | List_retained of Window_id.t * int64 * List_wire.Retained.t list
  [@@deriving bin_io, equal, sexp_of]

  let valid_snapshot (t : Editor.Snapshot.t) =
    let boundary offset =
      Int64.(offset >= 0L && offset <= of_int (String.length t.text))
      && (Int64.equal offset (Int64.of_int (String.length t.text))
          || Char.to_int t.text.[Int64.to_int_exn offset] land 0xc0 <> 0x80)
    in
    let selection (t : Editor.Selection.t) = boundary t.anchor && boundary t.head in
    Int64.(t.revision >= 0L)
    && String.length t.text <= 262_144
    && Stdlib.String.is_valid_utf_8 t.text
    && (not (String.contains t.text '\000'))
    && selection t.selection
    && Option.for_all t.composition ~f:(fun range ->
      Int64.(range.anchor <= range.head) && selection range)
  ;;

  let rec valid_event = function
    | List_retained (_, revision, notices) ->
      Int64.(revision > 0L) && Or_error.is_ok (List_wire.Retained.validate_all notices)
    | List_viewport (_, _, _, revision, viewport) ->
      Int64.(revision >= 0L) && Or_error.is_ok (List_wire.Viewport.validate viewport)
    | Animation_endpoint (_, _, _, revision, endpoint) ->
      Int64.(revision >= 0L && endpoint.generation > 0L)
    | Image_state (_, _, _, revision, state) ->
      Int64.(revision >= 0L)
      &&
        (match state with
        | Loading | Failed _ -> true
        | Ready { width_px; height_px; frames } ->
          Int64.(
            width_px > 0L
            && width_px <= 16384L
            && height_px > 0L
            && height_px <= 16384L
            && frames > 0L
            && frames <= 120L
            && width_px * height_px * 4L * frames <= 67108864L))
    | Asset_response (correlation, _) -> Int64.(correlation > 0L)
    | File_dialog_result (request, _, Selected paths) ->
      Int64.(request > 0L)
      && (not (List.is_empty paths))
      && List.length paths <= 128
      && List.sum (module Int) paths ~f:String.length <= 262_144
      && List.for_all paths ~f:(fun path ->
        String.length path > 0
        && String.length path <= 16_384
        && Char.equal path.[0] '/'
        && not (String.contains path '\000'))
    | File_dialog_result (request, _, (Cancelled | Failed _ | Capabilities _)) ->
      Int64.(request > 0L)
    | Drag_source_event (_, _, _, revision, sample) ->
      Int64.(revision >= 0L) && Drag_and_drop.Source_sample.is_valid sample
    | Drop_target_event (_, _, _, revision, sample) ->
      Int64.(revision >= 0L) && Drag_and_drop.Target_sample.is_valid sample
    | Pointer_event (_, _, _, revision, sample) ->
      Int64.(revision >= 0L && sample.gesture > 0L)
      && List.for_all
           [ sample.window_x; sample.window_y; sample.local_x; sample.local_y ]
           ~f:Float.is_finite
    | Palette_dismissed (window, node, handler, revision, Selected id) ->
      valid_event (Choice (window, node, handler, revision, id))
    | Palette_dismissed (_, _, _, revision, (Escape | Outside_pointer)) ->
      Int64.(revision >= 0L)
    | Command_invoked (_, _, _, revision, id, generation, _) ->
      Int64.(revision >= 0L && generation > 0L)
      && String.length id > 0
      && String.length id <= 256
      && Stdlib.String.is_valid_utf_8 id
      && not (String.contains id '\000')
    | Combobox_selected (window, node, handler, revision, id, snapshot) ->
      valid_event (Choice (window, node, handler, revision, id))
      && valid_snapshot snapshot
      && Option.is_none snapshot.composition
      && not (String.contains snapshot.text '\n' || String.contains snapshot.text '\r')
    | Toast_dismissed (_, _, _, revision, _)
    | Tooltip_open_changed (_, _, _, revision, _)
    | Overlay_dismissed (_, _, _, revision, _) -> Int64.(revision >= 0L)
    | Choice (_, _, _, revision, id) ->
      Int64.(revision >= 0L)
      && String.length id > 0
      && String.length id <= 256
      && Stdlib.String.is_valid_utf_8 id
      && not (String.contains id '\000')
    | Editor_event (_, _, _, revision, kind, snapshot) ->
      Int64.(revision >= 0L)
      && valid_snapshot snapshot
      &&
        (match kind with
        | Changed -> true
        | Submitted -> Option.is_none snapshot.composition)
    | Editor_result (_, _, _, Applied snapshot) -> valid_snapshot snapshot
    | Editor_result (_, _, _, Failed _)
    | Welcome _
    | Opened _
    | Closed _
    | Accepted _
    | Rejected _
    | Rendered _
    | Frame_requested _
    | Press _
    | Failed _
    | Stopped
    | Overloaded _ -> true
  ;;

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
          then
            if List.for_all events ~f:valid_event
            then Ok events
            else Or_error.error_string "invalid native event data"
          else Or_error.error_string "trailing event bytes")
      with
      | Bin_prot.Common.Buffer_short
      | Bin_prot.Common.Read_error _
      | Generational_id.Invalid_wire_handle
      | Drag_and_drop.Invalid_wire_data
      | File_dialog.Invalid_wire_result ->
        Or_error.error_string "malformed event envelope")
  ;;
end
