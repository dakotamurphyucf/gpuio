pub mod extensions;
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
mod document_highlight;
pub mod document_host;
pub mod document_jobs;
pub mod document_store;
mod ffi;
pub mod file_dialog;
mod host;
#[cfg(feature = "native-tests")]
pub fn run_native_animation_test() {
    host::animation_test::run();
}
#[cfg(feature = "native-tests")]
pub fn run_native_animation_program_test() {
    host::animation_program_test::run();
}
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
mod selection;
mod semantics;
pub mod session;
mod style;
mod transport;
pub mod tree;

#[cfg(feature = "native-tests")]
pub fn run_native_ui_test() {
    host::native_test::run();
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

pub mod document_diff;

pub mod document_markdown;

pub mod document_editor;

#[cfg(feature = "native-tests")]
pub fn run_native_document_test() {
    host::document_view::test::run();
}

pub mod document_search;

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
