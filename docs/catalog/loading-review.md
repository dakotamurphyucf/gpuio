# Loading family source review (OCH-41)

Review date: 2026-10-01. Longbridge revision
`84f57fdfcb4910623fb0bb7f795b077e249f9271`; exact
[spinner](sources/component-spinner.rs.txt),
[skeleton](sources/component-skeleton.rs.txt) and
[glyph shimmer](sources/component-shimmer.rs.txt) snapshots are checksummed in
[the manifest](sources/manifest.json), under the adjacent upstream license.
This review does **not** mark the loading family complete.

## Spinner

| Pinned source behavior | Public GPUIO behavior / remaining gap |
| --- | --- |
| Default rotating Loader icon | `Loading.Kind.Spinner` paints twelve fading radial strokes. Both are indeterminate busy indicators; their artwork differs. GPUIO adds a required accessible label and progress-indicator role, without inventing a numeric fraction or completion event. |
| `icon(impl Into<Icon>)` | `View.spinner` now accepts an SVG registration through `Spinner.Config`; its alpha mask rotates natively with inherited foreground. Ordinary Loading retains built-in strokes. Public runtime/GPU acceptance remains pending. |
| `ease(native closure)` | `Spinner.Config` reuses `Animation.Easing` presets and validated cubic Bézier curves, evaluated in Rust. Existing Loading remains linear. Arbitrary native closures are not serializable application values; there are no per-frame OCaml callbacks. |
| `color` | `Style.Foreground`, including tokens and inherited/state-local refinements, colors the native strokes. |
| `Sizable::with_size` | Ordinary width/height styles set logical dimensions; the drawing uses the smaller dimension and centers in the bounds. Source preset sizes are not a pixel-size promise for GPUIO. |
| Timing | The source's private speed is 800ms and its default easing is `ease_in_out`; it has **no public speed setter** in this snapshot. GPUIO's existing public period defaults to 1200ms, accepts 100ms..60s and rounds up to whole milliseconds. Preserve/document this existing contract rather than silently changing it during catalog review. |
| Reduced motion | GPUIO renders a recognizable static indicator, with no recurring native animation demand. Explicit `animated:false` has the same static policy independently of the system preference. |

The [custom spinner implementation](../design/custom-spinner.md) now supplies the
SVG slot, easing and public gallery controls with scoped resource ownership,
queued image-state observations and native fallback. The new helper defaults to
800ms with GPUIO's CSS cubic Bézier ease-in-out, while the source's ease-in-out is
quadratic. Existing Loading defaults remain 1200ms/linear. Paired Core/Rust codecs,
atomic native admission and production-View TestPlatform checks pass. Native GPU,
accessibility and gallery/consumer runtime acceptance remain required OCH-41 work;
this source review does not establish them.

## Skeleton

| Pinned source behavior | Public GPUIO behavior / difference |
| --- | --- |
| Full-width, one-rem placeholder | `Loading.Kind.Skeleton` defaults to 160×16 logical pixels. Ordinary sizing styles can make it full-width or another shape; the logical-unit default is deliberate. |
| Theme skeleton color; `secondary()` multiplies alpha by 0.5 | GPUIO uses foreground color for the placeholder. Callers can supply a theme token and a half-alpha color (or root opacity for the whole indicator). No duplicate mutable secondary state is needed. Root opacity also affects borders/backgrounds, so it is not interchangeable with foreground alpha for every styled root. |
| Two-second bounce/ease pulse, opacity 0.5..1 | GPUIO's period is configurable with the same existing 1200ms default as other loading kinds. Its cosine pulse ranges from 0.25 to 0.75 of the foreground alpha. This is a visual-policy difference, not identical timing/paint. |
| `Styled` refinement | GPUIO styles apply to its enclosing element (layout, clipping, borders, backgrounds, foreground and corners). The animated placeholder paint uses foreground and resolved corners. A gradient root background is not automatically the pulsing foreground; do not claim every upstream background refinement animates identically. |

The skeleton is a functional busy-placeholder equivalent, with its paint/style
differences documented. Native style coverage and application examples still need
the applicable consolidated catalog/release checks below.

## Glyph shimmer versus placeholder shimmer

`Loading.Kind.Shimmer` is a rectangular sweeping placeholder. It is **not** the
source's `ShimmerText`. The separate
[`Text_shimmer` API](../../lib/core/text_shimmer.mli) and
[implementation contract](../design/text-shimmer.md) supply shaped-glyph shimmer
on ordinary/selectable text: duration, relative/pixel spread, physical direction,
Once/Loop, highlight color, application appearance, static/reduced-motion policy
and style composition without duplicating text or focus ownership.

The public constructors reject invalid ranges rather than silently clamp them;
the native decoder independently validates them. Highlight/theme colors resolve
at construction and must be recreated when the application's palette changes.
These differences are documented in the linked contract. Scoped native,
public-gallery and independent-consumer evidence already exists there, including
a failing-before visible-layout clock regression. Whole-application performance,
release qualification and capability advertisement remain open. Historical
staged implementation notes are not a statement that the current gallery is absent.

## Inert artwork regression

The review exposed a bug in all three `Loading.Kind` variants. The host selected
the artwork using focus/timer eligibility, which excludes an inert subtree.
Consequently `Style.Inert true` removed the indicator's artwork, contrary to its
public retain-layout-and-paint contract.

The host now uses paint eligibility to retain the artwork and the existing
stricter eligibility to suppress its repeating clock. Inert indicators use their
recognizable static phase; ordinary hidden content still does not paint. Removing
inertness resumes frame demand only for an animated configuration. No callback,
capability, protocol operation or application state was added.

The production-View TestPlatform regression in
[loading_lifecycle_test.rs](../../rust/native/src/loading_lifecycle_test.rs)
failed before the fix on retained Skeleton artwork and passes after it. It checks
all three variants, own/ancestor inert policy, hidden paint suppression, drained
frame callbacks, resume, stable owner identity and unmount accounting. Static and
animated configurations are both included. TestPlatform uses no actual OS window;
this proves scene/clock policy, not new GPU pixels or Accessibility acceptance.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native \
  --features native-image-tests,native-canvas-tests --lib inert_loading_keeps_artwork
```

The complete local native library suite passes **428 tests, two existing tests
ignored**, with both native-test feature sets enabled. Strict native/protocol
Clippy for all targets, Rust/OCaml formatting and the structural catalog audit
also pass. The full library run used a 240-second process-group watchdog and
finished normally; it created no OS window. These commands do not run the native
GUI integration executables or qualify current hosted Linux/macOS behavior.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native \
  --features native-image-tests,native-canvas-tests --lib
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j2 \
  -p gpuio-native -p gpuio-protocol \
  --features gpuio-native/native-image-tests,gpuio-native/native-canvas-tests \
  --all-targets -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all --check
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @fmt
python3 scripts/audit_component_catalog.py
```

## Acceptance still required

The older [presentation evidence](../evidence/presentation-components-och33.md)
records native cycles, size/color/corners, progress-indicator semantics,
reduced/static/hidden idle, minimize/restore and teardown at its recorded scope.
The public Presentation gallery has loading examples and a shared animation
toggle. These do not establish the missing custom spinner configuration or
current native acceptance of the inert repair.

Exercise the implemented spinner API, expanded styles/configuration and inert
policy through the native/public gallery and independent consumer, and measure
application-level idle/frame/resource behavior. Preserve macOS/Linux required
build gates and distinct platform evidence. Full Linux desktop qualification is
OCH-47; no current source review or headless check establishes Linux GUI acceptance.
