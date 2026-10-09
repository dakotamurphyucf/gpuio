# Selectable-list native input — OCH-41

2026-10-03, macOS arm64, milestone worktree based on `83eb87e`. This extends the
[bridge checkpoint](selectable-list-bridge-och41.md) with production Host input.
Public Core/Bonsai/Eio adapters and the standalone-list gallery are still required.
No dependency version, compiler, switch or global default changed.

## Implemented behavior

The native owner retains focus separately from the logical cursor and selected
metadata. Ordered relative keys reduce in OCaml; native code consumes the key
synchronously without storing another selection model. Both list axes, explicit
query-editor navigation, selection modifiers, primary/secondary confirmation,
cancel and independent context requests have production routing. Override commands
retain priority; child editors and IME composition retain native handling.

Matched row clicks and accessibility actions carry the admitted handler and
interaction generation. Row element identity retires a pending mouse-down across
epoch changes. A second click confirms once without toggling selection again;
subsequent click counts do not repeat confirmation. Embedded controls retain their
own input. Root/input disable, clearing and removal retire native focus.

Cursor reveal uses the existing bounded virtual list. A 100k-row test jumps to the
last row with at most four requested rows, then replaces old rows with one row and
an active budget of one while preserving root focus. Logical cursor state does
not create a native eviction veto. Actual child focus/composition retention is
unchanged.

See [the native contract](../design/selectable-lists.md#production-native-input-contract).

## Reproduced adapter issues and scoped fixes

- A linked query placed after its results painted too late for the previous GPUI
  explicit active-descendant method. The Host regression reported Search instead
  of First. GPUI now resolves one fixed-size explicit claim after siblings paint,
  verifies the final owner and restores the claim on prepaint rollback. Both
  sibling orders pass without moving the editor's real keyboard focus.
- Actual AppKit selected setters `[false, true, true, false]` against an unchanged
  selected snapshot emitted two generic Click actions before the fix. The ninth
  AccessKit adaptation, `list-selection.patch`, opts enabled selectable
  ListBoxOptions into ordered desired setters only when they declare both custom
  selection IDs and a CustomAction handler. All four desired values now arrive;
  the setter does not mutate the displayed snapshot. Incomplete contracts and
  other roles retain their existing behavior.

The complete GPUI patch SHA256 is
`08a36db3b2825b153d54a4ea5471625e803058e60ea604bf267f3150d051058c`,
against unchanged pin `a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b`.
All reconstructed GPUI source bytes match the vendored source. The AccessKit
verifier passes 13 original source files, nine exact patches and two license hashes.

## Validation

Use the isolated wrapper `GPUIO_JOBS=2 ./scripts/gpuio exec` for commands below.

- `cargo test --offline --locked -j 2 -p gpuio-native --features native-image-tests,native-canvas-tests --lib --test list_input --test lists --test tables --test tree_input --test accessibility --test session --test tree --test accessibility_busy --test accessibility_list_selection`:
  **805 library tests passed; two existing private-D-Bus tests skipped**. The
  selected admission/session/tree suites passed **43 tests**. Both main-thread
  AppKit harnesses passed.
- `cargo clippy --offline --locked -j 2 -p gpuio-native --all-targets --features native-image-tests,native-canvas-tests -- -D warnings`: passed.
- GPUI `window::a11y::tests` in an isolated copy of the patched crate, using the
  same locked graph, `test-support,gpui_platform/runtime_shaders` and the repository's
  AccessKit/Taffy patches: **20 passed**, including late-owner resolution,
  mismatched owner, rollback, frame reset and existing duplicate-claim guards.
- `python3 scripts/verify_accesskit_macos.py --archive /path/to/accesskit_macos-0.26.3.crate`:
  passed exact reconstruction from the recorded archive checksum.
- `python3 scripts/vendor_gpui.py --archive /path/to/zed-a57ba9b.tar.gz --output /fresh/path`,
  then byte comparison with `vendor/gpui/src`: passed.

- `dune build @runtest examples/gallery/main.exe`: passed all OCaml tests and the gallery link.
- `GPUIO_JOBS=2 ./scripts/gpuio check-fmt` and `git diff --check`: passed.
- `python3 scripts/audit_component_catalog.py`: passed structural source coverage;
  this does not close the documented implementation/acceptance gaps.

- `GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --workspace /private/tmp/gpuio-selectable-host-gallery-20261003`:
  passed a fresh staged installation and outside-checkout gallery/backend build
  with the locked dependencies (`run=False`). It validates integration of the
  changed native dependencies; the future public selectable-list example still
  requires its own consumer check.

The eight new native checks comprise seven production Host TestPlatform tests and
one key-policy test. They cover cursor versus selection, repeated ordered keys,
query caret/composition precedence, child focus, Tab, both axes, pointer/context,
AX actions, stale presses, hidden/disabled/input retirement, Override shortcuts,
both query sibling orders and bounded 100k-row reveal. AppKit fixtures call actual
Objective-C adapter selectors on an NSView without an NSApplication or OS window.

These checks do not establish physical keyboard/IME, external AX notifications,
VoiceOver, GPU or macOS release acceptance. The completed public component still
needs its own gallery/consumer and physical qualification. Linux compilation,
unit/private-bus and consumer checks remain required; actual Linux desktop
qualification stays deferred to OCH-47. OCH-41, OCH-17 and milestone 07 remain open.
