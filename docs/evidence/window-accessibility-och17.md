# Initial macOS window accessibility focus

Local checkpoint: 2026-09-30, macOS 14.5 arm64. This covers external AX focus
getters and actual OS keyboard routing, not VoiceOver speech or notification
delivery. OCH-17 and the macOS release audit remain open.

## Failure and ownership

Opening a second Settings gallery window while the application was already active
allowed normal keyboard editing but left its editor's `AXFocused` false, including
after ten seconds of polling. The first window reported focus correctly.

The pinned GPUI macOS backend installs its AccessKit adapter after creating the
native window. A newly opened window can already be key at this point, so its
first key-window notification arrived before the adapter existed. AccessKit
0.26.3's `SubclassingAdapter::for_window` initialized host focus to false and
waited for a subsequent key-window change. AccessKit therefore suppressed the
focused accessibility node despite correct GPUI editor focus and AppKit input.

The vendored adapter now initializes this existing state from
`NSWindow.isKeyWindow`. The generic view constructor keeps its documented
before-first-focus behavior. Later key-window updates continue through the
existing GPUI callback, which uses the same native property. There is no synthetic
activation, timer, new editor state or protocol change. The source version,
licenses, reconstruction patch and checksums are recorded in
[adapter provenance](../../vendor/accesskit-macos/GPUIO.md).

## Regression

The Settings two-window public-API fixture now requires:

- The new second window's clicked editor reports `AXFocused=true` and receives
  an actual Command+A / character edit while the first window's value is unchanged.
- Switching to the first window reports its editor focused and the second's
  editor unfocused, with independent actual keyboard input.
- Switching back to the second window restores its focus report and accepts
  another edit, while the inactive first editor reports unfocused.
- Resetting the first value preserves the second value; closing the second
  leaves the first editable and correctly reporting focus.

The regression fails before the patch at the initial second-window assertion and
passes after it. It uses bounded polling for asynchronous paint/AX publication,
checks the actual window at the mouse target, and closes the windows afterward.
The focused run used a 90-second process-group watchdog and exited zero.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/gallery/main.exe
python3 scripts/test_gallery.py --section settings-windows
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native --features native-image-tests --test native_editor --no-run
# Run the Cargo-reported native_editor executable with a bounded watchdog.
```

The native editor regression also exits zero under a 90-second process-group
watchdog: grapheme editing, clipboard, Tab policy, revision guards, resizing,
AppKit accessibility focus/value actions and native `NSTextInputClient`
marked/committed text calls pass. Those direct text-client calls are separate
from real OS input-method candidate-window acceptance.

A freshly staged installed-library consumer with its independently locked native
backend passes the same two-window OS/AX regression and closes normally under a
90-second process-group watchdog. It uses the composed backend's existing
AccessKit path patch, without installing into the opam switch. The source patch
also reconstructs byte-for-byte from the recorded upstream file checksum.

```sh
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --workspace /private/tmp/gpuio-window-focus-20260930-1
python3 scripts/test_gallery.py --section settings-windows --executable /private/tmp/gpuio-window-focus-20260930-1/consumer/_build/default/main.exe
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt
```

The full Dune build/expect/format suite, Python syntax check, source rustfmt,
diff checks, structural catalog audit and native all-target Clippy pass:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j2 -p gpuio-native --features native-image-tests --all-targets -- -D warnings
```

The temporary consumer path is
evidence only, not a build input or clean-machine distribution claim. The complete
Settings walkthrough passed at the preceding warm-row checkpoint; this adapter
checkpoint reruns the focused window regression and native editor coverage.

Disabled Settings row discoverability is a separate open issue; the panel's
current Inert subtree policy still hides disabled fields from AX. These checks
also do not establish Linux GUI acceptance, full multi-window accessibility
semantics or clean-machine distribution.

## Separate hit-test qualification — source review, 2026-10-04

The [diagnostics helper review](../catalog/diagnostics-review.md) identified a
separate routing question. Pinned Component Root installs Base's
`NSWindow.accessibilityHitTest:` forwarding helper in non-test builds; GPUIO's
custom host does not call it. AccessKit implements view-level hit-testing, but
source presence alone does not establish equivalent window-level routing. This
is not a reproduced defect and no Objective-C hook was added from this audit.

When desktop qualification is available, query actual accessibility elements at
known window-content points across multiple windows and overlays; verify that
the result is the expected visible native control. Record that independently of
focused-node getters and keyboard input. The successful focus regression above
must not be relabelled as point-based hit-test or VoiceOver evidence.

## Screen-point routing repaired — 2026-10-04

The formerly open routing question now has a physical reproduction. The
`settings-windows` fixture queries `AXUIElementCopyElementAtPosition` at the
center of each visible workspace-name editor and requires `CFEqual` with the
editor found through the semantic tree. Before the fix, the second window returns
`AXWindow` instead of its `AXTextField`. The former PID/window-only ownership
assertion did not detect that failure.

Production window creation now invokes Base's existing macOS
`install_window_hit_test_forwarder`, which forwards the native window's point
query into its content view. The call is macOS-only and uses the existing pinned,
statically linked implementation. No additional vendor patch, protocol change or
synchronous OCaml input callback was introduced.

After rebuilding the gallery, the same physical fixture passes exact control
identity at the editor's screen point across both windows, initial/switched AX
focus, actual keyboard edits and independent reset values, then closes the second
window and verifies that the first remains usable. The application exits zero.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 examples/gallery/main.exe
python3 scripts/test_gallery.py --section settings-windows \
  --images scratch/agents/root-20261004-resumed/window-hits-images-002
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native \
  --features native-image-tests,native-canvas-tests --lib --offline --locked -j 2
```

Local macOS 14.5 arm64, worktree based on `1cd5de9`. Logs
`gallery-window-hits-001.log` (failing), `gallery-window-hits-002.log` (passing),
`window-hit-build-001.log` and `native-window-hit-tests-001.log` are retained in
the same local session directory. All 919 native library tests pass, with the
two existing private-bus skips. Other controls, overlay routing, VoiceOver speech,
real input-method candidate windows and Linux desktop acceptance remain separate
qualification work.
