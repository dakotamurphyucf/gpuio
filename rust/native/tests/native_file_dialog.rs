#[cfg(target_os = "macos")]
fn main() {
    gpuio_native::run_native_file_dialog_test();
}

#[cfg(not(target_os = "macos"))]
fn main() {
    println!("GPUIO_FILE_DIALOG_SKIPPED: this harness covers the AppKit adapter only");
}
