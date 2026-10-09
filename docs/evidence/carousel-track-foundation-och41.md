# Measured carousel geometry foundation — OCH-41

2026-10-02, macOS arm64, branch `milestone-07-gallery-release`, base `83eb87e`
plus local milestone changes. Existing dependency pins remain unchanged.
**This is a foundation checkpoint, not a completed public carousel-track widget.**
See the [implementation design](../design/carousel-track.md).

The axis-independent geometry module checks bounded finite intervals, computes
measured snap targets and canonical stop indices, skips duplicate end stops,
handles unequal sizes/gaps/insets and distinguishes finite, continuous-loop and
boundary-jump presentation. Six pure tests include a periodic-coverage comparison
across a matrix of widths, gaps, viewports and intermediate offsets. The jump
policy preserves single native ownership in layouts where a continuous wrap
would need two visible fragments of the same item.

One GPUI TestPlatform probe uses real `ScrollHandle` layout measurements on both
axes. It verifies unequal extents, gaps, padding-aware scroll limits, unchanged
unscrolled bounds after offset changes, remeasurement after resize and retained
focus handles. It does not go through GPUIO's bridge, managed focus policy or a
public OCaml track constructor. No physical OS window or GPU-pixel acceptance is
claimed.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j2 \
  -p gpuio-native --features native-image-tests,native-canvas-tests --lib carousel_track
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --offline --locked -j2 \
  -p gpuio-native --all-targets --features native-image-tests,native-canvas-tests \
  -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j2 \
  -p gpuio-native --features native-image-tests,native-canvas-tests --lib
GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all --check
git diff --check
python3 scripts/audit_component_catalog.py
```

All seven focused tests pass. Full native suite: **668 passed, two existing
private-D-Bus skips**. Strict Rust lint, formatting and whitespace/source audits
pass. No OCaml or wire code changed at this checkpoint; those suites and the
installed consumer were not repeated for the unconnected geometry foundation.

The typed OCaml model, layout observation transport, native measured presenter,
gesture/clock/focus/AX integration, public gallery and consumer/physical evidence
remain required OCH-41 work. The original page carousel remains unchanged.
