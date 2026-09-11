//! Opt-in native diagnostics for the private foundation executable.
struct Logger;

impl log::Log for Logger {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.level() <= log::Level::Info
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
}
