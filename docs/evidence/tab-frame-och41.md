# Tab frame and trailing content — OCH-41

2026-10-02, macOS arm64, base `83eb87e` plus uncommitted milestone work.
Core/Bonsai expose `View.tab_bar_frame` for a direct plain, decorated or structured
tab bar. Independent prefix/suffix views stay outside the native scroller, while
an arbitrary trailing view stays inside it after the logical options. The wrapper
preserves existing viewport/reveal configuration and opts into the default native
viewport when absent. The Navigation gallery now demonstrates a fixed workspace
label/restore control and a separate trailing Restore button.

Stable structural keys separate frame slots from user Choice IDs. Empty fixed
slots occupy no layout space or gap. Changing those slots or tab presentation
preserves unaffected native owners; keep the frame itself mounted. The checked
wrapper rejects non-tab/already-framed inputs and trees over 4096 nodes/128 levels.
Default trailing space is 12 pixels when a suffix exists. Explicit trailing
content remains an ordinary view with its own focus and callbacks.

Paired unpublished epoch-3 Op101 declares the final structural trailing slot.
Native admission checks shape, bounds and tab-only ownership, including late
nonstructural edits. Logical-option validation excludes the trailing subtree
from decorative-label restrictions; the trailing slot itself has no independent
behavior. Rendering measures only the logical choices for reveal and appends
trailing content once. See the [design](../design/rich-tabs.md) and
[bridge contract](../design/bridge-v1.md).

## Behavior evidence

Two Core expect tests cover rejected owners/depth, default viewport/spacing,
disjoint structural/user keys, no-op renders and independent Op101 bytes.
Six exact public transactions replay in both native admission and production-host
tests: initial frame, reorder plus reveal, prefix removal, decorated tabs,
structured tabs and complete disposal. The native admission regression also
rejects wrong-kind metadata, a flag reset without the required shape change,
styled/text-bearing/multiple-child trailing slots and missing option slots,
with unchanged revision/retained bytes after rollback. The codec independently
checks bytes `65000101`, truncation and invalid Boolean values.

The host test uses actual GPUI layout on TestPlatform. It measures fixed prefix/
suffix positions across scrolling, exact tab reveal, correct gap reclamation,
logical accessibility tab counts, independent prefix/trailing activation,
Tab-focus reveal of the trailing control, retained focus owners across all three
tab presentations, idle frames and cleanup.

This found two related native geometry issues:

- Absolute focus-marker children measure the padding box. Focus reveal now
  uses the measured outer box of buttons, links and choice controls, including their borders,
  through a transient paint scope; nested controls restore the enclosing scope.
- Viewport flex defaults could override `Display Hidden`. They now preserve it.
  GPUI can also clamp a hidden/zero-size scroll handle to zero, so the adapter
  retains the last painted offset before layout and restores it when usable
  paint resumes. A strengthened viewport regression cancels a request while
  hidden and verifies the prior offset survives reopening. New requests then
  execute normally. No polling or per-frame bridge traffic is added.

TestPlatform opens no OS window and does not qualify physical keyboard/IME,
VoiceOver or GPU rendering. The physical Navigation driver now checks that the
fixed suffix stays put across an explicit reveal; that addition is authored but
unrun. No new Linux execution is claimed. The all-tabs menu, native indicator/
color motion, flat split groups and broader OCH-41/OCH-17 acceptance remain open.

## Reproduction

Use the repository's isolated environment and `GPUIO_JOBS=2`:

```sh
./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe
./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-protocol
./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-native \
  --test tab_trailing --test tab_viewport --test tab_content \
  --test tab_appearance --test control_labels
./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-native \
  --features native-image-tests,native-canvas-tests --lib
./scripts/gpuio exec cargo clippy --offline --locked -j2 -p gpuio-native \
  --all-targets --features native-image-tests,native-canvas-tests -- -D warnings
./scripts/gpuio exec cargo fmt --all -- --check
python3 scripts/test_extension_consumer.py --example gallery --workspace <fresh-local-path>
python3 scripts/audit_component_catalog.py
git diff --check
```

Passed: full OCaml tests/format/gallery build, **344 protocol tests/no skips**,
**712 native tests/two existing private-D-Bus skips**, ten admission/replay tests,
strict all-target Clippy, Rust formatting and a fresh installed-gallery consumer
build (`run=False`). These are local build/TestPlatform checks; physical desktop
and release acceptance remain open.
