# OCH-36 color input evidence

## Value foundation — 2026-09-25

This checkpoint implements pure concrete color and hex-draft models in Core and
Rust. It is not native color-picker acceptance. The branch remains
`milestone-5-ui-extensions`; existing dependency/toolchain pins are unchanged.

`test/fixtures/color-values.tsv` contains twelve independently calculated values
from Python's `colorsys` HLS conversion on encoded sRGB, reordered as RGBA then
hue-degrees/saturation/lightness/alpha. Each line has eight tab-separated numbers.
The file includes black/white, all six hue sectors, gray, mixed channels and
transparent/partly transparent values. It is a numerical fixture, not wire bytes.
Both language tests compare each direction against these references (HSLA error
at most 1e-10 per channel; byte results exact).

Both suites also verify 24,576 byte-color round trips through HSLA and hex,
including RGB values in increments of 17 and six alpha values; 10,001 grayscale/
alpha quantization samples with error at most 0.5/255 + 1e-15; hue wrapping and
half-alpha rounding; nonfinite/out-of-range rejection; strict hex grammar and
bounded draft classification. Core additionally tests validated byte construction
and lossless style-color conversion. Transparent color differs from no selection.

Local checks use the isolated macOS arm64 toolchain:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @test/view_api/runtest @fmt
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-protocol --test color_value
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j 2 -p gpuio-protocol --all-targets -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all --check
```

Final command results are recorded with the commit in Linear. Scratch logs are
local evidence, not build inputs. No GUI windows were opened for this foundation.

Remaining at that checkpoint: native policy/child adapter, bounded wire/configuration, retained
bridge, Core/Bonsai/Eio controls, swatches and popup, public example, actual native
input/AX/GPU/lifetime/workload acceptance, capability advertisement, required
macOS/Linux hosted checks, merge and OCH-46 showcase integration. Linux GUI
acceptance remains deferred to OCH-17.

## Native policy checkpoint — 2026-09-25

Ten deterministic `color_input_state` tests now pass on local macOS. They exercise
the real Rust policy type, independently of GPUI drawing: preview/commit/cancel,
achromatic hue memory, explicit hue entry while gray, typed percentages,
composition/invalid drafts, stale interaction callbacks after Set/Reset, guarded
rejection without cancelling active edits, configuration/alpha/empty history,
disabled/read-only and blocked input, discrete interruptions, bounded labels/
palette, shared-config replacement/disposal, revision exhaustion and terminal
fault/close behavior. Each operation returns at most two events and the tests
check event validity and order. These tests do not prove mounted editor/IME,
pointer capture, accessibility or resource cleanup.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-native --lib color_input_state
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j 2 -p gpuio-native --all-targets --features native-image-tests -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all --check
```

The base slider source evaluation also found that AX Increment/Decrement changes
its value without emitting a SliderEvent, and no public method clears its private
drag flag on cancellation. This informs the private adapter integration work;
there is no mounted test result for a color widget yet. No windows opened during
this checkpoint. Paired codecs, retained bridge, native child synchronization,
public control/popup/example and native/hosted acceptance remain pending.
