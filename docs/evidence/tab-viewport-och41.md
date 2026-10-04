# Native tab viewport and reveal — OCH-41

2026-10-02, local macOS arm64, base `83eb87e` plus uncommitted milestone work.
Core/Bonsai's three tab constructors accept `Tab_bar.Viewport`, with checked,
stable-ID `Tab_bar.Reveal_request` values. The native viewport owns horizontal
scrolling and non-wrapping, non-shrinking targets. Application styles retain
colors and dimensions, including dynamic states, without overriding those axes.
The public Navigation gallery contrasts selecting the last tab with explicitly
revealing it and retains the existing close/reorder/restore/truncation controls.

Selection updates alone do not move the user's offset. Native compound keyboard
navigation and assistive focus reveal the active target without an OCaml echo;
child focus uses the existing native scroll path. Reveal changes neither focus
nor selection. Positive serials execute once per mounted viewport lifetime;
missing IDs are consumed, reused/older serials are ignored, and withdrawal cancels
an unexecuted application request. Hidden/zero-size viewports wait without polling.
Removing the viewport resets its offset and request history.

Paired unpublished epoch-3 Op100 carries optional viewport/reveal metadata. Native
admission checks the tab owner, bounded IDs and positive serials atomically and
charges the fixed owner plus exact-sized measured-bounds array. Native measurement
uses logical choice order rather than GPUI helper-child indices. No per-frame
geometry crosses the bridge. See the [contract](../design/rich-tabs.md).

## Coverage and fixes

Two Core expect tests validate serials, independent exact Op100 bytes, metadata
updates with no remount or selection write, no-op updates and reset. The Rust
codec independently agrees on `640001010101057461622d37` and rejects every
truncation, trailing bytes, invalid serials and malformed IDs. Native admission
checks tab-only use, rollback, missing targets, retained charges, reset and retired
generations. Existing exact public tab-content transactions remain covered by
the shared replay test; this checkpoint adds no new child shape.

Production-host TestPlatform tests exercise actual measured target/viewport
bounds, native wheel input, selection without scroll, once-only reveal, disabled
targets, keyboard disabled-item skipping, simulated assistive focus, same-batch
reorder/reveal, resize, hidden/zero-size waiting, dynamic styles, removal/reset,
idle scheduling and cleanup. A structured tab with an offscreen editor verifies
native Tab-focus reveal, editor-owned typing/arrow keys, retained focus/draft on
reorder and wheel offsets surviving unrelated changes. A reservation test checks
the implementation's fixed state and per-target bounds fit the admission charge.

Two issues found during validation were fixed:

- GPUI deliberately ignores `Window.refresh()` during paint. A changed reveal
  offset now requests one weak post-paint frame; the tests assert one follow-up
  frame and no recurring idle work after settlement.
- New bulky style-builder temporaries pushed a nested sidebar over the ordinary
  debug-test stack limit. Viewport style preparation now stays in the existing
  nonrecursive `node_style` phase. The sidebar regression and full host suite
  pass with the default stack; no larger stack setting is required.

The physical Navigation driver contains a selection-versus-reveal geometry
walkthrough, authored but unrun. TestPlatform opens no OS window and does not
qualify physical keyboard/IME, VoiceOver or GPU pixels. No new Linux execution is
claimed. The all-tab menu, bar relationships, indicator/color motion and broader
OCH-41/OCH-17 release acceptance remain open.

## Reproduction

Use the isolated repository environment with `GPUIO_JOBS=2`:

```sh
./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe
./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-protocol
./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-native \
  --test tab_viewport --test tab_content --test tab_appearance --test control_labels
./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-native \
  --features native-image-tests,native-canvas-tests --lib
./scripts/gpuio exec cargo clippy --offline --locked -j2 -p gpuio-native \
  --all-targets --features native-image-tests,native-canvas-tests -- -D warnings
./scripts/gpuio exec cargo fmt --all -- --check
python3 scripts/test_extension_consumer.py --example gallery --workspace <fresh-local-path>
python3 scripts/audit_component_catalog.py
git diff --check
```

Passed: full OCaml tests/format/gallery build, **343 protocol tests/no skips**,
**711 native tests/two existing private-D-Bus skips**, eight admission/replay tests
and strict all-target Clippy. Rust formatting, catalog/whitespace audits and Python
driver syntax checks pass. A fresh installed-gallery consumer build also passes
(`run=False`); no physical desktop test was run for this checkpoint.
