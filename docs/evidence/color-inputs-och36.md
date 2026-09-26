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

## Core contract and standalone codec checkpoint — 2026-09-25

Compiled Core `Color_input` interfaces now validate labels, bounded palettes,
revisions and imported snapshots; observations are separate from explicit commands.
Rust and OCaml independently encode/decode these manually assembled fixtures:

| Fixture | Bytes | Meaning |
| --- | ---: | --- |
| `color-config.hex` | 96 | Accent labels, two palette colors, alpha allowed, empty allowed, read-only |
| `color-preview.hex` | 69 | Revision 8, text interaction 5, red at half alpha preview, green committed, valid hex draft |
| `color-set.hex` | 9 | Set `#11223344` guarded by revision 7 |
| `color-failed.hex` | 2 | Failed Stale_interaction response |

The reference bytes were assembled with Python `struct` and manual bin_prot
tags/length prefixes, without invoking either production encoder. Packed RGBA
uses integer tags `FD` (signed 32-bit little endian) or `FC` (signed 64-bit little
endian) where required; HSLA is little-endian binary64. Positive integers below
128, variant/option tags and these short UTF-8 string lengths are single bytes.
The preview's HSLA tuple is exactly `(0, 1, 0.5, 0.5)`; the stored RGBA alpha is
128 after nearest-byte rounding. Fixture files are versioned under `test/fixtures`.

Local Core expect tests pass fixture agreement/full consumption, public accessors,
identity retention, invalid UTF-8/label/palette/revision guards, malformed snapshot
and draft rejection, 1,805 public/wire color-conversion cases and the maximum
97,308-byte configuration. Rust codec tests pass every fixture truncation, tags,
trailing data, all channel/interaction/draft/source/cancel/error variants,
semantic contradictions, nonfinite channels, maximum payloads, excess lengths
and huge length declarations rejected before allocation. The ten native policy
tests still pass under the stricter snapshot validation. These checks remain
independent of mounted GPUI behavior.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @test/view_api/runtest @fmt
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-protocol --test color_value --test color_input_codec
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-native --lib color_input_state
```

Final lint/format results are recorded with the checkpoint in Linear. Outer
protocol tags, retained admission/routing, native controls and Bonsai/Eio runtime
controllers remain pending. No color capability or GUI acceptance is claimed.

## Retained bridge checkpoint — 2026-09-25

Core `View.color_input` and reconciliation now connect kind 41 and operation 47
to the retained native tree, with observation envelope 52 routed into Eio dispatch.
Two Core expect tests and six native integration tests cover independently
assembled full request/event fixtures (123/78 bytes), bounded decoding, stable
seed/configuration history, callback refresh, invalid observations, stale identity
and tree revisions, remove/remount/close, retained storage release, preview
coalescing, ordering barriers and atomic count/byte overload rejection. A full
queue retains the previous preview when the next required pair cannot fit.
Same-transaction restrictive reconfiguration preserves the initially admitted seed.

Local isolated macOS arm64 checks:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @test/view_api/runtest @fmt lib/eio/gpuio_eio.cmxa
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-native --test color_input --test calendar --test otp_input --test slider
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j 2 -p gpuio-native --all-targets --features native-image-tests -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all --check
```

Core checks and 22 Rust integration tests pass. Final lint results accompany the
checkpoint in Linear. No native window was opened; this is bridge acceptance,
not mounted color-widget acceptance. Native controls, correlated runtime commands,
popup/example, actual macOS interaction/AX/GPU/lifetime checks, capability
advertisement and consolidated hosted gates remain pending.
