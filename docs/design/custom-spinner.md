# Custom spinner implementation contract

OCH-41, implementation in progress. This document closes design questions from
the [pinned loading review](../catalog/loading-review.md); the custom spinner API
is **not shipped yet**. Existing `Loading.Config` behavior remains unchanged.

## Public API

`Spinner.Config`, `View.spinner`, the Bonsai specialization and the paired atomic
node operation are implemented locally. The retained native adapter connects image
observations, mask admission, measured SVG raster sizing and paint-driven clocks.
Real macOS GPU/accessibility and public-gallery runtime acceptance remain open.

Provide a dedicated `Spinner.Config` and `View.spinner`/Bonsai specialization,
using the existing native Loading kind and progress-indicator semantics:

```ocaml
module Config : sig
  type t [@@deriving equal, sexp_of]

  val create
    :  label:string
    -> ?icon:Asset.Handle.t
    -> ?easing:Animation.Easing.t
    -> ?period:Time_ns.Span.t
    -> ?animated:bool
    -> unit
    -> t Or_error.t
end

val spinner
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?on_icon_change:(Image.State.t -> 'action)
  -> config:Spinner.Config.t
  -> unit
  -> 'action View.t
```

The icon must be an SVG registration, checked at public construction and native
acquisition. It supplies a monochrome alpha mask tinted by the current inherited
foreground; it is decorative beneath the spinner's one accessible busy label.
An absent/loading/failed source displays the built-in stroke artwork. Optional
icon observations use existing asynchronous image-state delivery, with current
source/handler generations checked at publication and dispatch. They describe
the icon asset, never completion of the application's work.

The new helper defaults to an 800ms cycle, matching the source's period, with
the existing `Animation.Easing.ease_in_out` preset. That preset is GPUIO's cubic
Bézier curve (0.42, 0, 0.58, 1); pinned GPUI's same-named function is quadratic.
This is an explicit visual-policy difference, not an identical phase curve.
Period limits and label validation
reuse Loading's existing bounds. Presets and validated cubic Bézier curves run
entirely in Rust; arbitrary OCaml easing callbacks are excluded. Finite overshoot
can express extra/reversed turns; reduce turns modulo one before conversion to
the rendering angle. Existing `Loading.Kind.Spinner` keeps its 1200ms linear
default and twelve-stroke presentation.

Explicit static/reduced motion produces a recognizable nonmoving indicator.
Inert content remains painted and static; hidden/clipped/transparent content must
not sustain a recurring frame loop. Theme/style changes preserve the clock owner;
label changes also preserve its phase. Changes to period, easing, animated flag
or source restart at phase zero; a loading-to-ready transition or resampling the
same SVG for a new density does not restart it. Hidden, clipped and transparent
intervals pause elapsed animation time. Reduced motion and inert presentation
paint phase zero while retaining the paused owner for later resumption. These
rules are wired into the retained View; real native lifecycle acceptance remains
required in addition to the headless checks below.

Schedule the next frame only after visible, nontransparent artwork actually
paints. A layout-only animation wrapper is insufficient: the existing GPUI
`AnimationExt` requests frames during layout, even when a parent clips the child.
Rendered elements and pending frame callbacks must borrow the retained owner
weakly so removal/window close cannot keep the clock alive. No per-frame OCaml
state update, serialization, SVG parsing or CPU bitmap rotation is allowed.

The retained View now owns the native clock and uses the low-level painter. `spinner_clock::Owner` retains one shared configuration and a
monotonic clock; elements carry configuration-stamped weak drivers. A frame
preparation pass disarms retained owners before layout, and a completion pass
discards intervals for skipped paint. The adapter must invoke both passes,
including deferred content. At most one weak frame callback may remain pending
across rapid updates. Removing the strong owner releases it even if an element
or callback still exists. Atomic tree admission reserves 512 bytes per
owner in addition to its shared spinner configuration and synthesized Loading/image
configuration. Invalid updates leave the previous tree and accounting unchanged.

`spinner_paint::paint` centers and contains the decoded icon, rotates its mask
around the indicator center, inherits foreground/opacity and clips to the supplied
indicator bounds. Missing pixels use twelve native strokes; failed mask painting
returns a report that the adapter translates into deferred image-state handling.
Empty, malformed, clipped and style-transparent calls skip painting and suspend
the driver. The painter requires prior `image_host::image_mask` admission; it
neither acquires sources nor resamples SVGs. The View adapter handles source loading, measured raster sizing and asynchronous
observations, and gives the root its one progress-indicator accessibility label.
It bypasses ordinary image artwork and image roles for this decorative source.
A window-close subscription is installed during binding synchronization, so
never-painted hidden bindings are released even if the View entity is retained.

## Bridge and ownership

Add one atomic spinner setter on native Loading nodes, carrying validated label,
period, animated flag, easing and optional image source. The Core description may
retain an ordinary image configuration for asset ownership and callback generation,
but must not emit an independent `Set_image` operation for a Loading node. The
spinner setter owns both the loading and image state; the legacy loading setter
clears spinner-specific state when the same keyed node changes back.

Acquire the source lease when the transaction is accepted, before a later source
release can race the first paint. Reuse the bounded image worker/cache service and
measured SVG raster-size admission. Wrong-kind/invalid configuration rejects the
transaction; an unavailable/failed source selects fallback normally. Stale callbacks,
source replacement, unmount and window close follow existing image ownership.
The standalone configuration encodes label, animated flag, period in milliseconds,
easing and optional image source, in that order. The Rust decoder bounds the label
before allocation, validates the complete configuration and rejects trailing data.
The public constructor shares Loading's label/period checks and Icon's SVG-only
validation; native acquisition must independently check the actual asset format.
The paired operation is Op65 `Set_spinner` and requires capability bit 58
(`CAP_SPINNER`). The shared required mask is `9223372036854775807` (2^63−1); the
Hello bytes are `0001fcffffffffffffff7f`. An older host fails negotiation.
Same-key transitions back to `View.loading` emit `Set_loading` even when its
loading fields are equal, clearing icon state and the custom clock owner.

## Rendering prerequisite

Pinned GPUI's SVG primitive accepts a transformation matrix, but decodes its own
SVG. GPUIO instead decodes SVG in its bounded background asset workers and paints
the resulting `RenderImage`. The pinned bitmap painter has no transformation
parameter. Directly calling `svg().data(...)` would bypass the established decode
and resource policy; generating a rotated bitmap per frame would add unnecessary
work and cache growth.

Add a narrow GPUI core `paint_image_mask` primitive. It uploads the alpha channel
of an already decoded image to a monochrome atlas entry and paints that entry
through the existing transformed monochrome sprite shader. The cache key is the
image identity and frame, distinct from the ordinary color-image key. Rotation,
foreground and inherited opacity do not create additional cache entries. No new
platform shader or synchronous asset I/O is needed. `drop_image` must retire both
representations. Zero paint opacity/alpha, empty bounds or fully clipped draw calls
should not allocate an atlas tile. Invalid frames/geometry must not introduce
invalid scene data.

Mask upload is bounded by the same admitted raster dimensions, but it consumes
additional atlas bytes if the same image also appears in color. The image service
now admits color and mask representations separately. `image` reserves BGRA bytes;
`image_mask` reserves one alpha byte per pixel. Both return the same decoded image
and reserve every animation frame before painting. Repeated use in the same
window is free; a second representation or window needs another reservation.
Ordinary color-only images retain their existing capacity.

The application-wide limits remain 32 windows, 1,024 window/image/representation
reservations, 4,096 reserved frames and 256 MiB of atlas payload. The decoded cache
has its own unchanged limits. Atlas payload accounting excludes GPU page padding
and allocator overhead, so it is not a measurement of physical GPU memory or RSS.
Admission failure leaves existing reservations usable. Window close, cache
eviction and shutdown release the admitted representations; eviction drops both
GPUI atlas keys once per affected window. The low-level GPUI primitive itself
does not enforce GPUIO admission: component adapters must use the image service.

## Required evidence

Verify alpha extraction independently of source RGB, separate color/mask cache
keys, repeat-frame/rotation/color reuse, inherited opacity and content clipping,
and removal of both representations. Real macOS GPU pixels must verify static and
rotated asymmetric icons at representative densities; a scene test is insufficient.
Reconstruct the exact vendored patch and build an independent consumer.

Then test the complete public Core/Bonsai/bridge/gallery path: constructor and
decoder errors, asset provenance/release/replacement/failure/recovery, latest
callbacks, stable keyed owners, source/timing updates, reduced/inert/hidden and
offscreen idle, multiwindow ownership, and bounded resources after teardown.
Retain required Linux build/unit/private-bus/consumer gates and report native
platform evidence separately. OCH-41 and milestone 07 remain incomplete until the
functional and release acceptance requirements pass.

## Current rendering prerequisite evidence

The core primitive, distinct mask cache key, dual-representation retirement and
transformed scene bounds are now implemented locally. Alpha extraction is checked
against independent BGRA inputs with different colors, alpha and frame dimensions;
invalid frame indices return no mask. TestPlatform scene tests check color/mask
key separation, inherited opacity, translated/rotated tile reuse, invalid and
clipped calls without allocation, and retirement. Both monochrome and subpixel
sprite culling now uses transformed coverage; their original quad/UV geometry
is preserved. Identity transforms retain the original bounds.

The GPUI asset unit checks pass in a separate scratch crate reconstructed from
the same vendored sources, with explicit local source patches and the repository
toolchain. The full feature-enabled native library suite passes 430 tests with
two existing ignores; strict native/protocol all-target Clippy also passes.
The full Rust workspace and a fresh installed-gallery consumer **build** pass.
The consumer uses the current sources and existing toolchain, so this is dependency
integration evidence, not a clean-machine build or spinner runtime acceptance.
No real GPU/window was used by those tests. The cumulative GPUI patch reconstructs
exactly from the verified archive; see the
[core adaptation](gpui-core-adaptation.md#decoded-image-alpha-masks).

The representation-accounting implementation adds five production-service tests
using TestPlatform. They cover both arrival orders, shared animated decoding,
per-window/full-frame charges, loading observers, retired source registrations,
independent entry/frame/byte quota rejection and exact-boundary admission. A warm
cache workload forces eviction and verifies removal of both color and mask atlas
entries, including a second mask-only window and an externally retained pixel
reference. Window close, asynchronous shutdown and application quit with live handles return
reservations to zero. The expanded native suite passes 435 tests with two existing
ignores; strict native/protocol all-target Clippy passes.
These tests do not create an OS window or establish physical GPU behavior.

The public configuration and paired private codec also pass three Core expect
tests and three Rust tests. Independent fixtures cover the 800ms/EaseInOut default
and a static custom SVG with overshooting easing and upward period rounding.
Checks include all non-SVG formats, maximum labels, malformed UTF-8, invalid
Booleans/tags/resource generations, truncation, trailing bytes, time/easing bounds
and application provenance. Foreign handles encode the established image error;
construction performs no acquisition or I/O. Legacy Loading keeps its defaults.
The full Dune test suite and formatting check pass, including rebuilding the
native archive. The full Rust protocol test suite also passes. These checks
exercise configuration and existing integration; they do not render a custom
spinner or establish native accessibility/idle behavior.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune runtest -j2
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @fmt
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-protocol
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j2 \
  -p gpuio-protocol --all-targets -- -D warnings
```

The public component is implemented locally, but **catalog/release acceptance
remains incomplete**. The Presentation gallery adds a scoped circular-arrow SVG,
built-in and failed-icon fallback modes, static/animated controls, 800ms/2s periods
and linear/ease-in-out selection. Native GPU pixels, VoiceOver, complete public
gallery/installed-consumer runtime checks and broader resource workloads remain
required; a compiled gallery or TestPlatform scene is insufficient.

The native owner/painter adds six deterministic timing tests and three
TestPlatform scene/lifecycle tests. Timing checks cover monotonic and large elapsed
time, hidden/layout intervals, label preservation, atomic invalid updates, all
restart triggers, static/reduced policy, rapid update queue bounds, weak teardown
and finite overshooting curves before float conversion. Scene checks cover an
actual quarter-turn transformation with the same mask tile, inherited color and
opacity, no color-atlas upload, inert/static fallback, malformed-image fallback,
clipped/transparent/hidden idle and removed owners. Invalid and empty geometry
skips upload and wake scheduling during the real TestPlatform paint phase.
The expanded feature-enabled native suite passes 444 tests with two existing
ignores; the Rust workspace suite, strict native/protocol all-target Clippy and
formatting checks also pass. These are headless scene and
lifetime checks, **not** Metal pixel readback, VoiceOver, live node-bridge integration
or whole-application resource/idle acceptance.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --workspace --locked -j2 --no-fail-fast
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native \
  --features native-image-tests,native-canvas-tests --lib
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j2 \
  -p gpuio-native -p gpuio-protocol \
  --features gpuio-native/native-image-tests,gpuio-native/native-canvas-tests \
  --all-targets -- -D warnings
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery \
  --workspace scratch/agents/root-20260929-m7-resumed/spinner-mask-installed-consumer
```

The consumer destination must be fresh when reproducing this command. No test
above opens a native GUI. Runtime, physical GPU and required hosted platform gates
remain separate requirements.


## Retained adapter integration evidence

Three production View/Session TestPlatform tests pass: shared mask painting with
inherited foreground, acquisition before source registration retirement, queued
Ready/Unsupported observations, stable-node legacy reset, inert/hidden/transparent
idle and resume, unmount, and window-close disposal with a retained View entity.
A separate initially hidden case verifies that a never-painted spinner releases
its retired source when the window closes. These tests use background workers and
the real adapter, but no OS window or physical GPU.

Core expect tests verify one atomic setter without a separate image/loading
update, current callbacks without redundant transactions, handler retirement and
same-key transitions in both directions. The paired Op65 fixture matches Rust,
including strict decoding and the new capability. A native Session test checks
wrong-kind/invalid updates, whole-transaction rollback, retained-byte admission,
orphan-handler rejection and clearing image ownership on legacy reset.

The final retained-adapter implementation passes the full Rust workspace and
447 feature-enabled native library tests (two existing ignores). The first
workspace run caught two older handshake expectations; both now independently
specify the capability-58 aggregate and encoded Hello bytes. Tree coherence
validation compares borrowed loading fields, avoiding a label allocation on each
validation. Strict native/protocol all-target Clippy also passes. This evidence
remains headless; native desktop acceptance is open.

A fresh installed-gallery consumer build also passes with the public spinner
example (`spinner-view-installed-consumer`, `run=False`). It uses the repository
sources and isolated toolchain; this is not clean-machine or GUI qualification.

The `native_images` executable now contains a dedicated background-window spinner
fixture. It reads physical GPU pixels for an asymmetric red SVG rendered as a
green alpha mask at four quarter-turn phases, inherited half-opacity, a label
update preserving phase and an explicit static reset. It also checks static frame
quiescence and teardown. Its deterministic clock override exists only in tests or
the `native-image-tests` feature. The fixture uses the actual display scale; it
does not establish a multi-density matrix, OS accessibility or input behavior.
**Runtime assertions have not been run** because desktop access is unavailable;
compilation alone must not be reported as successful GPU validation.

Validation of this authored fixture: `cargo test --locked -j2 -p gpuio-native
--features native-image-tests --test native_images --no-run` links successfully.
The final bounded render-readiness wait also passes all-target strict Clippy.
Rust formatting and the structural catalog audit pass. None of these commands
executes the fixture's physical GPU assertions.

Two more production View tests pass on TestPlatform: deferred observations are
suppressed when source, callback generation or mount lifetime changes before
publication; two windows share an SVG registration while keeping independent
1s/2s clocks and close lifetimes. Closing the first releases its bindings without
retiring the second window's source; closing the second releases the retired
registration. These checks use the actual Session/View/image worker adapters,
not an OS window or physical GPU.

The focused public driver is `python3 scripts/test_gallery.py --section spinners`
and is included in `--section all`. It checks native progress-indicator identity,
40px geometry, source failure/recovery, both themes, captured static/animated
pixels, a real Space-key toggle, timing/easing changes and page retirement/remount.
It also supports the independent consumer through `--gallery-section spinners`.
Python syntax/import/CLI checks pass; **the driver has not executed its desktop
assertions**. This is an executable acceptance procedure, not recorded acceptance.

After the additional lifecycle cases, the complete feature-enabled native suite
passes **449 tests with two existing ignores**. Strict native/protocol all-target
Clippy passes. The public gallery driver remains unrun on the desktop.


## Prepared-frame callback ordering correction

A deterministic spinner regression now reproduces the same delivery-order
fault found in progress: prepare at 100ms, deliver the pending callback at 200ms,
then paint at 250ms in a one-second cycle. Before the correction the phase stayed
at 0.1 instead of 0.25 because delivery discarded the prepared interval. Delivery
now preserves that interval until paint consumes it or the paired frame finish
discards it. Pending callbacks still hold weak owners and remain bounded to one.

The regression checks that an omitted frame does not replay its elapsed time on
reveal, and reduced motion remains quiet. Text shimmer checks both Loop and Once.
Preparation alone cannot prove that paint will be omitted; one pending callback
may request paint before finish. The older tests now pair omitted construction
with `finish_frame` and require silence afterward, preserving the hidden-phase
assertions rather than treating callback delivery as a substitute for finish.

Both new tests fail on the previous implementation and pass after the correction.
The full native library passes **468 tests with two existing ignores**. These are
headless timing/lifecycle checks, not fresh GPU, OS-input or whole-application
performance acceptance. Earlier gallery/installed-consumer evidence predates this
private timing correction; no OCaml API, wire schema or dependency changed.

Strict native/protocol all-target Clippy and Rust formatting pass after this
correction. The structural catalog audit also passes; it is not native acceptance.
