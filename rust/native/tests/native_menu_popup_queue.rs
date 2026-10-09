fn main() {
    #[cfg(target_os = "macos")]
    gpuio_native::run_native_menu_popup_queue_test();
    #[cfg(not(target_os = "macos"))]
    eprintln!("SKIP: AppKit queued-popup qualification is macOS-only");
}
