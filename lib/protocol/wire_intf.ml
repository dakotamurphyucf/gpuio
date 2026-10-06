open Core

module type S = sig
  module Asset = Asset_wire
  module Image = Image_wire
  module Animation = Animation_wire
  module Accessibility = Accessibility_wire
  module Loading = Loading_wire
  module Spinner = Spinner_wire
  module Avatar = Avatar_wire
  module Rating = Rating_wire
  module Slider = Slider_wire
  module Number_input = Number_input_wire
  module Otp_input = Otp_wire
  module Calendar = Calendar_wire
  module Color_input = Color_input_wire
  module Table = Table_wire
  module Tree_input = Tree_input_wire
  module List_input = List_input_wire
  module Carousel = Carousel_wire
  module Carousel_track = Carousel_track_wire
  module Navigation_stack = Navigation_stack_wire
  module Container_query = Container_query_wire
  module Animation_program = Animation_program_wire
  module Document = Document_wire
  module Document_diff = Document_diff_wire
  module Chart = Chart_resource_wire
  module Canvas = Canvas_resource_wire
  module Canvas_view = Canvas_view_wire
  module Chart_view = Chart_view_wire
  module Input_region = Input_wire
  module Highlight = Highlight_wire
  module Window = Window_wire
  module Desktop = Desktop_wire
  module Notification = Notification_wire
  module Grid_location = Grid_location_wire
  module Extension = Extension_wire
  module Split = Split_wire
  module Drag_and_drop = Drag_and_drop_wire

  val version : int64
  val capabilities : int64

  (** Check the native response before enabling window/tree submission. *)
  val validate_welcome
    :  protocol_version:int64
    -> available_capabilities:int64
    -> unit Or_error.t

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
      | Document_view
      | Tab_bar
      | Tab_panel
      | Split_pane
      | Extension
      | Canvas_view
      | Animation_program
      | Container_query
      | Loading
      | Avatar
      | Rating
      | Slider
      | Number_input
      | Otp_input
      | Calendar
      | Color_input
      | Panel
      | Disclosure
      | Accordion
      | Navigation_stack
      | Hover_card
      | Carousel
      | Chart_view
      | Input_region
      | Highlight_scope
      | Link
      | Radio
      | Choice_picker
      | Carousel_track
      | Carousel_track_group
      | Split_group
    [@@deriving bin_io, equal, sexp_of]
  end

  module Shortcut_modifier = Command_wire.Shortcut_modifier
  module Shortcut_priority = Command_wire.Shortcut_priority
  module Shortcut_text_input = Command_wire.Shortcut_text_input
  module Shortcut = Command_wire.Shortcut
  module Native_command = Command_wire.Native_command
  module Command_target = Command_wire.Command_target
  module Command = Command_wire.Command
  module Command_source = Command_wire.Command_source

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
      | Sheet_left
      | Sheet_right
      | Sheet_top
      | Sheet_bottom
      | Alert_dialog
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
      | Radio of bool * Checkable_wire.Position.t option * bool
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
      | Linear_gradient_in of int64 * float * Color.t * float * Color.t * float
    [@@deriving bin_io, equal, sexp_of]
  end

  module Scrollbar : sig
    module Axis : sig
      type t =
        | Horizontal
        | Vertical
        | Both
      [@@deriving bin_io, equal, sexp_of]
    end

    module Mode : sig
      type t =
        | Scrolling
        | Hover
        | Always
      [@@deriving bin_io, equal, sexp_of]
    end

    module Entrance : sig
      type t =
        | Fade
        | Slide_and_fade
      [@@deriving bin_io, equal, sexp_of]
    end

    module Track : sig
      type t =
        { background : int64 option
        ; border : int64 option
        ; width : float option
        }
      [@@deriving bin_io, equal, sexp_of]
    end

    module Thumb : sig
      type t =
        { background : Fill.t option
        ; width : float option
        ; inset : float option
        ; radius : float option
        ; min_length : float option
        }
      [@@deriving bin_io, equal, sexp_of]
    end

    module Appearance : sig
      type t =
        { track : Track.t
        ; track_hover : Track.t
        ; track_pressed : Track.t
        ; thumb : Thumb.t
        ; thumb_hover : Thumb.t
        ; thumb_pressed : Thumb.t
        }
      [@@deriving bin_io, equal, sexp_of]
    end

    module Motion : sig
      type t =
        { idle_ms : int64
        ; enter_ms : int64
        ; exit_ms : int64
        ; expand_ms : int64
        ; entrance : Entrance.t
        ; thumb_hover_entrance : Entrance.t
        }
      [@@deriving bin_io, equal, sexp_of]
    end

    type t =
      { label : string
      ; axis : Axis.t
      ; mode : Mode.t
      ; appearance : Appearance.t
      ; motion : Motion.t
      }
    [@@deriving bin_io, equal, sexp_of]

    val max_config_bytes : int
    val dimension : float -> bool
    val valid_track : Track.t -> bool
    val valid_thumb : Thumb.t -> bool
    val valid_motion : Motion.t -> bool
    val valid : t -> bool
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
      | Inert of bool
      | Pointer_occlusion of int64
      | Border_style of int64
      | Aspect_ratio of float
      | Disabled of bool
      | Grid_location of Grid_location.t
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

  module Number_presentation : sig
    type t =
      { gap : float
      ; button_width : float
      ; button_min_height : float
      ; stacked_button_min_height : float
      ; editor_padding : float
      ; border_width : float option
      ; frame_style : Style.t list
      ; editor_style : Style.t list
      ; decrement_style : Style.t list
      ; increment_style : Style.t list
      }
    [@@deriving bin_io, equal, sexp_of]
  end

  module Split_button : sig
    module Parts : sig
      type t =
        | Primary
        | Menu
        | Split
      [@@deriving bin_io, equal, sexp_of]
    end

    type t =
      { parts : Parts.t
      ; surface : Style.t list
      ; menu_open : Style.t list
      }
    [@@deriving bin_io, equal, sexp_of]
  end

  module Choice_picker_presentation : sig
    type t =
      { config : Choice_picker_wire.Config.t
      ; popup_width : float
      ; max_height : float
      ; estimated_row_height : float
      ; overscan : float
      ; empty_label : string
      ; popup_style : Style.t list
      ; option_style : Style.t list
      ; header_style : Style.t list
      ; empty_style : Style.t list
      ; slots : Choice_picker_wire.Slot.t list
      }
    [@@deriving bin_io, equal, sexp_of]
  end

  module Document_style : sig
    module Part : sig
      type t =
        | Foreground
        | Muted_foreground
        | Link
        | Selection
        | Code_background
        | Border
      [@@deriving bin_io, compare, equal, sexp_of]
    end

    module Heading_sizes : sig
      type t =
        { h1 : float
        ; h2 : float
        ; h3 : float
        ; h4 : float
        ; h5 : float
        ; h6 : float
        }
      [@@deriving bin_io, equal, sexp_of]
    end

    module Underline : sig
      type t =
        { color : Color.t option
        ; thickness : float
        ; wavy : bool
        }
      [@@deriving bin_io, equal, sexp_of]
    end

    module Strikethrough : sig
      type t =
        { color : Color.t option
        ; thickness : float
        }
      [@@deriving bin_io, equal, sexp_of]
    end

    module Inline_code : sig
      type t =
        { foreground : Color.t option
        ; background : Color.t option
        ; font_weight : int64 option
        ; italic : bool option
        ; underline : Underline.t option
        ; strikethrough : Strikethrough.t option
        ; fade_out : float option
        }
      [@@deriving bin_io, equal, sexp_of]
    end

    type t =
      { colors : (Part.t * Color.t) list
      ; paragraph_gap_rem : float option
      ; heading_base_font_size : float option
      ; heading_sizes : Heading_sizes.t option
      ; inline_code : Inline_code.t
      ; code_block : Style.t list
      ; table : Style.t list
      ; table_head : Style.t list
      ; table_cell : Style.t list
      }
    [@@deriving bin_io, equal, sexp_of]
  end

  module Control_appearance : sig
    module Label_position : sig
      type t =
        | Before
        | After
      [@@deriving bin_io, equal, sexp_of]
    end

    type t =
      { size : float
      ; switch_width : float
      ; gap : float
      ; label_position : Label_position.t
      ; indicator_style : Style.t list
      ; mark_style : Style.t list
      }
    [@@deriving bin_io, equal, sexp_of]
  end

  module Tab_motion : sig
    type t =
      { spring : Animation.Spring.t
      ; color_duration_ms : int64
      }
    [@@deriving bin_io, equal, sexp_of]
  end

  module Tab_viewport : sig
    module Reveal : sig
      type t =
        { serial : int64
        ; target : string
        }
      [@@deriving bin_io, equal, sexp_of]
    end

    type t = { reveal : Reveal.t option } [@@deriving bin_io, equal, sexp_of]
  end

  module Tab_content : sig
    module Label : sig
      type t =
        | Default
        | Custom
        | Hidden
      [@@deriving bin_io, equal, sexp_of]
    end

    type t =
      { max_width : float option
      ; labels : Label.t list
      }
    [@@deriving bin_io, equal, sexp_of]
  end

  module Split_group_appearance : sig
    type t =
      { thickness : float
      ; hit_extent : float
      ; handle_style : Style.t list
      ; item_styles : (string * Style.t list) list
      }
    [@@deriving bin_io, equal, sexp_of]
  end

  module Tab_appearance : sig
    module Variant : sig
      type t =
        | Tab
        | Outline
        | Pill
        | Segmented
        | Underline
      [@@deriving bin_io, equal, sexp_of]
    end

    type t =
      { variant : Variant.t
      ; height : float
      ; gap : float
      ; padding : float
      ; tab_style : Style.t list
      ; item_styles : (string * Style.t list) list
      }
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

  module Editor = Editor_wire

  module Menu_definition : sig
    type t =
      { label : string
      ; disabled : bool
      ; items : item list
      }

    and item =
      | Command of string
      | Separator
      | Submenu of t
      | Label of string
    [@@deriving bin_io, equal, sexp_of]
  end

  module Menu_presentation : sig
    type t =
      | Button
      | Context
      | Bar
      | Platform_bar
      | Editor_context
    [@@deriving bin_io, equal, sexp_of]
  end

  module Menu : sig
    type t =
      { presentation : Menu_presentation.t
      ; menus : Menu_definition.t list
      }
    [@@deriving bin_io, equal, sexp_of]
  end

  module Palette : sig
    type t =
      { label : string
      ; placeholder : string
      ; commands : string list
      ; dismiss_on_outside_pointer : bool
      }
    [@@deriving bin_io, equal, sexp_of]
  end

  module Palette_dismissal : sig
    type t =
      | Escape
      | Outside_pointer
      | Selected of string
    [@@deriving bin_io, equal, sexp_of]
  end

  module Progress = Progress_wire

  module Toast_politeness : sig
    type t =
      | Polite
      | Assertive
    [@@deriving bin_io, equal, sexp_of]
  end

  module Toast_corner : sig
    type t =
      | Top_left
      | Top_right
      | Bottom_left
      | Bottom_right
    [@@deriving bin_io, equal, sexp_of]
  end

  module Toast : sig
    type t =
      { label : string
      ; close_label : string
      ; timeout_ns : int64 option
      ; politeness : Toast_politeness.t
      }
    [@@deriving bin_io, equal, sexp_of]
  end

  module Toast_stack : sig
    type t =
      { label : string
      ; corner : Toast_corner.t
      ; width : float
      ; max_visible : int64
      }
    [@@deriving bin_io, equal, sexp_of]
  end

  module Toast_dismissal : sig
    type t =
      | Timeout
      | Close_button
      | Escape
      | Overflow
    [@@deriving bin_io, equal, sexp_of]
  end

  module Pointer : sig
    module Button : sig
      type t =
        | Left
        | Right
        | Middle
        | Back
        | Forward
      [@@deriving bin_io, equal, sexp_of]
    end

    module Cancel_reason : sig
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

    module Phase : sig
      type t =
        | Started
        | Moved
        | Released
        | Cancelled of Cancel_reason.t
      [@@deriving bin_io, equal, sexp_of]
    end

    module Modifiers : sig
      type t =
        { shift : bool
        ; control : bool
        ; alt : bool
        ; command : bool
        ; function_ : bool
        }
      [@@deriving bin_io, equal, sexp_of]
    end

    module Config : sig
      type t =
        { label : string
        ; button : Button.t
        ; disabled : bool
        ; prevent_default : bool
        ; stop_propagation : bool
        }
      [@@deriving bin_io, equal, sexp_of]
    end

    module Sample : sig
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
      | Set_document of Node_id.t * Document.Config.t
      | Set_split of Node_id.t * Split.Config.t
      | Set_extension of Node_id.t * Extension.Config.t
      | Set_canvas of Node_id.t * Canvas_view.Config.t
      | Set_animation_program of Node_id.t * Animation_program.Config.t
      | Set_container_query of Node_id.t * Container_query.Config.t
      | Set_accessibility of Node_id.t * Accessibility.Config.t option
      | Set_loading of Node_id.t * Loading.Config.t
      | Set_avatar of Node_id.t * Avatar.Config.t
      | Set_rating of Node_id.t * Rating.Config.t
      | Set_slider of Node_id.t * Slider.Config.t * Slider.Value.t
      | Set_number_input of Node_id.t * Number_input.Config.t * Number_input.Value.t
      | Set_otp_input of Node_id.t * Otp_input.Config.t * string
      | Set_calendar of Node_id.t * Calendar.Config.t * Calendar.Selection.t * int64
      | Set_color_input of Node_id.t * Color_input.Config.t * Color_input.Value.t
      | Set_navigation_stack of Node_id.t * Navigation_stack.Config.t
      | Set_carousel of Node_id.t * Carousel.Config.t
      | Set_tree_input of Node_id.t * bool
      | Set_tree_moves of Node_id.t * bool
      | Set_table of Node_id.t * Table.Config.t
      | Set_table_cell of Node_id.t * Table.Cell.t
      | Table_command of Node_id.t * Table.Command.t
      | Set_chart of Node_id.t * Chart_view.Config.t
      | Set_input_region of Node_id.t * Input_region.Config.t
      | Set_highlight_scope of Node_id.t * Highlight.Config.t
      | Set_document_diff of Node_id.t * int64 * Document_diff.Config.t option
      | Set_styled_text of Node_id.t * Text_content_wire.t
      | Set_link of Node_id.t * Link_wire.t
      | Set_text_shimmer of Node_id.t * Text_shimmer_wire.Config.t option
      | Set_command_binding of Node_id.t * Command_binding_wire.Config.t option
      | Set_number_input_draft of Node_id.t * string option
      | Set_rating_appearance of Node_id.t * Rating.Appearance.t option
      | Set_spinner of Node_id.t * Spinner.Config.t
      | Set_progress_presentation of Node_id.t * Progress.Presentation.t
      | Set_control_appearance of Node_id.t * Control_appearance.t option
      | Set_tab_order of Node_id.t * Checkable_wire.Tab_order.t option
      | Set_button_presentation of Node_id.t * Button_wire.Config.t option
      | Set_split_button of Node_id.t * Split_button.t option
      | Set_hover_observer of Node_id.t * Handler_id.t option
      | Set_choice_picker of Node_id.t * Choice_picker_presentation.t
      | Set_editor_privacy of Node_id.t * Editor.Privacy.t
      | Set_editor_frame of Node_id.t * Editor_frame_wire.t option
      | Set_editor_content_hint of Node_id.t * Input_content_hint_wire.t option
      | Set_editor_format of Node_id.t * Input_format_wire.t option
      | Set_editor_validation of Node_id.t * Input_validation_wire.Rule.t option
      | Set_text_area_layout of Node_id.t * Text_area_layout_wire.t option
      | Set_editor_clear_on_escape of Node_id.t * bool
      | Set_editor_searchable of Node_id.t * bool
      | Set_otp_appearance of Node_id.t * Otp_presentation_wire.t option
      | Set_number_presentation of Node_id.t * Number_presentation.t option
      | Set_number_step_mode of Node_id.t * Number_input.Step_mode.t
      | Set_slider_appearance of Node_id.t * Slider_presentation_wire.t option
      | Set_reveal of Node_id.t * Reveal_wire.t option
      | Set_calendar_appearance of Node_id.t * Calendar_presentation_wire.t option
      | Set_color_presentation of Node_id.t * Color_presentation_wire.t option
      | Set_popover of Node_id.t * bool
      | Set_calendar_content of Node_id.t * Calendar_content_wire.t option
      | Set_overlay_backdrop of Node_id.t * int64 option
      | Set_overlay_motion of Node_id.t * bool
      | Set_tooltip_motion of Node_id.t * bool
      | Set_placement_geometry of Node_id.t * Placement_geometry_wire.t option
      | Set_sheet_insets of Node_id.t * Sheet_insets_wire.t option
      | Set_calendar_viewport_observer of Node_id.t * Handler_id.t option
      | Set_carousel_track of Node_id.t * Carousel_track.Config.t
      | Set_carousel_track_motion of Node_id.t * Carousel_track.Motion.t option
      | Set_tab_appearance of Node_id.t * Tab_appearance.t option
      | Set_tab_content of Node_id.t * Tab_content.t option
      | Set_tab_viewport of Node_id.t * Tab_viewport.t option
      | Set_tab_trailing of Node_id.t * bool
      | Set_choice_menu of Node_id.t * bool
      | Set_tab_motion of Node_id.t * Tab_motion.t option
      | Set_split_group of
          Node_id.t * Split_group_wire.Config.t * Split_group_appearance.t
      | Set_toast_placement of Node_id.t * Toast_placement_wire.t option
      | Set_toast_layering of Node_id.t * Toast_layering_wire.t option
      | Set_toast_motion of Node_id.t * Toast_motion_wire.t option
      | Set_scrollbar of Node_id.t * Scrollbar.t option
      | Set_list_axis of Node_id.t * List_wire.Axis.t
      | Set_list_input of Node_id.t * List_input.Config.t option
      | Set_table_behavior of Node_id.t * Table_wire.Behavior.t option
      | Set_table_appearance of Node_id.t * Table_wire.Appearance.t option
      | Set_table_header of Node_id.t * Table_header_wire.t option
      | Set_table_header_style of Node_id.t * Style.t list
      | Set_table_row_style of Node_id.t * Style.t list
      | Set_document_selection_format of Node_id.t * bool
      | Set_document_preview of Node_id.t * Document_preview_wire.Config.t
      | Set_document_text_style of Node_id.t * Document_style.t option
      | Set_document_markdown_options of Node_id.t * Document.Markdown_options.t
      | Set_document_actions of Node_id.t * Document_actions_wire.Config.t
      | Set_document_profile of Node_id.t * Document_profile_wire.Config.t
      | Set_window_region of Node_id.t * Window_region_wire.t option
      | Create_table_text of Node_id.t * Table.Cell.t
      | Set_table_text of Node_id.t * Table.Cell.t
      | Set_palette_options of Node_id.t * Palette_options_wire.t option
      | Set_palette_layout of Node_id.t * Palette_layout_wire.t option
      | Set_palette_observed of Node_id.t * bool
    [@@deriving bin_io, equal, sexp_of]
  end

  module File_dialog : sig
    module Selection : sig
      type t =
        | Files
        | Directories
        | Files_and_directories
      [@@deriving bin_io, equal, sexp_of]
    end

    module Open : sig
      type t =
        { selection : Selection.t
        ; multiple : bool
        ; title : string
        ; accept_label : string
        ; directory : string option
        }
      [@@deriving bin_io, equal, sexp_of]
    end

    module Save : sig
      type t =
        { directory : string
        ; suggested_name : string
        ; title : string
        ; accept_label : string
        }
      [@@deriving bin_io, equal, sexp_of]
    end

    module Selection_support : sig
      type t =
        | Unsupported
        | Single
        | Multiple
      [@@deriving bin_io, equal, sexp_of]
    end

    module Capabilities : sig
      type t =
        { files : Selection_support.t
        ; directories : Selection_support.t
        ; files_and_directories : Selection_support.t
        ; save : bool
        }
      [@@deriving bin_io, equal, sexp_of]
    end

    module Config : sig
      type t =
        | Open of Open.t
        | Save of Save.t
        | Capabilities
      [@@deriving bin_io, equal, sexp_of]
    end

    module Error : sig
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

    module Result : sig
      type t =
        | Selected of string list
        | Cancelled
        | Failed of Error.t
        | Capabilities of Capabilities.t
      [@@deriving bin_io, equal, sexp_of]
    end
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
      | File_dialog of int64 * Window_id.t * File_dialog.Config.t
      | Asset of int64 * Asset.Request.t
      | Set_motion of Animation.Preference.t
      | Document of int64 * Document.Request.t
      | Window_command of int64 * Window_id.t * Window.Command.t
      | Open_configured of int64 * Window_id.t * Window.Config.t
      | Canvas of int64 * Canvas.Request.t
      | Slider_command of int64 * Window_id.t * Node_id.t * Slider.Command.t
      | Number_input_command of int64 * Window_id.t * Node_id.t * Number_input.Command.t
      | Otp_input_command of int64 * Window_id.t * Node_id.t * Otp_input.Command.t
      | Calendar_command of int64 * Window_id.t * Node_id.t * Calendar.Command.t
      | Color_input_command of int64 * Window_id.t * Node_id.t * Color_input.Command.t
      | Desktop of int64 * Desktop.Request.t
      | Notification of int64 * Notification.Request.t
      | Chart of int64 * Chart.Request.t
      | Palette_command of
          int64
          * Window_id.t
          * Node_id.t
          * Handler_id.t
          * int64 option
          * Palette_command_wire.Command.t
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
      | Command_invoked of
          Window_id.t
          * Node_id.t
          * Handler_id.t
          * int64
          * string
          * int64
          * Command_source.t
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
      | Document_response of int64 * Document.Response.t
      | Document_navigation of
          Window_id.t
          * Node_id.t
          * Handler_id.t
          * int64
          * Resource_id.t
          * int64
          * Document.Navigation.t
      | Close_requested of Window_id.t
      | Quit_requested
      | Reopen_requested
      | Window_changed of Window_id.t * Window.Snapshot.t
      | Window_response of int64 * Window_id.t * Window.Response.t
      | Window_capabilities of Window.Capabilities.t
      | Split_resized of
          Window_id.t * Node_id.t * Handler_id.t * int64 * int64 * Split.Snapshot.t
      | Extension_event of
          Window_id.t * Node_id.t * Handler_id.t * int64 * int64 * Extension.Signal.t
      | Canvas_response of int64 * Canvas.Response.t
      | Canvas_event of
          Window_id.t
          * Node_id.t
          * Handler_id.t
          * int64
          * Resource_id.t option
          * int64
          * int64
          * Canvas_view.Observation.t
      | Animation_program_event of
          Window_id.t * Node_id.t * Handler_id.t * int64 * Animation_program.Batch.t
      | Container_selected of
          Window_id.t * Node_id.t * Handler_id.t * int64 * Container_query.Snapshot.t
      | Rating_requested of
          Window_id.t * Node_id.t * Handler_id.t * int64 * Rating.Request.t
      | Slider_event of Window_id.t * Node_id.t * Handler_id.t * int64 * Slider.Event.t
      | Slider_result of int64 * Window_id.t * Node_id.t * Slider.Response.t
      | Number_input_event of
          Window_id.t * Node_id.t * Handler_id.t * int64 * Number_input.Event.t
      | Number_input_result of int64 * Window_id.t * Node_id.t * Number_input.Response.t
      | Otp_input_event of
          Window_id.t * Node_id.t * Handler_id.t * int64 * Otp_input.Event.t
      | Otp_input_result of int64 * Window_id.t * Node_id.t * Otp_input.Response.t
      | Calendar_event of
          Window_id.t * Node_id.t * Handler_id.t * int64 * Calendar.Event.t
      | Calendar_result of int64 * Window_id.t * Node_id.t * Calendar.Response.t
      | Color_input_event of
          Window_id.t * Node_id.t * Handler_id.t * int64 * Color_input.Event.t
      | Color_input_result of int64 * Window_id.t * Node_id.t * Color_input.Response.t
      | Carousel_requested of
          Window_id.t * Node_id.t * Handler_id.t * int64 * Carousel.Request.t
      | Tree_input of
          Window_id.t * Node_id.t * Handler_id.t * int64 * Tree_input.Request.t
      | Table_input of Window_id.t * Node_id.t * Handler_id.t * int64 * Table.Input.t
      | Desktop_response of int64 * Desktop.Response.t
      | Desktop_pending
      | Notification_response of int64 * Notification.Response.t
      | Notification_pending
      | Chart_response of int64 * Chart.Response.t
      | Chart_event of
          Window_id.t
          * Node_id.t
          * Handler_id.t
          * int64
          * Resource_id.t option
          * int64
          * int64
          * Chart_view.Observation.t
      | Input_observed of
          Window_id.t * Node_id.t * Handler_id.t * int64 * Input_region.Event.t
      | Highlight_observed of
          Window_id.t * Node_id.t * Handler_id.t * int64 * Highlight.Observation.t
      | Document_diff_event of
          Window_id.t
          * Node_id.t
          * Handler_id.t
          * int64
          * Resource_id.t
          * Document_diff.Event.t
      | Command_binding_observed of
          Window_id.t
          * Node_id.t
          * Handler_id.t
          * int64
          * Command_binding_wire.Observation.t
      | Menu_open_changed of Window_id.t * Node_id.t * Handler_id.t * int64 * bool
      | Hover_changed of Window_id.t * Node_id.t * Handler_id.t * int64 * bool
      | Choice_picker_event of
          Window_id.t * Node_id.t * Handler_id.t * int64 * Choice_picker_wire.Event.t
      | Editor_search_observed of
          Window_id.t * Node_id.t * Handler_id.t * int64 * Editor_search_wire.Snapshot.t
      | Calendar_viewport_changed of
          Window_id.t * Node_id.t * Handler_id.t * int64 * Calendar_viewport_wire.t
      | Carousel_track_requested of
          Window_id.t * Node_id.t * Handler_id.t * int64 * Carousel_track.Request.t
      | Split_group_resized of
          Window_id.t
          * Node_id.t
          * Handler_id.t
          * int64
          * int64
          * Split_group_wire.Snapshot.t
      | List_input of
          Window_id.t * Node_id.t * Handler_id.t * int64 * int64 * List_input.Request.t
      | Table_columns_observed of
          Window_id.t * Node_id.t * Handler_id.t * int64 * Table_wire.Column_viewport.t
      | Document_preview_observed of
          Window_id.t
          * Node_id.t
          * Handler_id.t
          * int64
          * Resource_id.t
          * Document_preview_wire.Event.t
      | Document_action of
          Window_id.t
          * Node_id.t
          * Handler_id.t
          * int64
          * Resource_id.t
          * Document_actions_wire.Event.t
      | Document_profile_event of
          Window_id.t
          * Node_id.t
          * Handler_id.t
          * int64
          * Resource_id.t
          * Document_profile_wire.Event.t
      | Palette_observed of
          Window_id.t * Node_id.t * Handler_id.t * int64 * Palette_state_wire.t
      | Palette_result of
          int64 * Window_id.t * Node_id.t * Handler_id.t * Palette_command_wire.Response.t
    [@@deriving bin_io, equal, sexp_of]

    (** Decode one bounded event envelope, requiring full byte consumption and
        validating handle representations. This performs no callback dispatch. *)
    val decode : string -> t list Or_error.t
  end
end
