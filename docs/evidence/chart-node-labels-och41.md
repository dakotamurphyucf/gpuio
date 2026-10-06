# Rich Sankey node labels — OCH-41

Implementation based on `1d623f7`, 2026-10-06, macOS 14.5 arm64 / M1 Max.
Scoped local core/protocol/native checks, the root gallery and the independently
installed gallery walkthrough pass. The first installed launch failed as described
below; this checkpoint does not establish startup reliability. See the
[contract](../design/chart-node-labels.md) and
[beginner sample walkthrough](../../examples/charts/samples/sankey_presentation.md).

The public OCaml `Chart_node_labels` API supplies bounded, immutable caption
lines keyed by typed node IDs, with optional theme-resolved color and font size.
An absent override retains the source label; an empty entry hides its plotted
label. Source names, values, geometry and selection identity remain unchanged.
The worker expands labels before admission and accounts for their storage.
Style schema -2 appends the collection; decoder bounds become 64/66 KiB for
style/view. Options/data remain 4/1 and both bridge packages must match.

## Local checks

- Public expect tests cover constructors, themes, invalid IDs/counts/text/fonts,
  total text bounds, style composition and independent wire bytes. The new paired
  view fixture is `chart-v4-node-labels-view.hex`; old style -1 is rejected.
- The full protocol suite passes **428 tests**. The new tests cover independent
  bytes, truncation, trailing data, invalid UTF-8 and a maximally populated valid
  label/ordinal/style envelope. The final three focused tests pass after the
  initializer-only lint correction described below.
- The complete native suite passes **1011 tests, with two existing skips**. New
  tests check ID-based overrides after reorder, missing IDs, explicit hiding,
  global label suppression, retained text charges, unchanged source geometry,
  requested font/color and nonoverlapping block clipping down to a 1×1 viewport.
- Full `@all @runtest @fmt`, strict Clippy and rustfmt pass.
- The rebuilt root gallery passes all chart families/directions/presets, raw
  keyboard selection, updates, original-data browsing and scope teardown. Rich
  labels change 6171 sampled plot pixels from default; Hide target changes 3927.
  The custom source/target captions appear, and the hidden target caption is
  absent from the native accessibility tree. Keyboard selection retains 100 and
  the tiny widened flow retains 0.01; publication changes 100 to 101. The original
  table still contains five node/edge rows. Resource counts return to zero.

The rich-label screenshot was inspected: teal 22-pixel and amber 18-pixel
headlines have separate default-size second lines, with no visible overlap.
This is scoped visual/native event evidence, not VoiceOver qualification. The
root executable SHA-256 is
`e45d806d878a131badb66d40814583deddc36bf8d74cdb4afa6b4e8094a7436b`.

The fresh consumer build and `--check-catalogs` pass. Its initial GUI launch
remained alive but exposed no AX windows before the 35-second lookup expired.
No application error was printed. The driver terminated and reaped its child.
Rerunning the exact installed binary with the chart driver passes the complete
walkthrough, including both new label presets, raw-value selection, source updates,
original-data rows and zero-resource teardown. No implementation change occurred
between these launches, and the initial absence has no confirmed cause. Do not
attribute it to user interaction or count the original combined command as passing.
The installed executable SHA-256 is
`e919273e2690c2de49ae0d3a5f016d489b743c820fc5f045a9179b4a7693d1fe`.

[Evidence archive](chart-node-labels-och41/evidence.tar.gz) and
[per-file checksums](chart-node-labels-och41/manifest.json) retain raw successes
and failures, source overlay, root/installed screenshots and exact commands.

The first root gallery attempt ran the previous executable while the full build had
not finished linking. Its hash was `1e0e93af8274447cfe615f787d992778cfabdbbef3c65143f36a172e93becf6b`;
its AX tree contained only the five old presets and it timed out seeking the
new button. After the build exited successfully, the completed binary passed.
This was a test sequencing error, not evidence of a missing runtime control.
The initial Clippy check rejected field reassignment after `Style::default()`
in the maximum-metadata test. A struct initializer fixed it without changing the
test values; focused tests and the final strict lint pass. No expectations were
automatically promoted.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-protocol
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-protocol --test chart_node_labels
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native --lib --features native-image-tests,native-canvas-tests
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j2 -p gpuio-native -p gpuio-protocol --all-targets --features native-image-tests,native-canvas-tests -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all -- --check
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt
python3 scripts/test_gallery.py --section charts --images scratch/agents/root-20261004-resumed/chart-node-labels-images
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --run --gallery-section charts --workspace scratch/agents/root-20261004-resumed/chart-node-labels-consumer
python3 scripts/test_gallery.py --executable scratch/agents/root-20261004-resumed/chart-node-labels-consumer/consumer/_build/default/main.exe --section charts --images scratch/agents/root-20261004-resumed/chart-node-labels-consumer-images
python3 scripts/audit_component_catalog.py
python3 scripts/audit_example_docs.py
git diff --check
```

Outer-column margins/above-middle label placement, source-to-target ribbon
gradients and broader chart options remain catalog work. Whole-catalog,
VoiceOver, optimized performance, Linux GUI and release acceptance remain open.
Hosted run 37447717604 covers older head `548bcde`, not these changes.
