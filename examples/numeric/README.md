# Native numeric controls

Build with `GPUIO_JOBS=2 ./scripts/gpuio build examples/numeric/main.exe`, then
run `_build/default/examples/numeric/main.exe`.

The initial example demonstrates a native-owned range slider through the public
Bonsai/Eio controller: drag or keyboard-step either thumb, reset values, read
native state, disable it, and unmount/remount it. Observations never feed an
implicit replacement back to Rust. Numeric editors/steppers and OTP will extend
this example as OCH-34 continues.

`--self-test` opens a local window, uses real correlated bridge commands, checks
stale revisions and leases, wrong-mode/thumb errors, disabled replacement,
remount initialization and closed-window rejection, then closes the window.
The separate Rust `native_slider` suite exercises pointer/keyboard and AppKit
accessibility behavior. No external service or credentials are needed.
