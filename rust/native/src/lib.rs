mod ffi;
mod host;
pub mod mailbox;
mod selection;
pub mod session;
mod style;
mod transport;
pub mod tree;

#[cfg(feature = "native-tests")]
pub fn run_native_ui_test() {
    host::native_test::run();
}
