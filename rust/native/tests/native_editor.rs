// This harness includes the real host's tests so it can exercise native input
// without exposing test-only editor handles as public application API.
fn main() {
    gpuio_native::run_native_editor_test();
}
