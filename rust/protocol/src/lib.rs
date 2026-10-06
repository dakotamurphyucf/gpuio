//! Owned wire data. No GPUI, OCaml runtime, or I/O scheduler dependencies.
//!
//! V1 is under development; the private foundation protocol is unrelated.
pub mod accessibility;
pub mod animation;
pub mod animation_program;
pub mod asset;
pub mod avatar;
pub mod calendar;
pub mod calendar_input;
pub mod calendar_viewport;
pub mod canvas;
pub mod canvas_resource;
pub mod canvas_scene;
pub mod canvas_view;
pub mod carousel;
pub mod carousel_track;
pub use decode::{decode_carousel_track_config, decode_carousel_track_request};
pub mod chart_appearance;
pub use decode::decode_chart_appearance;
pub mod chart_axis;
pub mod chart_data;
pub mod chart_grid;
pub mod chart_inspection;
pub mod chart_node_labels;
pub mod chart_pie_labels;
pub use decode::decode_chart_node_labels;
pub mod chart_options;
pub mod chart_resource;
pub mod chart_sampling;
pub mod chart_selection;
pub mod chart_style;
pub mod chart_view;
pub mod color_input;
pub mod color_value;
mod command;
pub mod command_binding;
pub mod container_query;
mod decode;
pub mod desktop;
pub mod document;
pub mod document_actions;
pub mod document_diff;
pub mod document_preview;
pub mod document_profile;
pub mod document_style;
pub use decode::decode_document_style;
pub mod drag_drop;
pub mod editor_frame;
pub mod editor_geometry;
pub mod editor_search;
pub mod editor_viewport;
pub mod extension;
pub mod file_dialog;
pub mod file_path;
pub mod grid_location;
pub mod highlight;
mod id;
pub mod image;
pub mod input;
pub mod input_content_hint;
pub mod input_format;
pub mod input_validation;
pub mod list;
pub mod loading;
pub mod spinner;
pub use decode::decode_input_format;
pub use decode::decode_input_validation_source;
pub use decode::decode_spinner_config;
pub mod calendar_content;
pub mod calendar_presentation;
pub use decode::decode_calendar_content;
pub mod color_presentation;
mod menu;
pub mod navigation_stack;
pub mod notification;
pub mod number_input;
pub mod number_presentation;
pub mod otp;
pub mod otp_input;
pub mod otp_presentation;
mod palette;
pub mod rating;
pub mod reveal;
pub mod scrollbar;
pub use decode::decode_scrollbar_config;
pub mod slider_presentation;
pub mod v1;

pub use decode::decode_chart_selection;
pub use decode::decode_chart_view_config;
pub use decode::{
    DecodeError, decode, decode_accessibility, decode_animation_program, decode_calendar_command,
    decode_calendar_config, decode_calendar_constraints, decode_calendar_event,
    decode_calendar_response, decode_calendar_selection, decode_canvas_scene,
    decode_canvas_view_config, decode_carousel_config, decode_carousel_request, decode_chart_data,
    decode_chart_options, decode_chart_request, decode_chart_response, decode_chart_sampling,
    decode_chart_style, decode_color_command, decode_color_config, decode_color_event,
    decode_color_response, decode_container_query, decode_desktop_launch, decode_desktop_request,
    decode_navigation_stack, decode_number_input_command, decode_number_input_config,
    decode_number_input_event, decode_number_input_response, decode_numeric_domain,
    decode_otp_input_command, decode_otp_input_config, decode_otp_input_event,
    decode_otp_input_response, decode_slider_command, decode_slider_config, decode_slider_event,
    decode_table_cell, decode_table_command, decode_table_config, decode_table_request,
};
pub use decode::{
    decode_notification_event, decode_notification_request, decode_notification_response,
};
pub use id::{HandlerId, NodeId, ResourceId, WindowId};

pub mod progress;
pub mod progress_presentation;
pub use decode::decode_progress_presentation;

pub mod choice_picker;
pub mod link;
pub use decode::{
    decode_choice_picker_config, decode_choice_picker_event, decode_choice_picker_presentation,
};
pub mod table;
pub mod table_header;
pub use decode::decode_table_header_target;
pub mod text_content;
pub mod text_shimmer;
pub use decode::decode_link_config;
pub use decode::decode_text_shimmer_config;
pub mod toast;
pub use decode::decode_text_content;
pub mod list_input;
pub mod tree_input;

pub mod pointer;

pub mod window;
pub mod window_region;

pub mod split;
pub mod split_group;
pub mod split_group_appearance;
pub use decode::{
    decode_split_group_appearance, decode_split_group_config, decode_split_group_snapshot,
};

pub mod numeric;

pub mod slider;

pub use decode::{decode_document_diff_config, decode_document_diff_event};
pub use decode::{decode_highlight_config, decode_highlight_observation};
pub use decode::{decode_input_config, decode_input_event};

pub use decode::{decode_command_binding_config, decode_command_binding_observation};

pub mod control_appearance;
pub use decode::decode_control_appearance;
pub mod checkable;
pub use decode::{decode_radio_position, decode_tab_order};

pub mod button;
pub mod split_button;
pub use decode::decode_button_config;
pub use decode::decode_split_button_config;

pub mod text_area_layout;

pub mod placement_geometry;

pub mod sheet_insets;

pub mod tab_appearance;
pub mod tab_content;
pub mod tab_motion;
pub mod tab_viewport;

pub mod toast_layering;
pub mod toast_motion;
pub mod toast_placement;

pub mod table_presentation;

pub mod menu_command;
pub mod palette_command;
pub mod palette_layout;
pub mod palette_options;
pub mod palette_results;
pub mod palette_state;
