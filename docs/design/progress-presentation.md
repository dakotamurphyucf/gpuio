# Extended progress implementation contract

OCH-41 implementation contract; local implementation, not release acceptance. The
[pinned review](../catalog/progress-review.md) identifies circular progress,
native value transitions and rounded moving-edge styling as required gaps.
Existing `Progress.Config` and `View.progress` remain supported. The typed
`Progress.Transition` and standalone paired presentation codec are implemented
locally. Core/Bonsai constructors, atomic Op66, native ownership/painter and a
Feedback gallery preview are connected. The signatures below are the public API;
current verification and remaining acceptance are recorded at the end.

## Public interface

Reuse `Progress.Config` for the semantic label/value shared by both shapes.
Do not duplicate label validation or introduce a second percentage type.

```ocaml
(* Inside Progress. *)
module Transition : sig
  type t [@@deriving equal, sexp_of]
  val immediate : t
  val tween
    :  ?easing:Animation.Easing.t
    -> Time_ns.Span.t
    -> t Or_error.t
end

val progress
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?transition:Progress.Transition.t
  -> config:Progress.Config.t
  -> unit
  -> 'action View.t

val progress_circle
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?transition:Progress.Transition.t
  -> config:Progress.Config.t
  -> 'action View.t list
  -> 'action View.t
```

The matching Bonsai constructors specialize child actions to effects. Existing
linear calls default to immediate updates. The new circle defaults to a 200ms
ease-out value transition; callers can select immediate or a checked tween.
Tween durations are positive, round upward to milliseconds and are at most 60s;
easing reuses validated `Animation.Easing` presets/Bézier curves. This is a GPUIO
policy, not a promise to copy the styled source's mutable theme motion tokens.
The constructor shape above is implemented; native public acceptance remains
required before marking the catalog family complete.

Circle sizing uses ordinary styles, with a 32×32 logical-pixel default. It centers
an inscribed ring in the available paint bounds; rectangular bounds do not stretch
the ring. Stroke thickness derives from the smaller dimension (15%, capped at
5px), and empty/invalid geometry must not emit invalid paths or request frames.
Foreground colors the arc; the track uses 20% of its alpha. Background, borders,
padding, clipping and root corners retain ordinary style meaning. Center children
are real keyed views above the ring, centered by default and independently styled.
Do not bake a percentage text into the native painter. Respect both root clipping
and child layout; no per-frame layout/child remount is needed to animate a ring.

For linear progress, preserve existing palette and size defaults. Refine the fill
corners from the root while clipping to the track, including asymmetric radii and
short/zero/full fills. A fill's moving edge must not require a wrapper that changes
the public semantic node or its size.

## Semantic and lifecycle rules

- The root remains one passive progress indicator. Determinate accessibility
  reports the latest target percentage immediately, even while artwork approaches
  it. Indeterminate progress omits a numeric value. Labels may update without
  resetting motion. No completion event is inferred from reaching 100%.
- Center text and controls retain their own semantics, keys and native state.
  The root itself does not take focus; interactive children retain normal input
  behavior. Inert/hidden policy applies to the full subtree. Avoid a duplicate
  image or progress role for decorative canvas artwork.
- Initial determinate mount paints the target, without a fabricated 0%-to-target
  intro. Subsequent target changes start from the current displayed fraction,
  including interruptions, and reach the exact target at the bounded deadline.
  Clamp eased fractions into 0..1, including finite overshooting curves.
- Label/style changes preserve transition state. Transition-policy changes
  retarget from the current displayed value. Entering indeterminate begins its
  native cycle; leaving it paints the first determinate target directly rather
  than treating the decorative segment as a measured fraction.
- Reduced motion and inert presentation snap determinate artwork to its target
  and use recognizable static indeterminate artwork. Hidden/clipped/transparent
  content must not sustain frame requests. Determinate updates while hidden use
  the latest target on reveal; indeterminate cycles pause during omitted paint.
- Native owners retain transition/cycle time and schedule only while visible
  artwork needs another frame. Do not serialize OCaml updates, parse assets or
  allocate geometry proportional to elapsed time on every frame. Pending wakes
  and rendered elements borrow owners weakly. Unmount and close release all
  owners, including when an entity or a stale element is externally retained.

## Bridge and native work

Use the existing native Progress kind with an atomic extended presentation
operation carrying semantic configuration, shape and transition policy. Op66
`Set_progress_presentation` and capability bit 59 now identify that operation. Independent fixtures, bounded decoding and whole-tree
validation must reject malformed configurations without changing the accepted
revision/accounting. Charge each retained native motion owner explicitly.

Legacy `Set_progress` restores linear/immediate presentation. Same-key shape
changes preserve the semantic root; transactions must remove circle children
before the final linear tree is validated. Ordinary linear Progress remains a
leaf. Circle center children participate in regular reconciliation, focus,
accessibility, asset and cleanup rules. No synchronous OCaml callback is allowed
from native layout, paint or platform accessibility.

Build on existing native easing/clock and path primitives, while keeping semantic
target and displayed fraction separate. Reuse shared timing helpers where the
contracts agree; do not shoehorn progress into the spinner's source-restart
configuration. Native canvas tessellation must use fixed bounded complexity.

## Required acceptance

Implement public interfaces/codecs, retained native ownership/painter and the
Feedback gallery example before claiming parity. Verify zero/tiny/full fractions,
invalid inputs, interrupted transitions, current semantic target, immediate/
reduced/inert modes, hidden/offscreen idle, stable center input across updates,
shape transitions, multiple windows and teardown. Native GPU checks must cover
arc direction, ring/track alpha, size/scale, corners and clipping. Public gallery
and fresh installed-consumer checks must exercise actual keyboard and accessible
center controls, not just screenshots. Retain Linux build/unit/private-bus/consumer
gates with graphical evidence separate; OCH-47 remains deferred. OCH-41/OCH-17
remain incomplete until their full acceptance requirements pass.

## Configuration wire contract

`Progress_wire` now holds the existing label/optional-fraction payload, with
`Wire.Progress` as an alias. Its bytes are unchanged. The standalone extended
presentation appends shape (`Linear=0`, `Circle=1`) and transition (`Immediate=0`,
`Tween=1` followed by duration in milliseconds and existing easing). Rust's nested
`ProgressConfig` serializes the same semantic prefix. It reuses existing label and
fraction validation; there is no second numeric convention for circular progress.

The Rust decoder bounds labels before allocation, checks complete configuration,
rejects trailing data and caps standalone input at 16,384 bytes, allowing the
maximum label alongside a 256-stop easing curve. Independent linear
and circle fixtures are under `test/fixtures/progress-presentation-*.hex`; they
were specified separately from either language's encoder. The live atomic operation uses this same configuration; standalone codec tests
remain distinct from native rendering acceptance.

The configuration layer passes three Core expect tests and four Rust tests,
including independent bytes, legacy payload preservation, positive/upward-rounded
durations, label/fraction bounds, all easing presets and malformed input. The full
Dune test/formatting run also passes after the wire-module alias change. These
checks validate configuration only. The later integration connects circular
nodes, native value interpolation and the extended bridge operation.

The full Rust protocol suite and strict protocol all-target Clippy also pass.
The current shared mask is `9223372036854775807` after wiring the native operation.

## Retained clock implementation

The standalone native `progress_clock` owner/driver is implemented locally. It
keeps the accepted configuration separate from a bounded displayed-value tween
or indeterminate elapsed duration. It reuses protocol easing, handles interruption
and policy retargeting, preserves cosmetic updates, snaps static/hidden measured
progress and pauses omitted indeterminate intervals. Visible layout time counts;
a layout that never paints cannot replay its interval on reveal. The final queued
wake still paints the exact target rather than silently finishing offscreen.

Rendered elements and the one pending frame callback borrow the owner weakly.
Eight checks pass: seven deterministic timing/configuration tests and a TestPlatform
frame-queue test with 200 label replacements and owner retirement. They cover
initial targets, exact completion, invalid updates, stale drivers, large monotonic
time and finite overshooting easing. No OS window is created by these checks.
The full feature-enabled native suite passes 458 tests with two existing ignores.

The 768-byte owner/map/wake admission allowance excludes the separately shared
configuration and is not an RSS measurement. Tree accounting now reserves it
for each progress node. The View adapter pairs
frame prepare/finish calls, shares the accepted configuration and explicitly
releases owners on window close, even when the View entity is retained. The
clock tests alone do not establish integrated rendering acceptance.

Strict native/protocol all-target Clippy and Rust formatting also pass for the
standalone clock. These checks do not qualify its future rendering integration.


## Standalone painter implementation

The native `progress_paint` canvas painter and private `progress_geometry` module
now implement circular tracks/arcs and rounded linear fills. The circular ring
starts at three o'clock, grows clockwise and remains inscribed in rectangular
bounds. The full track uses 20% foreground alpha; GPUI applies inherited element
opacity to both paths. The changing-length circular indeterminate arc uses the
existing CSS ease-in-out curve over a one-second cycle, with its trailing edge
starting halfway through the cycle. This is an explicit GPUIO easing choice,
not a byte-for-byte reproduction of the source's quadratic easing helper.

Linear fill geometry intersects its rounded moving segment with the rounded
track. This is necessary because GPUI's content mask is rectangular; painting a
rounded background alone does not clip a foreground fill to those corners.
Per-corner radii follow GPUI's half-shortest-side clamp. Sampling scales with
physical size up to fixed caps (16,384 linear outline points; 4,096 segments per
ring arc), keeping work independent of elapsed time. The chord-error target is
0.125 physical pixels before those caps; extremely large geometry has bounded
work rather than a universal subpixel error guarantee. Invalid/empty bounds and
nonfinite coordinates cannot be submitted as paths.

A visible zero-width fill still marks its canvas as painted and requests the
next frame if a transition is active. Otherwise a zero-to-positive transition
would never start. Clipped, transparent and omitted content stops requesting
frames; inert/reduced presentation uses static artwork. The painter performs no
asset loading or callbacks into OCaml.

Three geometry tests inspect actual tessellated triangles for ring area, empty
center, clockwise orientation and containment inside both rounded track/fill,
including tiny/asymmetric fills. They also check invalid inputs and bounded path
sizes. Two TestPlatform paint/queue tests cover zero-to-full completion and idle,
circular track/arc submission, clipping, transparency, hidden/inert state and
owner retirement. The full native library suite passes 463 tests with two
existing ignores. These are headless checks; **GPU pixels, accessibility and
public View behavior are not established by this standalone painter**. Integrated native GPU/AX and public gallery/installed-consumer acceptance remain
pending; bridge, tree ownership and gallery code are now connected.

Strict native/protocol all-target Clippy, Rust formatting and the structural
catalog audit also pass after the standalone painter changes.


## Public/native integration checkpoint

Core/Bonsai now expose both constructors above. Reconciliation emits atomic Op66
when extended presentation is selected; ordinary legacy calls retain Op20 and
explicitly clear an earlier circle/tween. The native tree validates matching
semantic/presentation configurations and permits children only for a final Circle
shape. A transaction can remove center children and restore linear presentation
in either operation order; malformed updates leave revision and accounting intact.
Capability bit 59 joins the exact current mask `9223372036854775807`.

All progress nodes, including legacy bars, now use the retained painter. Circle
roots center ordinary keyed children above the decorative canvas; foreground,
computed root radii and clipping are evaluated natively. Root accessibility
continues to report the accepted target. Frame completion suspends omitted paint,
and a window-close subscription releases owners even if the View entity survives.

Five Core expect tests cover configuration, independent bytes, transition bounds,
center callback retention and legacy reset. The full Dune test/format run and
public gallery build pass. The native library passes 464 tests with two existing
ignores; a separate admission test covers rejected mutations, legal circle children,
legacy reset and memory cleanup. A production-View TestPlatform test covers actual
center-control layout and focus across updates, semantic target versus animated
fraction, and close cleanup with retained stale drivers. It exposed and fixed the
generic splice gate initially rejecting Progress children. These are headless
checks, not macOS accessibility, OS keyboard input or GPU-pixel acceptance.

The Feedback gallery's editable ring center, value buttons, tween toggle and inert
policy use only public APIs. Actual gallery walkthrough, independently installed
consumer runtime, circle GPU/AX matrices, multi-window/resource and broader release
acceptance remain required. Linux graphical qualification remains deferred OCH-47.

The full default Rust workspace and strict native/protocol all-target Clippy also
pass. Existing aggregate-handshake fixtures were updated for bit 59, including
independent encoded bytes. Rust formatting and the structural catalog audit pass.
A fresh independently installed gallery consumer builds successfully with the
new public API (`run=False`); it includes the final native splice correction.
This is local macOS build evidence, not a clean-machine or runtime qualification.


## Gallery driver and frame-delivery regression

`scripts/test_gallery.py --section progress` now selects a dedicated public
walkthrough (also included in `all`). It is authored for native percentage/unknown
AX values, center editor keyboard input and identity, both themes, full/empty/tiny
ring pixels, indeterminate motion, inert AX removal/restoration, transition toggling
and page retirement/remount. The image sampler excludes the editable center so
caret blinking cannot satisfy the animation assertion. Independent synthetic 1×/2×
raster checks verify that exclusion, actual ring changes, translated windows and
out-of-window rejection. Python compilation and CLI discovery pass. **The desktop
walkthrough has not run**; these are syntax and sampling checks, not native acceptance.

A production-View two-window TestPlatform check exposed a retained-clock defect:
a pending frame callback delivered after update preparation but before paint
cleared the flag that preserves visible layout time. The surviving window then
painted 25% instead of the expected 50% midway through its transition. Delivery
now preserves the prepared interval until actual paint resumes it or the frame's
finish discards it. While paint eligibility is unresolved, that one pending wake
may request paint; after omitted paint finishes, the owner remains idle. An older
unit assertion incorrectly assumed preparation alone proved paint was omitted;
it now checks silence after the finish boundary instead.

The new deterministic interval test covers painted and omitted branches plus
reduced motion. The production check covers independent 1s/2s windows, closing
one, circle-to-linear replacement preserving the other tween, reduced-motion
completion, drained frame requests and zero session retention after both close.
The full native library passes **466 tests with two existing ignores**. These are
headless windows; no desktop window was created. Earlier workspace/Dune/installed
consumer build evidence above precedes this private native timing correction.

Strict native/protocol all-target Clippy and Rust formatting pass after this
clock correction, as do Python compilation and the structural catalog audit.

## Physical rendering fixture — execution pending

`rust/native/src/progress_native_test.rs`, wired into `native_images`, now
contains GPU readback assertions for circular fill/track quadrants, an inscribed
ring with an empty center, keyed center content, inherited opacity, deterministic
tween samples, short rounded linear fills and ancestor clipping. It also checks
static/inert/reduced-motion idle behavior, owner retirement and session cleanup.
On macOS it checks progress and center-button accessibility roles and inert
subtree removal. This background fixture does not test OS keyboard input or IME.

On the local macOS checkout, the fixture links with
`GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native --features native-image-tests --test native_images --no-run`.
Strict native/protocol all-target Clippy with native-image/native-canvas test
features and `cargo fmt --all -- --check` also pass. **The new fixture has not
executed**: its pixel, accessibility and idle assertions remain unverified.
The earlier black-window run timed out; neither its cause nor a rendering fix
has been established. Linking this fixture does not resolve that failure.
