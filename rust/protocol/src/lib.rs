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
pub mod canvas;
pub mod canvas_resource;
pub mod canvas_scene;
pub mod canvas_view;
pub mod color_input;
pub mod color_value;
mod command;
pub mod container_query;
mod decode;
pub mod document;
pub mod drag_drop;
pub mod extension;
pub mod file_dialog;
pub mod file_path;
mod id;
pub mod image;
pub mod list;
pub mod loading;
mod menu;
pub mod number_input;
pub mod otp;
pub mod otp_input;
mod palette;
pub mod rating;
pub mod v1;

pub use decode::{
    DecodeError, decode, decode_accessibility, decode_animation_program, decode_calendar_command,
    decode_calendar_config, decode_calendar_constraints, decode_calendar_event,
    decode_calendar_response, decode_calendar_selection, decode_canvas_scene,
    decode_canvas_view_config, decode_container_query, decode_number_input_command,
    decode_number_input_config, decode_number_input_event, decode_number_input_response,
    decode_numeric_domain, decode_otp_input_command, decode_otp_input_config,
    decode_otp_input_event, decode_otp_input_response, decode_slider_command, decode_slider_config,
    decode_slider_event,
};
pub use id::{HandlerId, NodeId, ResourceId, WindowId};

pub mod progress;

pub mod toast;

pub mod pointer;

pub mod window;

pub mod split;

pub mod numeric;

pub mod slider;
