mod appearance;
mod ffi;
mod host;
pub mod mailbox;
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
