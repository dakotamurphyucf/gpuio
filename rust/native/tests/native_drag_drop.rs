fn main() {
    #[cfg(target_os = "macos")]
    {
        let args: Vec<_> = std::env::args().collect();
        if args.get(1).is_some_and(|arg| arg == "--drive-public") {
            assert_eq!(args.len(), 3, "--drive-public PID");
            gpuio_native::drive_native_drag_drop_test(args[2].parse().unwrap());
            return;
        }
    }
    gpuio_native::run_native_drag_drop_test();
}
