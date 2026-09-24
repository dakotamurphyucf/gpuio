mod appearance;
pub mod asset_cache;
pub mod asset_decode;
pub mod asset_store;
pub mod asset_svg;
mod ffi;
pub mod file_dialog;
mod host;
#[cfg(feature = "native-tests")]
pub fn run_native_animation_test() {
    host::animation_test::run();
}
pub mod image_host;
pub mod list_index;
pub mod list_state;
pub mod mailbox;
pub mod motion;
mod motion_preference;
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
