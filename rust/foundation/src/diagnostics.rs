//! Opt-in native diagnostics for the private foundation executable.
struct Logger;

impl log::Log for Logger {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.level() <= log::Level::Warn
            || metadata.target().starts_with("gpui_wgpu")
            || metadata.target().starts_with("gpuio_foundation")
            || metadata.target().starts_with("gpui_linux")
    }

    fn log(&self, record: &log::Record<'_>) {
        if self.enabled(record.metadata()) {
            eprintln!("{} {}: {}", record.level(), record.target(), record.args());
        }
    }

    fn flush(&self) {}
}

pub fn init() {
    static LOGGER: Logger = Logger;
    if std::env::var_os("GPUIO_NATIVE_LOG").is_some() && log::set_logger(&LOGGER).is_ok() {
        log::set_max_level(log::LevelFilter::Debug);
    }
    #[cfg(target_os = "linux")]
    if let Ok(expected) = std::env::var("GPUIO_EXPECT_BACKEND") {
        let selected = gpui::guess_compositor();
        assert_eq!(selected, expected, "graphical smoke selected wrong backend");
        eprintln!("NATIVE_BACKEND {selected}");
    }
}
