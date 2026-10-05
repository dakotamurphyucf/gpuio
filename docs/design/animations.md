# Native declarative animations (OCH-12)

Status: typed configuration, view/reconciler/transport and GPUI rendering now pass
local macOS tests, including the public Bonsai/Eio example. Shared application motion
policy and live platform preference adapters are implemented. `CAP_ANIMATIONS`
(33554432) is advertised in aggregate mask 67108863. The
[milestone ledger](../evidence/milestones-01-02.md) records hosted acceptance and
links the final PR checks/merge status.

## Configuration

`Animation.Target.create` validates a nonempty property set: width, height, top,
right, bottom, left, opacity, multiplicative opacity and four corner radii. `Radius` is an OCaml shorthand
for all four corners. Duplicate properties after expansion are errors. Geometry
uses logical pixels, with the same finite bounds as the style API; opacity is in
[0,1]. Absolute opacity and [multiplicative opacity](animation-opacity-factor.md)
cannot occur in the same target; the latter preserves base/interaction styling.
Numeric targets deliberately exclude `auto` and percentage interpolation.

`Animation.Config.create` accepts a target, optional initial values, duration,
delay, easing and repetition. Initial and target property sets must match.
Duration defaults to 200 ms, delay to zero, easing to linear and repetition to once.

[Explicit iteration policies](animation-iterations.md) add full unsigned finite
counts and Normal/Reverse/Alternate/Alternate_reverse playback through
`Repeat.Finite` and `Repeat.Infinite`. The legacy policies retain their behavior.

Initial delay is signed, within ±one day, rounded away from zero to whole
milliseconds. Positive values hold the starting value; negative values start that
many milliseconds into the run, possibly beyond its endpoint or across repeat
cycles. The native owner adds this offset to active elapsed time rather than
backdating the clock origin. Hidden time remains excluded. With no initial values,
first placement remains immediate; reduced motion retains its existing policy.
On retargeting, a negative delay advances from the last painted value and can
therefore cause a deliberate jump. Completion still requires an accepted paint.
Core `Time_ns.Span` arguments are bounded to one day and rounded up to milliseconds.
The public type is abstract and does not expose the bridge's generation counter.

```ocaml
let target =
  Animation.Target.create [ Width, 240.; Opacity, 1. ] |> Or_error.ok_exn
in
let initial =
  Animation.Target.create [ Width, 0.; Opacity, 0. ] |> Or_error.ok_exn
in
Animation.Config.create
  ~initial
  ~target
  ~duration:(Time_ns.Span.of_ms 180.)
  ~easing:Animation.Easing.ease_out
  ()
```

Easing includes linear, ease, ease-in, ease-out, ease-in-out and validated cubic
Bezier control points. Bezier evaluation inverts the X curve, including endpoint
zero derivatives. X controls are in [0,1]; Y controls may be any finite value. Overshoot is
allowed; resulting properties clamp to their valid domain, such as nonnegative
width and opacity no greater than one. Timing is evaluated once per sample,
then applied to at most eleven properties.

The polynomial presets `ease_in_cubic`, `ease_out_cubic` and
`ease_in_out_cubic` are distinct from the CSS presets. The first two use Bezier
control points with linear x. The third evaluates `4*t^3` for `t <= 0.5`, then
`1 - 4*(1-t)^3`, entirely in Rust. All three clamp progress to the endpoints;
they share existing timing, interruption, reduced-motion and ownership rules.
`Easing.steps` adds the four [step-position policies](stepped-easing.md),
including immediate zero-progress jumps for Jump_start and Jump_both.

## Timing and ownership

`rust/native/src/motion.rs` owns a run's configuration, origin, last painted values,
pause state and terminal status. It has no scheduler, GPUI entity or OCaml callback.
The host passes a monotonic clock and interprets the returned scheduling request:

- `Idle`: no animation wake requested.
- `At deadline`: a delayed start needs one deadline wake, without frame polling.
- `Frame`: intermediate values need another native animation frame.

Layout may preview a sample. Only committing that sample at paint updates the
visible value and reports completion. Generation and opaque epoch identity reject
samples prepared before a replacement, visibility change or reduced-motion change.
Older timestamps cannot overwrite newer painted values. Epoch identity allocates
on state changes, not on every sample.

With no initial values, first placement is immediate. A changed target starts at
the last painted value, even if time has advanced further or the old run was in
delay. Newly targeted properties start at the supplied initial value or their target.
An identical snapshot does not restart. A changed snapshot requires a newer bridge
generation. Interruption cancels an unfinished old run once; a completed run does
not later emit cancellation. Zero duration respects delay unless placement is
already immediate or reduced motion applies.

Loop and alternate repetition require explicit initial values and positive duration.
After interruption, the first cycle goes from the painted value to the new target;
subsequent cycles use the declared initial/target range. Loop resets at its cycle
boundary; alternate reverses continuously. Integer duration arithmetic avoids
losing sub-frame precision after long uptime. Repeats have no per-cycle event.

Explicitly hidden runs pause elapsed time and request no animation wake. Showing
one resumes it. Reduced motion immediately settles a visible one-shot run at its
target, with completion at paint. A repeated run displays its initial values and
pauses without an endpoint or frame requests; full motion resumes its elapsed phase.
A finite run completed under reduced motion does not replay when the policy changes.
Cancellation freezes the last painted values and emits at most one endpoint.
Unmount cancels native state and pending deadlines; window disposal releases its
owned state and timers. Application callbacks are discarded on unmount or close.

## View integration

`View.animate ?key ?style ?on_event config children` and its Bonsai specialization
retain native state by node identity. Animation-owned fields take precedence over
base and state styles. Filtering is cached at configuration/style updates; numeric
samples are applied without a per-frame temporary vector allocation. The animated
wrapper remains a flex column and can clip a fixed-width inner sidebar.

The adapter keeps one cancellable deadline task for a delayed run. Its future owns
a weak View reference; painted elements own weak animation state references. Native
frame requests are coalesced by GPUI, with no animation callback into Bonsai. A
prepared sample rejected by the timing core cannot request another frame.

`on_event` receives a typed run ID and outcome using the latest callback closure.
Run numbers are local to the retained node. A replacement can report cancellation
of its previous run after the new generation is accepted. The bridge keeps FIFO
endpoint order; the reconciler checks node/handler/revision, bounds the endpoint
by the current generation, and keeps a shared delivery watermark to reject duplicate
or older endpoints. Removed widgets cannot deliver callbacks to replacements.
Adding an observer does not replay earlier completed runs.

The protocol adds Kind 24, Set_animation operation 27 and Animation_endpoint event
26. The Rust decoder caps property lists before allocation and validates geometry,
easing, initial sets and timing. Tree validation rejects incompatible configuration,
backward/reused changed generations and invalid updates atomically; retained
configuration buffers count toward the tree budget.

## Application motion preferences

`App.run ~motion:Animation.Preference.System` is the default.
`App.set_motion app Reduce` or `Full` overrides the system for all windows, including
ones opened later. Selecting `System` again uses the latest observed system value.
The runtime coalesces pending preferences and sends the initial preference before
opening windows. The wire command is `Set_motion` (message tag 9), with preference
tags System=0, Reduce=1, Full=2. Native state refreshes windows only when the resolved
Boolean changes; platform updates do not generate OCaml effects.

macOS reads `NSWorkspace.accessibilityDisplayShouldReduceMotion` on the GPUI main
thread. Its workspace display-options notification signals a one-slot channel;
the consumer reads the latest setting on the main thread. Subscription precedes
the initial read, and disposal unregisters the exact observer token before dropping
the consumer. See Apple's [preference](https://developer.apple.com/documentation/appkit/nsworkspace/accessibilitydisplayshouldreducemotion)
and [notification](https://developer.apple.com/documentation/appkit/nsworkspace/accessibilitydisplayoptionsdidchangenotification).

Linux observes the standard XDG Settings `org.freedesktop.appearance/reduced-motion`
key: unsigned 1 means reduced; 0 or other unsigned values mean no preference.
It subscribes to setting and portal name-owner changes before `ReadOne`. Signals
trigger a current read, so queued old payloads cannot revert a newer snapshot.
The private connection, subscriptions, request timeouts and update queue are bounded;
requests run on the background executor. See the [XDG Settings specification](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.Settings.html).

`System` falls back to full motion while the asynchronous Linux read is pending or
when no valid setting is available. Older desktops without this key use that
fallback and can still use an explicit application override. Portal restarts on an
existing connection are observed; a missing/disconnected session bus is not polled
or reconnected until a new application launch. Shutdown drops the observer and its
consumer. No extra permanent polling timer is introduced.

Both declarative transitions and native progress use the same resolved GPUI flag.
Reduced indeterminate progress displays a centered, static 25%-width bar with no
fabricated numeric accessibility value. Full motion resumes its native cycle.

OCH-12 completion requires the final-head macOS functionality and Linux build/test
gates and merge. Linux GUI checks remain informational for OCH-17.

Springs, sequences and synchronized repetition belong to OCH-25; its
[implementation design](animation-programs.md) preserves this baseline. This does not
remove any OCH-12 baseline requirement or OCH-11's shared basic-transition scope.

## Local evidence

Nine deterministic Rust motion tests pass: delay/deadlines, paint-confirmed
endpoints, interruption/reversal, immediate placement, zero duration, invalid
updates, hidden/reduced policy, repeats, long uptime, Bezier inversion/overshoot
and invalidation of previously prepared frames. Extreme finite Bezier Y controls
also produce bounded geometry. Full Rust workspace tests pass.
OCaml expect tests validate property expansion/canonical identity, geometry,
initial range, timing, easing and bridge generation. An independent binary fixture
checks the Rust and OCaml configuration encoders and full OCaml decoding.

The production-window `native_animation` test additionally verifies zero-width
initial placement, midpoint geometry, fixed-width inner content, interruption and
once-only native endpoints with a controlled clock. A real-clock repeat schedules
its own frames without tree revisions. Whole-window render counts stay unchanged
after completion, while an ancestor is hidden and under reduced motion. A finite
run settles immediately under reduced motion; removal releases a pending deadline
and weak state owner. This validates GPUI frames rather than physical keyboard/IME input. The test now
activates its window because a completely occluded background window can wait
indefinitely for its first frame. Local foreground tests are explicitly authorized.

The public `examples/animation --self-test` passes Bonsai/Eio target updates,
endpoint decoding/delivery, theme change and shutdown. A slow first frame may finish
the initial run before interruption, so that smoke accepts either initial outcome;
the deterministic native test proves interruption itself. Reconciler expect tests
cover current callbacks, stable identities, ordered prior-run cancellation,
duplicate/future generation rejection and unmount. Native validation and decoder
tests cover rollback, list limits and invalid values.

Local logs are `motion-*.log` and `animation-*.log` in the implementing agent's ignored
scratch directory. The checked-in CI adds native/public macOS runs and informational
Linux runs; the milestone ledger records consolidated hosted execution.

Motion policy tests pass the precedence/unknown-value matrix. The actual macOS
animation window passes live application policy and a real workspace-notification
round trip, then verifies observer disposal. The test posts a notification without
changing the user's OS settings. The Linux observer's private socket tests pass
initial and changed values, stale signal payloads, name-owner changes, unavailable
keys and a silent service timeout; these are protocol tests executed on macOS,
not Linux desktop validation. The public example's final 60-second transition
settles within its 15-second test scope after `App.set_motion app Reduce`.

The focused progress suite and full native controls suite pass with the shared
policy changes: reduced indeterminate progress remains visibly centered, leaves
the whole window idle, and resumes native frames under Full. Resumed feature-enabled
all-target Clippy passes. An earlier full-controls run failed an existing tooltip
hover check during severe host memory pressure; the resumed full suite passes.

Additional native acceptance covers zero-width/zero-height/zero-opacity first
placement, width/height plus top/left interpolation, right/bottom anchoring and
closing the window with a 60-second delayed start pending. The weak native owner
is released and no late endpoint is delivered. Startup markers help distinguish
launch problems from a frame wait. The background stall was diagnosed by activating
the exact running process, after which every assertion completed; the test now
activates itself, as the existing control tests do.

## Piecewise-linear easing

`Animation.Easing.Linear_stop.create ?input ~output ()` validates a finite output
and an optional input in `[0,1]`. `Easing.linear_stops` accepts 2–256 stops. Omitted
first/last positions become 0/1; omitted interior positions are evenly distributed
between their surrounding explicit anchors. Explicit positions must be
nondecreasing. Equal positions encode a jump: the last stop at that position wins.
Before the first position and after the last, the curve holds the endpoint output.
These are the pinned GPUI Kit `LinearStops` semantics, with an explicit bounded
transport and double precision.

Outputs need not start at zero, finish at one or stay within that range. Native
property clamps still apply. Positive initial delay holds the declared initial value;
at the start, the curve's zero-progress value applies. Finite completion settles
at the declared target even when the last curve output differs from one. Reduced
motion, retargeting and paint-confirmed completion retain their existing rules.

The paired unpublished epoch-3 representation appends easing tag 8 followed by
a bin-prot list of resolved `(input, output)` double pairs. Rust bounds the count
before allocating and independently validates positions and outputs. It owns an
immutable shared array, uses binary search and weighted interpolation without
per-frame allocation or an OCaml callback, and accounts for retained curve data
in admission reservations. Weighted interpolation avoids overflowing a subtraction
between finite opposite-sign extreme outputs.

Variable-length curves count toward the existing 16,384-byte program limit.
Both public program construction and native validation reserve the largest
generation/restart encodings, so later playback/restart changes remain encodable.
Progress and spinner presentation envelopes are now 16,384 bytes to accommodate
their existing maximum 4,096-byte labels together with a 256-stop curve; individual
label and curve limits are unchanged. Carousel, progress and spinner motion share the
same easing contract. Native consumers must rebuild with the paired OCaml schema.
