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
