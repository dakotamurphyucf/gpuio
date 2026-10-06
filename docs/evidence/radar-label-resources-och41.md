# Radar label resources and placement — OCH-41

2026-10-06, macOS 14.5 arm64, source base `328267a` plus this native regression.
This slice adds qualification; it does not change production behavior, public
APIs, wire schemas, dependency pins or the toolchain.

## Native resource and layout evidence

The hidden production-window chart harness mounts an ordinary SVG Icon in a
retained radar label. It releases the public asset registration before the first
decode/paint. New acquisition fails, but the mounted child's lease remains usable.
Actual GPU readback verifies its inherited green tint, then purple after a parent
foreground update, and verifies that hiding removes those pixels.

The icon initially has no explicit width/height: decoded SVG metadata supplies
its natural 24 × 16 logical size. The test then sets an explicit 36 × 16 size.
Hiding/returning the label retains the mounted image and retired registration;
unmount removes the image owner and returns encoded-asset counts/bytes to the
pre-fixture baseline. The enclosing harness also awaits image-worker shutdown.
These are scoped ownership checks, not process RSS or GPU-memory measurements.

Eight-axis empty-series data moves the same ID-keyed slot through every cardinal
and diagonal position. Independent trigonometric expectations compare the actual
native child bounds with the 40 px radius and 12 px gap. The tolerance is one
device pixel plus floating-point tolerance. Each direction is checked at test
densities 1, 1.5, 2 and the restored physical display scale, waiting for a prepared
plan at that density. GPU pixel readback occurs after restoring the original
scale. Test overrides do not establish real monitor-transition behavior.

An ordinary text label also inherits 12 → 20 → 12 font-size changes from its
parent. Native measurement grows in both dimensions and returns to its original
size. Foreground/size/font changes leave the completed chart-plan count unchanged
in these checks. Styling and measuring retained labels do not need an OCaml
paint/layout callback or a new source publication. This covers concrete inherited
style values; it is not a comprehensive public-theme or font-family audit.

## Commands and outcomes

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests --test native_chart_view --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests --test native_chart_input --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -p gpuio-native --features native-canvas-tests,native-image-tests --lib --tests --locked -j2 -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all --check
git diff --check
```

Both native harnesses pass. The first includes existing label text, capture,
slider, source lifecycle and two-window chart/list streaming checks, finishing
with zero chart worker reservations/workspace bytes. The foreground input harness
preserves the preceding ordinary button/command/AppKit AX/clipping/InputRegion
regressions. Clippy and formatting pass. No acceptance assertions were removed.

The first resource fixture detached a text node without placing it elsewhere in
the retained tree; transactional validation correctly rejected it. The next
fixture attempted an ordinary-node probe for a chart's specialized renderer.
The corrected test retains the text hidden under the root while the icon occupies
the slot, and obtains the known unpadded chart origin from the root probe. Both
fixture failures are retained alongside passing logs; neither was a runtime bug.

[Raw logs](radar-label-resources-och41-logs.tar.gz) and the
[verified manifest](radar-label-resources-och41-manifest.json) preserve those runs.
No new OCaml implementation, gallery build or installed-consumer run is claimed
for this test-only slice. All local build/test processes exited, and no VoiceOver
configuration changed.

OCH-41/OCH-17 remain open. The unrelated two-window streaming check does not prove
two-chart/two-window **label interaction isolation**. That, specialized child
actions, broader theme/font-family and required native accessibility qualification
remain distinct from these resource/layout results. Broader chart/catalog and
macOS release gates also remain. CI run 37502930557 covers production checkpoint
`328267a`, not this subsequent test addition; Linux desktop remains deferred to
OCH-47 with nongraphical Linux checks required.
