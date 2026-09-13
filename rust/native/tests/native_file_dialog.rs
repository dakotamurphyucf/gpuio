#[cfg(target_os = "macos")]
fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).is_some_and(|arg| arg == "--drive-picker") {
        assert_eq!(args.len(), 5, "--drive-picker PID FILENAME ACCEPT_LABEL");
        gpuio_native::drive_native_file_dialog_test(args[2].parse().unwrap(), &args[3], &args[4]);
    } else {
        gpuio_native::run_native_file_dialog_test();
    }
}

#[cfg(not(target_os = "macos"))]
fn main() {
    println!("GPUIO_FILE_DIALOG_SKIPPED: this harness covers the AppKit adapter only");
}
