# Native numeric controls

Build with `GPUIO_JOBS=2 ./scripts/gpuio build examples/numeric/main.exe`, then
run `_build/default/examples/numeric/main.exe`.

The example demonstrates single and range sliders, horizontal and vertical axes,
and linear and logarithmic scales through the public Bonsai/Eio controller. The
horizontal range includes lifecycle and command controls: drag or keyboard-step either thumb, reset values, read
native state, disable it, and unmount/remount it. Observations never feed an
implicit replacement back to Rust. Numeric editors/steppers have a separate executable described below. OTP remains
under implementation in OCH-34.

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

## Numeric editors and steppers

Build with `GPUIO_JOBS=2 ./scripts/gpuio build examples/numeric/number.exe`, then
run `_build/default/examples/numeric/number.exe`. The public
`Gpuio_eio.Number_input` controller owns observations and commands for Sides,
Stacked and Hidden step-control layouts. Each field shows its draft separately
from its committed value. Enter commits and normalizes; Escape restores the last
committed value; Up/Down step; visible buttons support pointer hold-repeat.

The primary field demonstrates explicit commit, restore, value replacement,
state reads, disabled/read-only policy and unmount/remount. Native observations
never feed an implicit text replacement back into the editor. A remount starts
from the supplied initial value; the old controller lease cannot edit it.

Run `_build/default/examples/numeric/number.exe --self-test` to open a local
window, exercise the public commands and asynchronous event path, and close it.
The test covers all three layouts, initial/committed/draft values, clamping,
transient and syntax rejection, empty-required handling, empty stepping, history,
UTF-8 selections, oversized/invalid text, revision and lifetime guards, policy
changes and closed-window rejection. A deliberately unplaced controller checks
`Not_mounted`. Focus denial uses `Focus_blocked`; disabled stepping uses `Disabled`.
The application self-test is a bridge/lifecycle check, not simulated OS keyboard
or accessibility evidence; the Rust native suite separately invokes those paths.
