fn main() {
    #[cfg(target_os = "macos")]
    {
        let args: Vec<_> = std::env::args().collect();
        let mode = args.get(1).and_then(|arg| match arg.as_str() {
            "--drive-public" => Some("internal"),
            "--drive-desktop" => Some("desktop"),
            "--drive-reenter" => Some("reenter"),
            "--drive-cancel" => Some("cancel"),
            "--drive-remove-source" => Some("remove-source"),
            _ => None,
        });
        if let Some(mode) = mode {
            assert_eq!(args.len(), 3, "--drive-<scenario> PID");
            gpuio_native::drive_native_drag_drop_test(args[2].parse().unwrap(), mode);
            return;
        }
    }
    gpuio_native::run_native_drag_drop_test();
}
