# Measured carousel track — implementation design

OCH-41, 2026-10-02. **In progress. Measured presentation, motion and pointer/trackpad/wheel input exist; control relationships and card set metadata are connected; clipped-card input and focus are connected; full AX qualification remains unfinished.**
The existing `View.carousel` remains the full-page carousel. The current
implementation checkpoint includes native geometry, a GPUI measurement probe,
the OCaml value model, Core/Bonsai view construction, checked bridge admission and
native publication/deadline state and measured retained-card rendering. Ordinary
child controls remain interactive, but the component is incomplete; this is not
component/release acceptance.

## Ownership and public API direction

Use a separate `Carousel_track` value model and `View.carousel_track` constructor.
Reuse carousel item IDs/payloads, but keep track-specific measured navigation out
of the existing page model. A track mounts all bounded items (up to 128) with
stable keys. It cannot offer the page API's selected-only `Unmount` policy while
also measuring and displaying neighboring items. Application removal still retires
the component; Bonsai/Eio work belongs to the application as usual.

The application owns selection and the latest *observed* navigation layout.
Rust owns authoritative measurements, scrolling, gesture capture, native clocks
and paint. Selection is never independently mutated in Rust. Ordinary viewport,
track and per-item styles provide variable dimensions and gaps. Both axes share
one geometry algorithm. Visible neighbors remain interactive; the navigation
stack's selected-only focus policy cannot be reused for this component.

The model stores an optional bounded layout observation alongside its
single logical selection. Its normal request reducer handles layout updates and
manual Previous/Next/First/Last/Select actions. This lets ordinary OCaml-built
buttons and native keyboard input use the same measured stops while preserving
ordered relative actions. Explicit selection can still address a logical item
whose snap position is shared with another item.

Layout observations carry canonical stop indices, effective loop presentation,
collection/axis lineage and a monotonically increasing geometry epoch. They
publish on availability, lineage, retained-owner or measured-geometry changes,
including equal stop maps after resize or collection/axis changes. Resizing must
retire an automatic proposal even if the logical successor is unchanged; animation
offsets are not new geometry and do not produce observations.
Native publication must precede input intents
from the frame that uses those measurements. The ordinary generation-checked
node/handler fence applies; the model additionally rejects stale epochs, changed
collection lineages and invalid index maps. Changing item order or axis clears
the observed layout; a same-order payload refresh can keep it until native layout
reports a change. Geometry-dependent controls wait for an initial measurement.

Automatic proposals must include the accepted model revision, source/target and
geometry epoch. A layout change retires stale pending proposals and restarts a
full interval after settled eligible paint. Reuse the existing clock's bounded
one-pending-proposal behavior; do not add polling or synchronous OCaml callbacks.
The value reducer, paired outer protocol, tree admission, reconciliation and native
publication/deadline state now have direct tests. Clipped-card focus and public
Bonsai driver tests are connected; physical AX qualification remains required for
component acceptance.

## Value model and planned view

The value model uses `module Id = Carousel.Id` and
`module Item = Carousel.Item`, plus these principal contracts:

```ocaml
module Layout : sig
  type t [@@deriving equal, sexp_of]
end
module Proposal : sig
  type t [@@deriving equal, sexp_of]
end
module Request : sig
  type t = private
    | Previous
    | Next
    | First
    | Last
    | Select of Id.t
    | Layout of Layout.t
    | Auto_next of Proposal.t
  [@@deriving equal, sexp_of]
end
type 'a t
val apply_request : 'a t -> Request.t -> 'a t Or_error.t
val has_layout : _ t -> bool
val can_previous : _ t -> bool
val can_next : _ t -> bool
```

Checked create/update operations manage the immutable items, selection, axis,
disabled/loop policy and auto-advance. Relative requests reduce against the
latest measured stop map. Explicit selection remains available before layout.
`Carousel_track.Motion` supplies checked duration/easing and an immediate option.
The public view defaults to 200 ms ease-out movement.
The staged Core/Bonsai view takes viewport/track/control styles and a per-item style
function. It retains keyed Panel items within one Container track; the native
CarouselTrack root is its viewport, with ordinary Previous/Next buttons outside
that root inside an explicit CarouselTrackGroup. The native renderer measures and samples movement before child prepaint;
viewport keyboard and automatic deadlines are connected; background dragging and trackpad/wheel input are implemented. Proposal and layout values keep
window/node/handler source identity private; the live bridge generation fence
runs before converting them. A new source may restart its epoch after remount,
whereas nonincreasing epochs from the same source cannot replace newer geometry.
Session and reconciler tests now cover stale sources, failed updates, disabled
observations and remounts; actual host TestPlatform tests cover measured placement
and motion. The public driver checkpoint below adds application integration;
physical qualification remains outstanding.

## Geometry foundation

`carousel_track_geometry` consumes unscrolled, viewport-relative item intervals
and GPUI's measured scroll limit. A loop runway is normalized out of the input.
It checks finite values, positive viewport size, bounded item count and ordered,
nonoverlapping intervals. Zero-size items are allowed. Not-ready or invalid layout
must disable geometric interaction gracefully rather than panic or invent sizes.

The scroll limit includes measured padding, with the content span as a lower
bound. Start-aligned snaps clamp at the finite end. For three 50-pixel items in a
100-pixel viewport, the stops are `[0; -50; -50]`: relative navigation skips the
duplicate, nearest-stop ties choose the first item, and explicit selection can
still choose the third item. `canonical_indices` expresses this without sending
pixel measurements to OCaml.

The pinned source's loop cycle is the item span plus the gap; leading runway
origin includes the spacer's gap. Continuous wrapping translates a retained item
to a neighboring cycle and silently rebases after settling. It must not create
duplicate editors, input owners or callback registrations.

There is a geometric constraint beyond merely fitting one cycle in the viewport:
if the viewport crosses both ends of a wide item, a seamless periodic view needs
two visible fragments of that same item. The source's nearest-copy placement
alone cannot provide both fragments. For example, 150/50-pixel items in a
180-pixel viewport require this during an intermediate wrap offset. This finding
comes from coordinate analysis; it is not a claim of a physical upstream test.

The foundation distinguishes continuous loops from boundary jumps. Continuous
placement requires `cycle - largest_item >= viewport`; a periodic-coverage test
checks unequal widths, gaps and intermediate offsets against repeated ideal
intervals. Otherwise selection can still wrap, but the adapter must settle a
boundary jump immediately on the finite track. Interior moves can still animate.
When the entire cycle is too short to fill the viewport, use the pinned behavior's
finite-track fallback. This explicit motion policy preserves single ownership;
it must be documented on the public API and demonstrated in the gallery.

## Evidence and remaining integration

The native measurement probe uses actual GPUI `ScrollHandle` bounds on both axes,
with unequal item sizes, gaps, padding, changed scroll offset and resize. It checks
that geometry is independent of the scroll offset and focus handles survive a
new layout. The probe bypasses GPUIO protocol and focus/transport policy; it does
not prove those integration requirements.

Current host evidence covers trackpad/wheel, related controls and timer cleanup;
fresh installed consumers build. The visibility and public gallery/driver
checkpoints below add clipping/focus and application integration. Full AX and
physical gallery qualification and broader resource acceptance remain open. Physical
macOS qualification and the broader milestone release gates remain separate.

## Bridge and native state checkpoint

The unpublished exact-epoch-3 protocol appends Kind54 `CarouselTrack`, Op96
`Set_carousel_track`, and Event72 `Carousel_track_requested`. No existing tag moves;
the navigation capability continues to cover this family. The config payload
remains the model/lineage pair. Presentation motion uses separate Op97 `Set_carousel_track_motion`; it does not
change model revision or collection lineage.
The final tree requires one Container track and one retained Panel per model item,
a live root handler and bounded label. Track-only splices revalidate their owner;
invalid transactions preserve the previous tree, revision and retained-byte budget.

Layout events remain legal while disabled. Manual requests and automatic proposals
require enabled policy; automatic admission additionally checks revision, current
ID and target membership. Exact measured successor/epoch checks are owned by native
state and the application reducer. Session and reconciler reject retired envelope
identities before converting a request to a private-source OCaml value. The mailbox
preserves layout/input ordering, accounts for bounded stop-map bytes, and rejects
late input after shutdown or overload. It does not coalesce across input boundaries.

`carousel_track_state::State` is a pure, bounded owner of measurement publication
and the existing native deadline scheduler. A source replacement must construct a
new State. A changed retained item owner invalidates its measurement immediately.
The presenter queues `pending_layout`, acknowledges only successful publication,
then permits interaction using that geometry. Equal repeated measurements neither
flood events nor postpone timers. Geometry changes retire old tickets/proposals;
rearming waits a full interval after published, eligible settled paint. Layout-only
updates do not require a model-revision echo. The measured presenter now publishes through the checked queue and disposes state
on unmount. It must still integrate cancellable clock tasks, full visibility/focus
eligibility, gestures and animated teardown.


The [renderer checkpoint](../evidence/carousel-track-renderer-och41.md) proves native
same-frame selection/measurement, retained neighbor pointer input, padding, resize,
loop-mode placement and empty teardown on GPUI TestPlatform. The structural track
container has only retained items (no implicit text or diagnostic layout child).
Actual physical platform acceptance, animated transitions, complete keyboard/AX
and a public application demonstration remain separate requirements.


## Native track motion

`View.carousel_track ?motion` uses `Carousel_track.Motion.default` (200 ms,
`Animation.Easing.ease_out`), `immediate`, or checked `create ?easing duration`.
Durations are positive, at most ten seconds, rounded up to whole milliseconds.
Both bridge decoders/admission and the public constructor validate the envelope.
Changing presentation alone preserves selection revision, keyed children and
native input owners. Raw bridge absence means immediate presentation.

This is a duration/easing functional equivalent for track movement. The pinned
GPUI Kit styled carousel uses its theme spring; GPUIO does **not** claim that same
spring trajectory or expose track spring tuning at this checkpoint. Progress is
clipped to [0,1] even for overshooting custom curves, with a finite fallback for
numerically extreme control points.

Rust retains one bounded geometry/ID snapshot and the last accepted painted
sample. Retargeting starts from that painted position, never speculative layout.
Resize preserves the old selected ID's visible coordinate where possible, then
moves to the new target. Finite tracks clamp to their new legal endpoints;
removed anchors and changed axes settle. Continuous loops normalize each sample
into one canonical cycle and use only neighboring translations of the same
retained owners. Directional selections use their requested direction; direct
selections use the nearest equivalent loop position. Jump-mode boundary wrapping
settles immediately because a small track cannot safely clone native owners.

Frame requests follow successful eligible paint only. Inactive/reduced-motion,
unavailable geometry, missing paint, source replacement and removal retire motion
history or settle; showing/reactivating does not replay stale travel. Private
sample identities reject obsolete paint. Motion has no worker, polling task or
per-frame OCaml event; geometry observations still follow actual measurements.
Tests cover pure interruption/resize/loop/numeric cases and both-axis host
paint/AX/hitbox correspondence, child retention and lifecycle behavior. See
[motion evidence](../evidence/carousel-track-motion-och41.md).


## Viewport keyboard and native deadlines

The viewport is a focusable region. Home/End and arrows on its current axis enqueue
ordered manual requests after successful layout publication. Modified keys and
keys from descendant controls remain theirs, including unhandled bubbled keys.
The ordinary external Previous/Next buttons retain their existing Core callbacks.
Rust does not change controlled selection while waiting for application reduction.

The mounted track schedules one cancellable deadline using its existing checked
publication/clock state and GPUI executor time. Eligibility requires settled,
visible paint, active window, full motion policy, no hover or contained focus,
no captured pointer or application drag, and current focus/modal permission.
Focus, pointer movement, activation, admitted updates and completed paint reconcile
the deadline. Every wake rechecks current eligibility, source identity, model
revision and geometry. Pausing discards elapsed time; resuming starts a full
interval. Unrelated frames keep an eligible deadline unchanged.

A successful wake emits one `Auto_next` proposal. The timer stays idle until the
model or actual measured geometry changes; pausing does not repeatedly emit the
same unacknowledged proposal. A resize can invalidate a queued proposal even when
its canonical stop map is unchanged. Source replacement, removal and window close
cancel tasks/subscriptions. Timer closures use weak View ownership; there is no
idle poll or synchronous callback into OCaml. See the
[native navigation evidence](../evidence/carousel-track-navigation-och41.md).


The control integration below follows the pinned styled carousel's
`focus_carousel_on_pointer_activation` behavior (carousel source around line 927)
and its card/control click tests. The explicit group supplies these focus-transfer behaviors for the default
Core-composed controls.
Native child editing/selection retains precedence during background dragging.
Offscreen focus/AX remains component acceptance work.


## Measured background pointer dragging

A primary-button drag starts on unused viewport/card background after native child
handlers. Pending movement follows the last accepted painted offset until locking
to the main axis (8-pixel slop, 1.25 dominance ratio); cross-axis motion cancels the
candidate. Once claimed, native pointer capture preserves movement outside the
viewport. Each preview uses measured pixels, bounded finite endpoints or a
normalized continuous loop cycle, and the existing single owner per retained card.
Preview frames change neither controlled selection nor geometry observations.

Release proposes the nearest measured item ID. A deliberate boundary movement
of at least a quarter viewport (minimum one pixel) uses the pinned first/last wrap
rule, including short tracks whose visual loop must jump. While awaiting the
application's controlled selection, preview returns toward the accepted target;
a new accepted selection retargets from actual paint. Reduced motion still allows
direct manipulation, with immediate settlement.

The viewport's mouse-down listener runs before its own automatic focus handler,
after nested handlers; it does not ignore a child's prevented default. Claimed
gestures suppress subsequent activation and focus the viewport. Escape, lost or
foreign capture, changed geometry/model/presentation/source, hidden/inactive state,
unmount and window close cancel ownership. Cancellation releases only this
track's capture, preserving a foreign owner's capture. Background clicks focus
the viewport; external Core controls use the explicit group relationship described below.

The host tests include a real native editor on TestPlatform: pointer selection,
arrow keys and text entry remain with that editor. The wheel integration below adds separate precise-trackpad and line-wheel evidence.
See [pointer evidence](../evidence/carousel-track-pointer-och41.md).


## Measured trackpad and wheel input

Precise trackpad deltas move the retained track in logical pixels. GPUI's ongoing
scroll filter locks the dominant axis; perpendicular input remains available to
other scroll owners. Ended gestures propose the nearest measured item, including
the same intentional boundary-wrap rule as background dragging. Cancelled gestures
return toward the accepted selection without proposing a new one. A 28 ms quiet
deadline, matching the pinned component, supplies completion when terminal events
are absent. Line-wheel input proposes one measured navigation step per burst.

The first useful delta determines ownership for the whole burst. Vertical tracks
at a finite endpoint let edge-started gestures reach an enclosing scroller;
horizontal tracks retain their horizontal axis. Nested native scrolling children
receive input first and consume it when they move. The background hitbox blocks
unrelated mouse interaction while permitting scroll propagation. Controlled-model
updates retire previews but retain a burst fence until quiet, so a line-wheel
selection echo cannot turn remaining momentum into another navigation request.

One epoch-checked, cancellable task observes the latest quiet deadline; extending
a burst does not allocate another live task. An early wake rearms for the remaining
time. Source replacement, unavailable paint, capture, disablement, geometry change,
Escape, unmount and close retire stale work. Tasks hold weak View/source ownership.
Automatic advancement pauses during owned bursts and previews. No idle polling or
per-frame bridge messages are introduced, and manual pixel movement remains
available with reduced motion enabled.

Finishing a preview retains the measured motion context with an explicit retarget
flag. A new gesture can therefore start from actual paint before another render,
while obsolete prepared samples remain invalid. The visibility and gallery
checkpoints below cover clipped-card focus and public-driver integration. Physical
desktop qualification remains separate acceptance work. See [wheel evidence](../evidence/carousel-track-wheel-och41.md).


## Explicit control group and card semantics

The public constructor wraps its viewport and optional default controls in the
structural `CarouselTrackGroup` (appended Kind55). It has no handler, implicit text
or independent selection state. Its first child is one CarouselTrack; the optional
second child is a plain Container with at most 128 direct ordinary buttons.
Candidate-tree validation checks this shape, including nonstructural text/handler
updates on unchanged owners. User keys and labels do not establish relationships.

Default controls retain their ordinary Core press callbacks. After a valid pointer
activation, native focus moves to the eligible viewport. Keyboard and accessibility
activation preserve control focus. Home/End and axis arrows work from these
controls as well as the viewport; descendant editors and unrelated buttons keep
their keys. Dispatch rechecks the current structural relationship and focus gate.
Removing the group removes the related-control behavior without replacing the
viewport or native buttons.

Hover across the group and focus in its related controls pause automatic advance.
Resuming starts a complete interval, matching existing viewport pause semantics.
Each retained card has Group semantics, its existing label, one-based position and
full set size. The viewport remains a labeled Region. Offscreen focus/reveal,
physical VoiceOver and full component accessibility acceptance remain open.
See [control evidence](../evidence/carousel-track-focus-och41.md).


## Retained-card visibility and focus

Every card remains mounted and measured. Fully clipped cards are excluded from
Tab navigation, native input and the accessibility tree; partially visible cards
keep their eligible controls. Selection/navigation reveals a card before focus
can enter it. Focusing an offscreen child does not introduce an implicit second
selection owner. A visible card can still reveal a descendant in its own ordinary
scroll container without changing the carousel selection.

If an accepted selection, gesture or resize moves a focused descendant entirely
out of view, focus returns to the eligible viewport. A viewport with no visible
extent cannot retain that focus. External controls and independent modal scopes
keep their focus. Disabled viewports reject background and accessibility focus;
ordinary child controls keep their own disabled policies.

The native host computes card visibility from actual translated bounds, including
ancestor clipping. A stable semantic wrapper keyed by the retained card ID hides
its accessibility descendants without replacing native owners or layout. Focus
records for clipped children cannot inherit permission from a visible ancestor.
A visibility transition schedules one further render so anchored surfaces see
the completed measurement; a stable mask schedules no redraw or idle timer.

Application-owned modal portals have independent window-space presentation and
remain active when their mounting card clips. Ordinary anchored popovers suspend
while their card is clipped and reappear on reveal, retaining their native owners
without repeating autofocus. Structural hiding, disabling and removal still obey
the existing overlay lifecycle. This is adapter behavior on GPUI TestPlatform;
physical VoiceOver/IME and full gallery qualification remain required.
See [visibility evidence](../evidence/carousel-track-visibility-och41.md).


## Public gallery and Bonsai integration

The Journeys page's “Ideas in motion” preview uses only public Core/Bonsai/Eio
interfaces. Four unequal cards retain identity and a native text editor through
selection and reversal. Controls demonstrate horizontal/vertical layout, compact
viewport sizing, immediate/default movement, loop fallback, automatic advancement
and disabling navigation. Model requests, including native layout observations,
reduce through one Bonsai state machine; native offsets stay in Rust.

A window-driver expect test dispatches ordered native requests before Bonsai
stabilization, rejects stale automatic/layout events, reverses by stable ID,
retains nodes, and checks disabled input, idle transactions and close fences.
Physical gallery appearance/input and end-to-end AX remain separate acceptance.
See [gallery evidence and walkthrough](../evidence/carousel-track-gallery-och41.md).
