# Motion source review

Reviewed 2026-10-05 against Longbridge GPUI Kit
`84f57fdfcb4910623fb0bb7f795b077e249f9271`. The eight unmodified
`base-animation`, `base-motion` and `base-motion-{easing,keyframes,presence,reveal,stagger,timing}`
snapshots in [sources/manifest.json](sources/manifest.json) are the source inputs.
This is a behavior review, not acceptance of every upstream motion primitive.
OCH-25 owns the existing implementation; OCH-41 owns catalog completeness and
OCH-17 owns consolidated macOS release qualification.

## Existing public equivalents

| Pinned source behavior | GPUIO contract and difference |
| --- | --- |
| `EffectTransition`: fade, top/left slide and width/height effects | [`Animation.Config`](../../lib/core/animation.mli) and `View.animate` support these numeric properties, plus right/bottom, corners and multiplicative opacity. Slides here are layout offsets, not a general paint-transform API. The retained wrapper owns motion; arbitrary OCaml style callbacks do not execute each frame. |
| Keyed `transition` / `transition_with_status` | A stable keyed wrapper adopts its first target immediately when initial values are omitted. Later targets start at the last **painted** value. Upstream samples elapsed time at retarget and shortens reversal duration with a reversing factor; GPUIO uses its declared timing from the painted position. This is functional continuity, not identical reversal timing. |
| CSS easing keywords and cubic Bezier | Public `linear`, `ease`, `ease_in`, `ease_out`, `ease_in_out`, `cubic_bezier`. Upstream transition/effect defaults use the polynomial `ease_out_cubic`; public `Config` defaults to linear. These defaults are not aliases. Upstream Bezier output clamps to `[0,1]`; GPUIO permits overshooting easing and clamps each resulting property to its valid domain. |
| Scalar spring with position/velocity continuity | `Spring`, `Timing.spring`, `Stage` and `Program` expose physical stiffness/damping/mass. Upstream uses response period and damping **ratio**: for mass 1, angular frequency is `2*pi/response`, stiffness is its square, and damping coefficient is `2*ratio*frequency`. A ratio must not be copied into our damping coefficient. GPUIO additionally bounds parameters and active duration, and settles exactly at its deadline. |
| Spring `with_travel(false)` / zero response | Upstream immediately adopts the target and clears velocity. GPUIO `Playback.Paused` holds the last painted value; it is **not** equivalent. Immediate target adoption can be expressed with a zero-duration tween/new configuration, but no matching spring travel toggle is exposed. |
| Sequential timed values | Up to 32 typed stages, with matching property sets, explicit initial values for sequences, per-stage timing/delay and native progression. `restart`, `reverse`, pause/resume and cancellation have explicit run identities. Stage events are paint-confirmed bounded batches, not a callback for each frame. |
| Infinite normal/alternate playback | `Repeat.Loop` / `Alternate`, with explicit initial values and positive duration. Shared application/named clocks also support phase-aligned timed repetition across windows; this is an additional GPUIO facility, not inferred from upstream `Timing`. |
| Stagger offsets | `Config`/`Program` initial delays can express bounded per-member offsets without OCaml frame work. Source `Stagger` computes `interval * abs(index - origin)`, with first/last/left-middle/explicit origins and clamping. There is no named public stagger helper; callers must compute and validate the delay. This composition does not imply dynamic shared-layout orchestration. |
| Measured vertical `MotionReveal` | [`Disclosure.Motion`](../../lib/core/disclosure.mli) supplies native measured-height reveal for collapsibles/accordions. Retained outgoing content becomes inert immediately; unmounted content skips closing motion. Supported ordinary vertical flow, hidden/reduced policy and fallback layouts are explicit. There is no general normalized-progress reveal sampler. |
| Enter/exit presence | Component-specific motion exists for disclosure, sidebar, tooltips, overlays, tabs and toasts. Toast exit has bounded delayed dismissal; overlay entry never delays removal. These are separate contracts, **not** a generic `Presence` equivalent. |

The public [motion gallery](../../examples/gallery/motion_page.ml) demonstrates
target retargeting, a tween/spring/tween sequence, playback controls, named shared
repetition, application motion policy and page departure. Native ownership and
decoding bounds are specified in [baseline motion](../design/animations.md) and
[programs](../design/animation-programs.md). `Opacity_factor` adds multiplicative
composition without replacing a widget's base/state opacity; see its
[contract](../design/animation-opacity-factor.md).

## Nested source coverage and unresolved differences

| Source surface | Current disposition |
| --- | --- |
| `Easing::Steps` with JumpStart/End/None/Both | The [stepped-easing contract](../design/stepped-easing.md) adds all four typed positions with the same positive 32-bit count range, explicit zero-progress jumps and constant-time native evaluation. Local API/codec/state tests and the gallery example are separate from physical qualification. |
| `LinearStops` with inferred positions and duplicate-stop jumps | **Missing typed easing surface.** The keyword/Bezier/step API cannot represent arbitrary piecewise linear curves exactly. This review does not defer it or certify easing parity. Resolve bounded encoding, validation, native evaluation and meaningful tests before claiming complete coverage of that surface. |
| Polynomial `ease_in_cubic` / `ease_out_cubic` / piecewise `ease_in_out_cubic` | Public `ease_in_cubic` and `ease_out_cubic` express the exact cubic polynomials using the existing Bezier representation with linear x. `ease_in_out_cubic` adds the exact native piecewise polynomial through paired easing tag 6. The Motion gallery exposes all three; CSS `ease_in_out` remains distinct. The first two have physical gallery evidence; the third has API/encoding/native-state tests and still requires its physical walkthrough. |
| Signed negative delays | **Not exposed:** public delays are nonnegative. Starting later stages or changing targets does not reproduce an arbitrary initial elapsed offset. Keep this difference visible; the source review itself does not authorize a deferral. |
| `Timing` finite iteration counts, reverse and alternate-reverse directions | Once/Loop/Alternate are exposed. `Program.reverse` reverses declared intervals while retaining their timing/delay; it is not the source's `easing(1-progress)` direction rule. Small finite sequences can be expanded within 32 stages, but that is not support for arbitrary `u64` counts or every reverse-easing rule. |
| Generic `Keyframes<T>` with normalized offsets, duplicate offsets, per-segment easing and stable-ID playback | Arbitrary keyframe timelines are explicitly outside the accepted OCH-25 program contract. Numeric stages cover a subset; do not label them complete keyframe compatibility. Changing a GPUIO program can replace a run; upstream stable-ID playback keeps its start time even when values/timing are reconstructed. |
| `Discrete<T>`, generic `Interpolate` / `Lerp`, color/point/bounds and `MotionTransform` values | General object/color interpolation is outside the accepted program contract. `MotionTransform` is a presentation-neutral value containing translation, scale, rotation and opacity; its presence does not itself install transform/hit-test behavior. Canvas transforms and static native extensions are separate capabilities, not generic animated-widget transform parity. |
| `Presence` phases and interrupted exit retention | General exit-presence/shared-layout orchestration is explicitly excluded by the existing program design. Component-specific exit behavior remains required under each component's own contract and evidence. |
| Render-time `MotionValue`/`TimingSample` status/progress, custom easing closures | No synchronous OCaml sampling callback. Bounded native completion/cancellation observations and the native extension SDK preserve the accepted runtime boundary. Pure OCaml state/timers are not substituted for native per-frame motion. |

The exclusions above come from the existing
[OCH-25 contract](../design/animation-programs.md#source-and-acceptance), not from
new scope reductions in this audit. Missing timing/easing conveniences without
such an accepted exclusion remain explicit catalog decisions/work; they must not
disappear behind the family-level phrase “animations supported.”

## Lifecycle, styling, accessibility and evidence

Public constructors reject nonfinite/out-of-range values and duplicate properties;
Rust independently validates serialized input. Native programs preserve painted
position/velocity, invalidate stale prepared samples, fence callbacks by node/run
generation, and release timers/group membership on removal and close. Hidden
independent motion pauses; shared members rejoin group phase. Reduced motion
settles finite runs and leaves repeated runs static without animation wakes.
This differs from upstream keyframe sampling, which returns its final keyframe
under reduced motion, even for repeating timing.

Animating a wrapper does not supply the child's accessible name or semantic role.
Interactive content must retain its ordinary focus, hit geometry and accessibility
behavior. A native AX geometry query is not a VoiceOver navigation/announcement
test. Neither an animation endpoint nor a rendered screenshot proves physical
presentation latency.

Existing evidence is source- and platform-scoped:

- [OCH-25 native/program evidence](../evidence/animation-programs-och25.md)
  covers actual native geometry, retargeting, visibility, cross-window groups,
  controls, bounded 1,024-owner workloads and idle wake accounting. Historical
  debug durations are not current release performance acceptance.
- [`exercise_motion`](../../scripts/test_gallery.py) uses a public installed
  gallery path for intermediate widths, interruption, sequence controls,
  reduced-motion endpoints, shared phase and departure/remount. Its existence
  does not qualify a new binary; exact run evidence belongs in the
  [gallery evidence](../evidence/gallery-och41.md) and current release handoff.
- [Disclosure/sidebar family review](journey-workspace-review.md),
  [overlay review](overlay-review.md), [tab motion](../evidence/tab-motion-och41.md)
  and [toast motion](../evidence/toast-motion-och41.md) retain their own input,
  clipping, styling and resource boundaries.

Linux build/unit/consumer results remain separate from deferred Linux desktop
qualification (OCH-47). Current macOS performance, VoiceOver and hosted release
acceptance remain OCH-17 work. This review does not close OCH-41 or OCH-25's
catalog mapping by treating every nested helper as already implemented.

The [cubic-preset follow-up](../evidence/cubic-easing-och41.md) records paired
public/native fixtures, polynomial sampling, the real-window gallery walkthrough
and independent consumer build for `ease_in_cubic` / `ease_out_cubic`.
The [piecewise follow-up](../evidence/piecewise-cubic-easing-och41.md) records
`ease_in_out_cubic` API, decoder and native-state checks plus the gallery build;
its physical walkthrough remains open.
The [stepped-easing follow-up](../evidence/stepped-easing-och41.md) records all four
step policies, boundary/decoder/native-state checks and the gallery build, with
physical and installed-consumer follow-up still open.
