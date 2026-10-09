# Native numeric controls

Read the adjacent implementation walkthroughs before changing these demos:

- [Sliders](main.md): [main.ml](main.ml), native preview/commit and guards.
- [Numeric editors](number.md): [number.ml](number.ml), draft versus value.
- [Segmented codes](otp.md): [otp.ml](otp.ml), one native editing session and events.

[dune](dune) defines the three independent executables. Run commands below from
the repository root using the isolated [toolchain](../../docs/development.md).
The [platform policy](../../docs/platform-release-policy.md) distinguishes
macOS-first release scope from Linux build and desktop qualification.

Build with `GPUIO_JOBS=2 ./scripts/gpuio build examples/numeric/main.exe`, then
run `_build/default/examples/numeric/main.exe`.

The example demonstrates single and range sliders, horizontal and vertical axes,
and linear and logarithmic scales through the public Bonsai/Eio controller. The
horizontal range includes lifecycle and command controls: drag or keyboard-step either thumb, reset values, read
native state, disable it, and unmount/remount it. Observations never feed an
implicit replacement back to Rust. Numeric editors/steppers and segmented OTP fields have separate executables
described below.

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

On macOS, `python3 scripts/test_number_input.py` launches and closes its own
instance and exercises all three layouts through external accessibility objects
and OS keyboard events. It verifies transient draft feedback, Enter/Escape and
arrow stepping, accessible step buttons, asynchronous observations rendered by
OCaml, read-only/disabled behavior and remounting. It uses the same local macOS
accessibility access as the other desktop test scripts.


## Segmented verification inputs

Build with `GPUIO_JOBS=2 ./scripts/gpuio build examples/numeric/otp.exe`, then run
`_build/default/examples/numeric/otp.exe`. The `Gpuio_eio.Otp_input` controller
manages a six-digit verification field and an eight-character case-sensitive
alphanumeric recovery field, each backed by one native editing session.
Full-width digits/Latin letters normalize to ASCII; paste also removes ASCII
whitespace and hyphens. Invalid input rejects atomically. Selection, clipboard,
undo/redo and IME composition remain native. A filled code is not authenticated.

The main field supports explicit fill/clear/history, masking, disabled/read-only
policy and unmount/remount controls. Masking conceals the painted and accessible
value and disables copying/cutting; application snapshots still contain text.
Seeds are applied once per mount. Observations never overwrite native text.

`_build/default/examples/numeric/otp.exe --self-test` exercises both modes through
the actual bridge: focus, canonical replacement, selection/history, revision and
lease guards, invalid policy/value/selection, disabled/read-only behavior, remount,
the 64-pending-request limit and ordered close behavior. It verifies programmatic commands never emit user Complete events.
The test closes its own window. Native platform composition and queue overload
are separately covered by the Rust `native_otp_input` harness.

On macOS, `python3 scripts/test_otp_input.py` launches and closes its own example.
It exercises real accessibility values/actions and OS selection, deletion and
history shortcuts, asynchronous completion/rejection feedback, secure masking,
policy changes and remounting. It needs the same accessibility access as the
other native test scripts. See the dated
[acceptance ledger](../../docs/evidence/numeric-inputs-och34.md) for recorded
revisions and platform coverage; these walkthroughs do not add native acceptance.
