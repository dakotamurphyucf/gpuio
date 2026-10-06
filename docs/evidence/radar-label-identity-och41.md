# Radar caption identity and operation storage — OCH-41

2026-10-06, macOS 14.5 arm64, source base `210b374` plus this change.
This is preparatory implementation for [arbitrary radar label Views](../design/radar-label-content.md),
not acceptance of the unfinished public child-content adapter.

Prepared radar labels now retain their source axis ID as `LabelKind::RadarAxis`.
The native text renderer keys captions with that ID instead of the caption-array
index. Pie captions remain distinct. An ordinary source rename or reorder no
longer changes which retained caption identity belongs to an axis. Layout, text
style, original data and selection metadata are unchanged.

The new geometry regression exercises duplicate caption strings, reorder,
rename, empty series, hidden labels and a pie caption with the same text. Existing
bounded placement tests also cover the radar label kind in tiny viewports.

## Broader lint finding and correction

The initial `cargo clippy -p gpuio-native --lib --locked -j2 -- -D warnings`
failed in the protocol dependency on `clippy::large_enum_variant`. After the prior
options expansion, inline `SetChart` made the entire `Op` enum at least 584 bytes;
the next-largest variant occupied 352 bytes. The earlier radar validation used
`--no-deps`, so its native lint success did not cover this dependency diagnostic.
This failure is not attributed to the new caption ID variant.

`SetChart` now stores `Box<Config>`, following existing indirection for other
large operation payloads. Decoding and fixture construction allocate that payload;
the retained tree still owns an independent `Arc<Config>`. This changes only the
Rust representation: bin_prot tags and bytes remain identical and no schema bump
is needed. The existing independent chart fixture and complete protocol suite
verify that boundary.

A local Rust size probe linked to the rebuilt protocol library reports:

```text
operation_bytes=352 chart_config_bytes=568
```

That is 232 fewer bytes per allocated operation slot on this target, approximately
40% of the former enum size. A chart-setting operation now additionally owns its
boxed payload. These type sizes are not whole-process memory or frame-latency
measurements, and they are not asserted as a stable cross-platform ABI.

## Validation

Executed using the repository's isolated environment and two build jobs:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-protocol --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests --lib --test chart_tree --locked -j2 chart
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -p gpuio-native --features native-canvas-tests --lib --tests --locked -j2 -- -D warnings
cargo fmt --all --check
git diff --check
```

The full protocol suite passes 431 tests, including independent byte fixtures,
malformed/truncated decoding and chart transaction encoding. All 98 chart unit
tests and both chart-tree tests pass. Strict Clippy, including dependencies and
the native-canvas test constructors, passes; formatting, whitespace and the new
documents' local links pass. No local desktop window was opened
for these changes. Root/installed custom-label interaction, accessibility and Linux
desktop qualification are not claimed. The existing hosted run `37470525492`
covers older head `00f43c8`, not this change; its Linux job has passed and macOS
validation is still running.
