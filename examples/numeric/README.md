# Native numeric controls

Build with `GPUIO_JOBS=2 ./scripts/gpuio build examples/numeric/main.exe`, then
run `_build/default/examples/numeric/main.exe`.

The example demonstrates single and range sliders, horizontal and vertical axes,
and linear and logarithmic scales through the public Bonsai/Eio controller. The
horizontal range includes lifecycle and command controls: drag or keyboard-step either thumb, reset values, read
native state, disable it, and unmount/remount it. Observations never feed an
implicit replacement back to Rust. Numeric editors/steppers and OTP will extend
this example as OCH-34 continues.

`--self-test` opens a local window, uses real correlated bridge commands, checks
stale revisions and leases, wrong-mode/thumb errors, disabled replacement,
remount initialization and closed-window rejection, then closes the window.
The separate Rust `native_slider` suite exercises pointer/keyboard and AppKit
accessibility behavior. No external service or credentials are needed.

On macOS, `python3 scripts/test_numeric.py` launches and closes its own instance,
then uses OS keyboard events and the application's accessibility tree to test
all four slider examples. It checks value/range actions, separate thumb focus,
read-only/disabled behavior, programmatic replacement, remount initialization and
an observation rendered back through OCaml. It uses the same local accessibility
access as the existing editor and presentation automation.
