pub mod document_profiles;
pub mod extensions;
#[cfg(target_os = "macos")]
pub mod input_content_macos;
#[cfg(feature = "performance-diagnostics")]
pub mod performance;
pub mod registrations;
pub use gpuio_extension_sdk as extension_sdk;

mod appearance;
pub mod asset_cache;
pub mod asset_decode;
pub mod asset_store;
pub mod asset_svg;
pub mod canvas_content;
pub mod canvas_host;
pub mod canvas_jobs;
pub mod canvas_mesh;
pub mod canvas_paint;
pub mod canvas_plan;
pub mod canvas_state;
pub mod canvas_store;
mod chart_appearance;
mod chart_cartesian;
mod chart_colors;
mod chart_details;
pub mod chart_geometry;
mod chart_hit;
mod chart_host;
pub mod chart_jobs;
pub mod chart_label_metrics;
mod chart_node_labels;
pub mod chart_paint;
mod chart_presentation;
pub mod chart_render_host;
mod chart_table;
#[cfg(all(test, feature = "native-image-tests"))]
mod image_mask_test;
#[cfg(feature = "native-canvas-tests")]
pub fn run_native_chart_labels_test() {
    chart_label_metrics::native_test::run();
}
#[cfg(feature = "native-canvas-tests")]
pub fn run_native_chart_paint_test() {
    chart_paint::native_test::run();
}
#[cfg(feature = "native-canvas-tests")]
pub fn run_native_chart_input_test() {
    host::chart_view::test::run_input();
}
#[cfg(feature = "native-canvas-tests")]
pub fn run_native_chart_inspection_test() {
    host::chart_view::test::run_inspection();
}
#[cfg(feature = "native-canvas-tests")]
pub fn run_native_chart_view_test() {
    host::chart_view::test::run();
}
pub mod chart_reduce;
pub mod chart_selection;
pub mod chart_store;
pub mod choice_picker_admission;
pub mod choice_picker_list;
pub mod choice_picker_rows;
pub mod choice_picker_state;
mod desktop_host;
#[cfg(any(target_os = "linux", test))]
mod desktop_instance;
#[cfg(any(target_os = "linux", test))]
mod desktop_linux;
#[cfg(target_os = "macos")]
mod desktop_macos;
mod desktop_operations;
pub mod desktop_state;
mod document_highlight;
pub mod document_host;
pub mod document_jobs;
pub mod document_profile_jobs;
pub mod document_store;
mod document_style;
mod ffi;
pub mod file_dialog;
mod font_defaults;
mod host;
mod input_format;
mod input_validation;
mod notification_host;
#[cfg(any(test, target_os = "linux"))]
mod notification_linux;
#[cfg(target_os = "macos")]
mod notification_macos;
mod notification_operations;
mod notification_state;
#[cfg(feature = "native-tests")]
pub fn run_native_animation_test() {
    host::animation_test::run();
}
#[cfg(feature = "native-tests")]
pub fn run_native_animation_program_test() {
    host::animation_program_test::run();
}
pub mod carousel_clock;
pub mod carousel_gesture;
pub mod carousel_track_geometry;
mod carousel_track_gesture;
#[cfg(all(test, feature = "native-image-tests"))]
mod carousel_track_layout_test;
mod carousel_track_motion;
pub mod carousel_track_state;
mod carousel_track_wheel;
mod editor_accessibility;
pub mod image_host;
pub mod list_index;
pub mod list_state;
pub mod mailbox;
pub mod motion;
pub mod motion_clock;
pub mod motion_host;
mod motion_preference;
pub mod motion_program;
pub mod motion_timeline;
pub mod navigation_motion;
pub mod progress_clock;
mod progress_geometry;
pub mod progress_paint;
pub mod reveal_layout;
#[cfg(all(test, feature = "native-image-tests"))]
mod reveal_layout_test;
pub mod reveal_motion;
pub mod scrollbar_clock;
pub mod scrollbar_geometry;
pub mod scrollbar_input;
pub mod scrollbar_lifecycle;
pub mod scrollbar_presentation;
pub mod scrollbar_widget;
mod selection;
mod semantics;
pub mod session;
pub mod spinner_clock;
pub mod spinner_paint;
pub mod split_group_appearance;
pub mod split_group_geometry;
pub mod split_group_state;
pub mod split_group_widget;
mod style;
mod styled_text;
mod text_shimmer_budget;
pub mod text_shimmer_clock;
mod text_shimmer_color;
pub mod text_shimmer_paint;
pub mod toast_geometry;
pub mod toast_lifecycle;
pub mod toast_reflow;
pub mod toast_stack_widget;
#[cfg(all(test, feature = "native-image-tests"))]
mod toast_stack_widget_test;
#[cfg(feature = "native-image-tests")]
pub fn run_native_text_shimmer_paint_test() {
    text_shimmer_paint::native_test::run();
}
#[cfg(feature = "native-image-tests")]
pub fn run_native_text_shimmer_clock_test() {
    text_shimmer_clock::native_test::run();
}
#[cfg(feature = "native-image-tests")]
pub fn run_native_text_shimmer_view_test() {
    host::text_shimmer_view_test::run();
}
mod split_button;
mod text_projection;
mod transport;
pub mod tree;

#[cfg(feature = "native-tests")]
pub fn run_native_ui_test() {
    host::native_test::run();
}

#[cfg(all(target_os = "macos", feature = "native-tests"))]
pub fn run_native_menu_popup_queue_test() {
    host::run_native_menu_popup_queue_test();
}

#[cfg(feature = "native-tests")]
pub fn run_native_scroll_test() {
    host::scroll_test::run();
}

#[cfg(feature = "native-tests")]
pub fn run_native_editor_test() {
    host::editor_test::run();
}

#[cfg(feature = "native-tests")]
pub fn run_native_menu_test() {
    host::control_test::run_menus();
}
#[cfg(feature = "native-tests")]
pub fn run_native_palette_test() {
    host::control_test::run_palette();
}

#[cfg(feature = "native-tests")]
pub fn run_native_control_test() {
    host::control_test::run();
}

#[cfg(feature = "native-tests")]
pub fn run_native_progress_test() {
    host::control_test::run_progress();
}

#[cfg(feature = "native-tests")]
pub fn run_native_toast_test() {
    host::control_test::run_toast();
}

#[cfg(feature = "native-tests")]
pub fn run_native_input_region_test() {
    host::control_test::run_input_region();
}

#[cfg(feature = "native-tests")]
pub fn run_native_pointer_test() {
    host::control_test::run_pointer();
}

#[cfg(all(feature = "native-tests", target_os = "macos"))]
pub fn run_native_file_dialog_test() {
    file_dialog::test::run();
}

#[cfg(all(feature = "native-tests", target_os = "macos"))]
pub fn drive_native_file_dialog_test(pid: i32, filename: &str, accept_label: &str) {
    file_dialog::test::drive(pid, filename, accept_label);
}

#[cfg(feature = "native-tests")]
pub fn run_native_drag_drop_test() {
    host::control_test::run_drag_drop();
}

#[cfg(all(feature = "native-tests", target_os = "macos"))]
mod drag_drop_macos_test;
#[cfg(all(feature = "native-tests", target_os = "macos"))]
pub fn drive_native_drag_drop_test(pid: i32, mode: &str) {
    drag_drop_macos_test::drive(pid, mode);
}

#[cfg(feature = "native-image-tests")]
pub fn run_native_image_test() {
    image_host::test::run();
}

#[cfg(feature = "native-image-tests")]
pub fn run_native_image_view_test() {
    host::image_view::test::run();
}

#[cfg(feature = "native-tests")]
pub fn run_native_list_test() {
    host::list_test::run();
}

#[cfg(feature = "native-tests")]
pub fn run_native_table_host_test() {
    host::table_view::run_test();
}

#[cfg(feature = "native-tests")]
pub fn run_native_table_history_test() {
    host::table_view::run_history_test();
}

pub mod document_diff;
pub mod document_diff_controls;
pub mod document_diff_projection;
mod document_diff_syntax;
mod document_diff_words;

pub mod document_markdown;

pub mod document_editor;

#[cfg(feature = "native-tests")]
pub fn run_native_document_test() {
    host::document_view::test::run();
}

pub mod document_search;
pub mod highlight_collect;
pub mod highlight_host;
pub mod highlight_jobs;
pub mod highlight_paint;
pub mod highlight_projection;
pub mod highlight_search;

#[cfg(all(feature = "native-tests", target_os = "macos"))]
pub fn run_native_window_test() {
    host::window_test::run();
}

#[cfg(feature = "native-tests")]
pub fn run_native_tabs_test() {
    host::control_test::run_tabs();
}

#[cfg(feature = "native-tests")]
pub fn run_native_split_test() {
    host::control_test::run_splits();
}

#[cfg(feature = "native-tests")]
pub fn run_native_extension_test() {
    host::control_test::run_extensions();
}

#[cfg(feature = "native-canvas-tests")]
mod canvas_test;
#[cfg(feature = "native-canvas-tests")]
pub fn run_native_canvas_test() {
    canvas_test::run();
}

#[cfg(feature = "native-canvas-tests")]
pub fn run_native_canvas_view_test() {
    host::canvas_view::test::run();
}

#[cfg(feature = "native-canvas-tests")]
pub fn run_native_canvas_input_test() {
    host::canvas_view::test::run_input();
}

#[cfg(feature = "native-tests")]
pub fn run_native_container_query_test() {
    host::container_query_test::run();
}

#[cfg(feature = "native-tests")]
pub fn run_native_presentation_test() {
    host::presentation_test::run();
}

#[cfg(feature = "native-tests")]
pub fn run_native_slider_test() {
    host::presentation_test::run_sliders();
}

pub mod calendar_state;
mod calendar_viewport;
pub mod color_input_state;
pub mod number_input_state;
mod number_presentation;
pub mod otp_edit;
pub mod otp_input_state;
pub mod slider_state;

#[cfg(feature = "native-tests")]
pub fn run_native_number_input_test() {
    host::number_input_view::test::run();
}

#[cfg(feature = "native-tests")]
pub fn run_native_otp_input_test() {
    host::otp_input_view::test::run();
}

#[cfg(feature = "native-tests")]
pub fn run_native_calendar_test() {
    host::calendar_view::test::run();
}

#[cfg(feature = "native-tests")]
pub fn run_native_color_input_test() {
    host::color_input_view::test::run();
}

#[cfg(feature = "native-tests")]
pub fn run_native_navigation_test() {
    host::control_test::run_navigation();
}

#[cfg(feature = "native-tests")]
pub fn run_native_tree_test() {
    host::control_test::run_trees();
}

#[cfg(feature = "native-tests")]
pub fn run_native_hover_card_test() {
    host::control_test::run_hover_cards();
}

#[cfg(feature = "native-tests")]
pub fn run_native_carousel_test() {
    host::control_test::run_carousel();
}

#[cfg(feature = "native-tests")]
pub fn run_native_highlight_host_test() {
    highlight_host::test::run();
}

#[cfg(feature = "native-image-tests")]
pub fn run_native_highlight_paint_test() {
    highlight_paint::native_test::run();
}

#[cfg(feature = "native-image-tests")]
pub fn run_native_highlight_view_test() {
    host::highlight::test::run();
}

#[cfg(feature = "native-image-tests")]
pub fn run_native_highlight_document_test() {
    host::document_view::highlight_test::run();
}

#[cfg(feature = "native-image-tests")]
pub fn run_native_styled_text_test() {
    host::styled_text_test::run();
}

#[cfg(feature = "native-image-tests")]
pub fn run_native_link_test() {
    host::link_test::run();
}

#[cfg(feature = "native-image-tests")]
pub fn run_native_border_style_test() {
    host::border_style_test::run();
}

#[cfg(feature = "native-image-tests")]
pub fn run_native_grid_location_test() {
    host::grid_location_test::run();
}

#[cfg(feature = "native-image-tests")]
pub fn run_native_aspect_ratio_test() {
    host::aspect_ratio_test::run();
}

#[cfg(feature = "native-image-tests")]
pub fn run_native_opacity_factor_test() {
    host::opacity_factor_test::run();
}

#[cfg(feature = "native-tests")]
pub fn run_native_command_binding_test() {
    host::command_binding_test::run();
}

pub mod control_appearance;
pub mod control_geometry;
mod control_paint;

pub mod tab_appearance;

#[cfg(all(test, feature = "native-image-tests"))]
mod horizontal_list_test;

#[cfg(all(test, feature = "native-image-tests"))]
mod document_profile_fixture;

mod window_frame;
