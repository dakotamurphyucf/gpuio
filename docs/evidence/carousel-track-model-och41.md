# Measured carousel model and payloads — OCH-41

2026-10-02, macOS arm64, branch `milestone-07-gallery-release`, base `83eb87e`
plus local milestone changes. Stock OCaml 5.3/Bonsai v0.17/Core/Eio and existing
Rust/GPUI pins remain unchanged. **The public track view is still unimplemented.**
This checkpoint extends the [geometry foundation](carousel-track-foundation-och41.md)
and [design](../design/carousel-track.md) with a value model and checked payloads.

`Carousel_track` reuses validated carousel IDs/items while adding measured-stop
navigation. The application owns one selection and an optional layout observation.
Relative actions reduce against the latest stop map; explicit selection still
addresses logical items with duplicate snap positions. Layout availability and
epochs do not themselves change selection. Reorder, axis and loop-policy changes
advance collection lineage and invalidate layout; same-order payload updates
preserve it. Disabled models still process observations.

Layout and automatic proposal values retain private window/node/handler identity.
A remounted native source can restart its epoch; nonincreasing epochs from the
same source cannot replace newer geometry. Automatic advancement also checks the
model revision, geometry epoch, source item and measured successor. The future
bridge must validate live generations before conversion: these pure value tests
do not claim that the unimplemented event dispatch path already does so.

`Carousel_track_wire` and Rust `carousel_track` define standalone config, layout,
stop-map and proposal/request payloads. The checked Rust decoders bound strings,
lists and total payload bytes before allocation, reject invalid canonical indices,
unknown variants, negative counters, truncation and trailing data. Four hand-written
bin_prot vectors are checked against both language encoders and the Rust decoder.
The geometry module now directly produces the bounded stop-map payload.

There is no new V1 operation/event tag, capability advertisement or native track
presenter yet. Motion/view configuration, tree admission, generation-fenced dispatch,
native gestures/clock/focus/AX and a public gallery example remain required.

Four Core expect tests cover measured/queued navigation, duplicate and explicit
selection, disabled behavior, unavailable/stale geometry, collection lineage,
remounting, automatic proposal fences and paired payload validation. Four Rust
protocol tests cover independent vectors and malformed/bounded transitions.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-protocol
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j2 \
  -p gpuio-native --features native-image-tests,native-canvas-tests --lib
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --offline --locked -j2 \
  -p gpuio-native -p gpuio-protocol --all-targets \
  --features native-image-tests,native-canvas-tests -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery \
  --workspace scratch/agents/root-20260929-m7-resumed/carousel-track-model-installed-gallery
```

All checks pass: **337 protocol tests**, **668 native tests with two existing
private-D-Bus skips**, strict Rust lint, full OCaml tests/formatting and gallery
build. The fresh installed-library gallery consumer passes with `run=False`.
An additional untracked pure-model executable in that consumer compiled and ran
against the staged installed libraries, reporting `GPUIO_INSTALLED_TRACK_MODEL_PASS`.
It imported the new public model/payload modules, applied a layout and two relative
requests, and verified duplicate-stop behavior. This is package/API evidence;
the reusable model scenarios live in the versioned expect tests.

No OS windows were opened. Physical macOS, required Linux automation and broader
catalog/resource/distribution/release acceptance remain open. OCH-41 and milestone
07 are not complete.
