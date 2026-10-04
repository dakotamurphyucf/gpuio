# Progress source review (OCH-41)

Pinned Longbridge revision `84f57fdfcb4910623fb0bb7f795b077e249f9271`.
The unmodified [base root](sources/base-progress.rs.txt),
[component exports](sources/component-progress-mod.rs.txt),
[linear bar](sources/component-progress-progress.rs.txt) and
[circle](sources/component-progress-progress_circle.rs.txt) are checksummed in
the source manifest. The archive hash matches `third_party/sources.json`.
This review does **not** mark the progress family complete.

## Linear progress

| Pinned source | GPUIO behavior and remaining work |
| --- | --- |
| Percentage value clamped to 0..100 | `Progress.Value.determinate` accepts a finite fraction in 0..1 and rejects invalid input; native decoding/admission independently validates it. Accessibility exposes percentage units. |
| `loading` ignores the stored number | The `Indeterminate` alternative carries no meaningless number. Native semantics omit a numeric value and completion callback. |
| Optional accessible label | GPUIO requires a nonblank, bounded UTF-8 label. The control remains passive and preserves keyboard focus on click. |
| Foreground color and 20%-alpha track derived from that color | `Style.Foreground` colors GPUIO's fill; `Style.Background` independently styles the track/root. Existing defaults are blue over neutral translucent gray. These are explicit palette differences. |
| Size presets 4/6/8/10px, or custom height; full width | GPUIO uses ordinary width/height styles, with existing 200×8px defaults. Layout/style composition replaces a duplicate preset enum. |
| Pill radii refined onto track and fill | The native adapter now intersects rounded track/fill geometry, including asymmetric radii and short moving segments. Triangle containment checks pass; actual GPU corner acceptance remains pending. |
| Determinate updates animate through theme duration/easing | `View.progress` defaults to immediate updates and accepts an explicit transition; `View.progress_circle` defaults to a 200ms ease-out transition. Native displayed values interpolate separately from immediate semantic targets. |
| Indeterminate changing-length bar, 1s eased cycle; reduced mode centered at 35% width | GPUIO uses a fixed 25% segment travelling across the track over 1.5s, or a centered 25% static segment. Both indicate unknown progress; this motion/artwork difference is deliberate. |

The upstream base `ProgressTrack` and `ProgressIndicator` are unstyled composition
helpers, not additional semantic controls. GPUIO's ordinary root style and native
fill own these roles; avoid duplicate focus/accessibility ownership. Arbitrary
native closures are not serializable public OCaml callbacks.

## Circular progress: implemented locally, acceptance pending

The component module exports **ProgressCircle** as well as the linear bar. The
circle has determinate/indeterminate values, an accessible label, color, size and
ordinary root styling, native value transitions, and arbitrary center children.
It paints a 20%-alpha full ring under the progress arc, using a size-derived
stroke capped at 5px. The source arc starts at angle zero and grows clockwise;
the indeterminate arc changes length over a one-second cycle. The circle's
loading branch does not explicitly check `reduce_motion` in this source file;
GPUIO must nevertheless honor its own reduced-motion and inert contracts.

GPUIO now exposes `View.progress_circle` with keyed center children, native value
transitions and retained ownership. The Feedback gallery includes an editable
center and styling/motion controls. See the [extended progress contract](../design/progress-presentation.md)
for integrated headless checks and outstanding GPU/AX/public-consumer acceptance.
A spinner remains a separate busy indicator with different semantics.

## Inert regression and evidence boundary

Review found the progress host selected artwork using input/timer eligibility.
Inert content therefore lost the bar, contrary to the retain-layout-and-paint
contract. The host now selects artwork using paint eligibility and requests a
static indeterminate bar when inert. Input/focus eligibility stays fenced.

The production View TestPlatform regression fails before the change for a 50%
determinate bar, and passes after it for determinate/indeterminate bars under
own and ancestor inert policy. It verifies retained paint width and identity,
drained frame demand, hidden paint suppression, resume and unmount accounting.
It creates no OS window and establishes no new GPU or accessibility evidence.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native \
  --features native-image-tests,native-canvas-tests --lib inert_progress_retains
```

Earlier [OCH-11 evidence](../evidence/native-progress-och11.md) records actual
linear paint/semantics, native indeterminate movement and public example checks
at its historical scope. It does not prove the new inert repair, circle/transition
features, complete current gallery/installed-consumer acceptance or release
qualification. Linux build/unit/consumer gates remain required; actual Linux
desktop qualification is deferred OCH-47.

The complete feature-enabled native suite passes **450 tests with two existing
ignores** after this repair. Strict native/protocol all-target Clippy, Rust
formatting and the structural catalog audit also pass. These commands remain
headless; no new desktop window was opened for this review.
