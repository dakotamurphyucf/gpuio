fn main() {
    #[cfg(target_os = "macos")]
    {
        if std::env::args().any(|arg| arg == "--native-child") {
            gpuio_native::run_native_window_test();
            println!("GPUIO_NATIVE_WINDOW_RETURNED");
        } else {
            // NSApplication.terminate can exit(0). Require a marker emitted
            // after the entire native test returns, not merely a zero exit.
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .arg("--native-child")
                .output()
                .unwrap();
            eprint!("{}", String::from_utf8_lossy(&output.stderr));
            assert!(
                output.status.success(),
                "native child failed: {}",
                output.status
            );
            assert!(
                String::from_utf8_lossy(&output.stdout).contains("GPUIO_NATIVE_WINDOW_RETURNED"),
                "OS terminated before native assertions/cleanup completed"
            );
        }
    }
    #[cfg(not(target_os = "macos"))]
    eprintln!(
        "GPUIO_NATIVE_WINDOW_NOT_RUN: macOS OS close/quit test; Linux public lifecycle test is separate"
    );
}
