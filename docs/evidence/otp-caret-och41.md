# OTP caret timing — OCH-41

Local checkpoint, 2026-10-02, macOS, isolated toolchain, worktree based on
`83eb87e`. This uses GPUI TestPlatform and a fake clock; no OS windows were
opened. It does not qualify physical macOS visual timing, keyboard/IME,
VoiceOver or measured application resource usage.

The retained OTP editor now owns a native 500 ms caret timer. Native timing is
separate from the code model, bridge events and logical selection/IME geometry.
Focus and accepted edits restart its visible phase; appearance changes do not.
See the [lifetime contract](../design/otp-caret.md) for eligibility, steady-caret
states and cancellation rules. No public wire change or vendor patch is needed.

Three native tests cover:

- Exact visible/off timing and painted caret quads, unchanged editor snapshot
  and an empty bridge mailbox across ticks. IME range geometry remains available
  in the off phase. Live grouping preserves phase; repeated accepted edits
  cancel old deadlines. Provisional composition is steady with no task, then
  cancellation resumes a fresh visible phase.
- Effective reduced motion, including an already queued deadline before a new
  paint; inactive/reactivated windows; read-only mode; nonempty selection;
  hidden/revealed nodes. Removal cancels the task even with an external strong
  editor reference. Dropping that reference releases the retired editor, proving
  the timer does not retain it.
- Zero-area geometry and a positive-width field fully clipped by its ancestor;
  native and inherited disabled state; window closure and delayed mailbox
  silence. The production Close path explicitly clears OTP owners/timers after
  Session retirement, matching the helper sequence exercised by the test.

The initial close test expected the editor's weak handle to expire immediately.
GPUI TestPlatform can retain its last platform input handler after closing a
window. The host view does expire; the corrected lifecycle assertion verifies
that any editor still held by that external handler has no timer. This does not
claim final platform-input-handler disposal on macOS. Node-removal coverage
independently proves editor release and absence of a timer ownership cycle.

Commands use `GPUIO_JOBS=2 ./scripts/gpuio exec`:

- `cargo test -j2 -p gpuio-native --features native-image-tests --lib --offline`:
  **584 passed, two existing skips**.
- After adding the ancestor-clip assertion,
  `cargo test -j2 -p gpuio-native --features native-image-tests --lib --offline otp_caret_clipping`:
  passed. Production code did not change after the full-suite pass.
- `cargo clippy -j2 -p gpuio-native -p gpuio-protocol --all-targets --features native-image-tests --offline -- -D warnings`: passed.
- `cargo fmt --all -- --check`: passed.
- `dune build -j2 @runtest @fmt examples/gallery/main.exe`: passed.

The catalog source audit and `git diff --check` also pass. These changes are
local and uncommitted; they do not complete OCH-41 or milestone 07.

The native tests establish lifecycle behavior and task ownership; they are not
idle CPU/RSS measurements or actual OS shutdown acceptance. Those OCH-17 gates,
required Linux automated checks and physical macOS gallery acceptance remain
open. Full Linux desktop qualification stays deferred to OCH-47.
