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

## Initial mounted channel/palette checkpoint — 2026-09-25

`native_color_input` now opens a real macOS GPUI window and dispatches native
keyboard/pointer events. It passes arrow adjustment, continuous unsnapped hue
preview (134.64 degrees), captured dragging across paints, Escape restoration,
pointer completion with nearest-byte alpha rounding, palette Enter activation,
Tab traversal into Clear, Clear activation, restrictive configuration during a
drag with cancellation/capture release, late release suppression, handler rebinding during a captured drag without
faulting the owner, historical alpha validity, disabled input/focus isolation and entity disposal after removal.
The test checks ordered native observations and protects application shutdown
on assertion failure. This is GPUI-dispatched input, not external OS keyboard or
IME acceptance.

The first pointer test caught a real routing issue: a child canvas hitbox blocked
the parent div's mouse-down handler. The callback now belongs to the painted
track hitbox. A visual readback also prompted replacing a banded sampled rail
with at most six smooth GPU gradients. Light/dark readbacks use explicit inherited
foreground/background styles; full appearance/scale/constrained-layout acceptance
is still pending.

All existing native slider tests pass after widening focus-part IDs to u16:
GPU styles/scales, native AppKit values/actions, capture/minimize/window lifetimes
and three 1,024-owner workload/disposal cycles. Core form metadata now accepts
color inputs. Native calendar metadata admission was corrected to match its
existing Core API; both calendar/color native metadata update/invalid-role
rollback regressions pass. The four bridge suites total 24 passing tests.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @test/view_api/runtest @fmt
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-native --test color_input --test calendar --test otp_input --test slider
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-native --features native-image-tests --test native_color_input --test native_slider
```

Final native all-target Clippy and format checks pass. Light/dark readbacks were
visually reviewed for legible labels/values, smooth ramps and contrasting thumbs.
The CI workflow now builds this test on both platforms and runs it on macOS;
that hosted run remains pending. Checkpoint details also appear in Linear.
Native text editors, runtime commands, popup/example, actual OS AX/IME input, full
color lifecycle/workload tests, capability advertisement, required hosted gates
and merge remain pending. No full OCH-36 or milestone acceptance is claimed.

## Native text editing checkpoint — 2026-09-25

The real-window color suite now passes native Hex/HSLA editing: invalid and valid
drafts, unsnapped 123.456-degree entry, Enter/Escape/blur behavior, synchronized
siblings, preserved spelling after commit, selection/history/composition through
a label change, undo/redo as draft changes, read-only and hidden rejection, and
full owner plus five-child disposal. Oversized native text is rejected; 80
maximum-size replacements stay within the 64 KiB field-history budget and Cancel
clears discarded history. The input fields have visible inherited-color borders
and focus/error feedback.

On macOS, the suite also invokes the actual NSView `setMarkedText` and `insertText`
delegates outside the GPUI window update. Marked text preserves the committed
preview, insertion ends composition and updates it, and Enter commits. These are
actual AppKit text-input delegate checks, not automated interaction with an IME
candidate window. Most keyboard/pointer scenarios still use GPUI event dispatch;
external OS keyboard and complete color AX acceptance remain pending.

A pressure test leaves one queue slot before a required Started/Preview pair. It
confirms neither event escapes, one overload fault appears and the native input
becomes noneditable. Native edit-command lookup/availability remembers the exact
field after focus leaves it. Existing native menu and numeric suites pass after
shared command-router integration, including real NSMenu activation, context Copy
restoration, native AppKit numeric AX/IME, repeat/window lifetimes and three
256-editor workload/disposal cycles.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-native --features native-image-tests --test native_color_input --test native_menus --test native_number_input
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j 2 -p gpuio-native --all-targets --features native-image-tests -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all --check
```

Native all-target Clippy and format checks pass. Light/dark GPU readbacks were
reviewed for field borders, text legibility and alignment. The pure Core/wire
contracts are unchanged in this checkpoint. Correlated color commands,
public controller/popup/example, comprehensive color AX/appearance/lifecycle/
workload acceptance and required hosted macOS/Linux gates remain pending.

## Correlated commands and public controller — 2026-09-25

Local macOS arm64 validation passes for appended command/result tags 18/53,
independently assembled paired fixtures, strict malformed/truncated/guarded
admission, result reservation and window-retirement barriers. Four Rust codec
tests and eight native bridge tests pass; OCaml view/codec expect tests pass.

Native real-window tests now cover pending platform edits before guard evaluation,
marked-text preservation on stale Set, successful Set cancellation/unmark/history
reset, equal-value Set after raw text commit, focus completing the previous field,
policy failures preserving invalid drafts, read-only/disabled commands and rejection
of reads/mutations after overload. Deferred editor notifications do not restart an
edit after programmatic replacement. Existing native color scenarios also pass.

The public `Gpuio_eio.Color_input` controller and Color Studio example pass the
real OCaml/Rust bridge test: 64 pending requests plus one Busy result, lease/revision
guards, historical alpha values under opaque-only policy, explicit Set/Reset/Clear,
focus, hidden/read-only/disabled policy, native observation/reply consistency,
remount seeding, old-controller rejection and ordered window closure. Commands
never manufacture user commit events. This is real-window command integration,
not external OS keyboard or full accessibility validation.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-protocol --test color_input_codec
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-native --features native-image-tests --test color_input --test native_color_input
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 examples/color_input/main.exe @test/view_api/runtest @fmt
_build/default/examples/color_input/main.exe --self-test
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j 2 -p gpuio-native --all-targets --features native-image-tests -- -D warnings
```

The workflow now includes the public self-test on macOS. Hosted execution remains
pending the consolidated milestone run. Popup Apply/Cancel, complete native color
AX/appearance/lifecycle/workload acceptance, capability advertisement and required
hosted macOS/Linux checks/merge remain outstanding.
