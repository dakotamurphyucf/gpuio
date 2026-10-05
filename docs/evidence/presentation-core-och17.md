# Bounded presentation collector — OCH-17

Local checkpoint: 2026-10-05, based on `da49671`, macOS14.5 arm64 / M1 Max,
Rust1.97.1 through the repository's isolated environment. This is core collection
and TestPlatform evidence. **No Metal renderer hook or measured GPUI presentation
workload is delivered by this checkpoint.** VoiceOver remains on the owner hold;
no local OS window, desktop preference or VoiceOver action was used here.

## Behavior and coverage

The default-off `presentation-diagnostics` feature layers on CPU profiling. A
window-scoped session admits at most 128 unfinished submissions, retains at most
4,096 raw results and maintains fixed-size histograms. Callback tickets own no
window, entity, native renderer or drawable. The scoped thread-local guard cannot
move between threads. Host-clock conversion retains conservative input-latency
bounds rather than subtracting unrelated clock epochs.

Eighteen presentation-filtered tests pass, including eight new collector cases,
a matching profiler metadata case and an actual TestPlatform window case. They
cover submission-order settlement despite driver-thread callback reordering,
nested windows/unwind, saturation, trace truncation, histogram overflow, missing
and zero presentation, duplicate callbacks, invalid clocks, matching input
conversion, old callbacks after session replacement and late completion after
window closure. The Window test exercises `Window::present`, observes unsupported
platform draws as `NotSubmitted`, stops admission and verifies window retirement.
The existing presentation tests selected by the same filter also pass.

All thirteen profiler regression tests pass. Locked default-feature native
compilation and locked `presentation-diagnostics` native compilation pass. The
last addition after those builds is a test-only Window case. Two portable vendor
manifest tests pass; rustfmt and `git diff --check` pass. This is not a full native
suite rerun or Linux validation of this new checkpoint.

The maintained GPUI patch is
`2e66c189a9ebb323229fcd48a0fc19b1e26ed06e0a3ad03491423a4a65912eb7`.
Reconstruction from the pinned, hash-verified Zed archive matches the vendor tree
exactly, excluding the local test Cargo.lock. Source hashes, the resolved isolated
test lock and compressed command logs are in [the evidence directory](presentation-core-och17/).
The tracked root Cargo.lock and dependency pins are unchanged.

## Commands and fixture setup

GPUI is an excluded dependency, so Cargo cannot run its dev tests with root
`cargo test -p gpui`. Its upstream SVG tests also compile two font fixtures from
Zed's top-level assets directory, even when a narrower test filter is selected.
Use a fresh isolated workspace with `crates/gpui` copied from `vendor/gpui`, a root
workspace manifest containing that member and resolver3, and the saved test lock.
Copy the two files `assets/fonts/ibm-plex-sans/IBMPlexSans-Regular.ttf` and
`assets/fonts/lilex/Lilex-Regular.ttf` from the hash-verified pinned Zed archive
into that workspace. Reproduce the repository Cargo patches with absolute paths,
pointing the `gpui` patch at the isolated copy. No shared Cargo checkout is edited.
Runtime shaders match the normal application build and avoid requiring the Xcode
Metal compiler. The initial attempts without that feature and the font fixtures
failed during harness setup, before tests; the corrected isolated commands pass.

For the local workspace `scratch/agents/root-20261004-resumed/presentation-test-workspace`:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test \
  --manifest-path scratch/agents/root-20261004-resumed/presentation-test-workspace/Cargo.toml \
  --config scratch/agents/root-20261004-resumed/presentation-test-workspace/patches.toml \
  --offline --lib --no-default-features \
  --features presentation-diagnostics,gpui_platform/runtime_shaders -j 2 \
  presentation -- --test-threads=2
# Repeat the same command with filter profiler::tests: 13 passed.
GPUIO_JOBS=2 ./scripts/gpuio exec cargo check -p gpuio-native --lib --locked -j 2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo check -p gpuio-native --lib --locked \
  --features presentation-diagnostics -j 2
python3 scripts/test_vendor_gpui.py
```

The isolated workspace is a reproducible local fixture, not a production build
input. Normal builds use the committed vendor tree. Next: the maintained Apple
renderer adaptation, platform capability handling and native qualification;
then repeated measured workloads, overhead and resource validation. CPU or
synthetic timestamps alone cannot satisfy those gates.
