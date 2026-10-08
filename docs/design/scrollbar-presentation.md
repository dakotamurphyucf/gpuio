# Native scrollbar presentation — OCH-41

Status: implementation contract; Core/Bonsai View metadata, paired transport,
transactional admission and ordinary-container/managed-list/tree/table native
integration are implemented. The shared widget, geometry, state styling and native clocks
have separate foundation evidence. Public gallery controls and an installed-consumer build now pass; physical
macOS qualification remains unfinished. See the
[table evidence](../evidence/scrollbar-table-och41.md) and earlier
[bridge evidence](../evidence/scrollbar-bridge-och41.md).
The [pinned collections review](../catalog/collections-review.md) records the
source surface and the existing Boolean-only list/table presentation.

## Public vocabulary

`Scrollbar` is a checked description of the bars attached to an existing native
viewport. `Axis.Horizontal/Vertical/Both` filters which bars may appear; it does
not turn on scrolling, manufacture overflow or change the data owner's axis.
`Mode.Scrolling/Hover/Always` controls visibility. A bar is absent when its axis
has no overflow, even in Always mode. `label` names the scroll area for native
accessibility and is bounded nonblank UTF-8 without NUL (1–1024 bytes).

`Track` supplies optional solid background, border color and cross-axis width.
`Thumb` supplies optional `Background` (solid or the existing two-stop sRGB/Oklab
gradient), cross-axis width, inset, radius and minimum along-axis length.
All dimensions are finite logical pixels in 0–16384. They are requested
presentation values: native layout clips them to the actual viewport. Zero
dimensions are valid; a zero-area bar has no paint or pointer target.

`Appearance` has separate base, hover and pressed Track/Thumb refinements.
An absent state field inherits the base field, then the native themed default;
pressed does not accidentally inherit a hover-only override. Pointer over the
track may style the track; pointer over the thumb additionally styles the thumb.
These are scalar refinements, not arbitrary layouts or synchronous callbacks.
Theme tokens resolve through the mounted theme before native admission, including
all nonvisible states. Missing tokens fail the whole description.

`Motion` contains idle hold, enter, exit and width-expansion durations, plus Fade
or Slide_and_fade for normal entrance and thumb-hover entrance. Durations are
0–60 seconds, rounded up to whole milliseconds. The default keeps upstream
Base's two-second idle hold and immediate transitions. Motion remains native.
Mode/style/motion changes preserve the existing native scroll handle and offset.

The intended View adapter applies optional metadata to a scrollable ordinary
container or managed list/table. Clearing metadata restores that owner's legacy
presentation, not its initial scroll position. List/table Boolean visibility
continues to suppress their bars; custom appearance must not silently override
an explicitly disabled bar. Ordinary overflow containers gain visible bars only
when opted in. Tree/list/table presenters must expose the same value vocabulary.
Do not claim complete viewport coverage until the table renderer and public
gallery/consumer acceptance join the existing container/list implementation.

The attaching API is receiver-first `View.with_scrollbar view config`, where
`config` is `Scrollbar.t option` and the result is `Or_error`. It accepts ordinary
Container and managed Virtual_list roots (including tree/table owners), rejecting
native editors and other specialized widgets. A container without current scroll
overflow retains dormant metadata; presentation does not manufacture a handle or
force scrolling. `None` clears the override. Reconciliation compares the resolved
description so theme changes update paint while retaining native node identity.
The attaching operation appends protocol tag 108, without changing earlier tags
or record layouts. Admission reserves bounded native state as well as the label
and description; invalid configuration and quota overflow reject the transaction.

## Native ownership and geometry

One native viewport owns offsets, clamping, wheel routing and content measurement.
Scrollbar input reads/writes that same handle: no extra scroll container, copied
offset or per-frame OCaml notification. A list drag calls its native start/end
hooks so tail following and row anchoring retain their established behavior.
Horizontal table bars use the unpinned viewport and must not scroll pinned cells.

Measure both axes together. Reserve a corner only when both bars have usable
overflow. With two eligible bars, clamp each edge envelope to half its cross
extent so corner reservation cannot erase both tracks in a tiny viewport.
Clamp track width to the available cross extent, inset to half the
usable track, and minimum thumb length to the remaining extent. Keep finite,
nonnegative bounds and defined zero-travel behavior. Do not divide by zero when
content disappears or clamp with an inverted range. Track/thumb paint and input
use the same clipped geometry, including borders/padding and parent clipping.
Native drag keeps a captured grip offset; content/viewport changes recompute
travel from current measurements rather than replaying obsolete bounds.

Use a stable interaction envelope across hover/pressed width refinements to
avoid hover oscillation when a painted track narrows. Thumb hover uses the union
of the four resolved state rectangles at the current native offset. A narrowed
hover style therefore cannot repeatedly leave its own activation region. This
union only selects hover styling: capture and track clicks still require the
actual last-painted translated thumb/track inside the current clip. A visually hidden bar must
not capture a thumb drag; Hover mode may use an eligible edge region to reveal
it. Pointer input, native accessibility actions and any keyboard path must use
the same live owner/disabled/hidden/modal checks. Existing editor/composition
focus takes precedence. The adapter must specify and test native range semantics
and keyboard navigation before feature acceptance; paint-only is insufficient.

No temporary global-theme mutation. The pinned `gpui_base::Scrollbar` stores
styles/mode per instance but reads motion from `Theme.scrollbar`, and its internal
state/deadlines use wall-clock helpers. Evaluate a scoped extraction or a narrow
reconstructible adaptation with explicit local policy; do not claim the existing
widget alone satisfies owner retirement, deterministic clocks or custom motion.
The upstream drag FPS knob maps to native event/frame coalescing, not an OCaml
timer or a requirement to reproduce a second scheduling API.

The pure geometry adapter uses viewport-local logical coordinates and positive
offsets from the start. Convert the existing GPUI handle's negative offsets once
at the boundary. Track clicks center the thumb; captured dragging retains a
pixel grip distance, clamped to the newly measured thumb after resize. With no
thumb area or no remaining travel, pointer projection is a no-op, not a jump to
the beginning. Parent clipping and eligibility remain the renderer's job.

The pure appearance resolver takes the current foreground explicitly. Defaults
follow the pinned Base vocabulary: transparent track/border, 16px track, 6px
resting/8px hovered-or-pressed thumb, 4px inset, zero radius, 48px minimum length,
and foreground with 35%/55% alpha (rounded to packed RGBA). Track hover alone
does not expand the thumb. Explicit gradients, transparent overrides and radii
are honored independently of unrelated global-theme radius settings. The fixed
interaction envelope is the maximum resolved track width across states. When a
configuration update narrows that maximum during width animation, the current
frame's envelope also includes the sampled painted track width. It contracts to
the new maximum as the animation settles. Otherwise a valid transition from a
wider accepted paint would fail geometry validation and spuriously retire focus.
Normal hover changes within an unchanged configuration retain the stable maximum.

## Lifecycle and bridge

Retain state by native node generation and axis. Appearance/theme updates keep
child editors, selection, focus and offset. Axis removal, metadata reset, zero
geometry, hidden/inert/disabled owner, modal exclusion, window deactivation,
unmount and close cancel captured dragging and balance list drag hooks. Old
listeners/actions must not act on a replacement node. Restoring visibility must
not replay old pointer input.

Motion advances only while a visible channel is unsettled. Reduced motion snaps;
ineligible/inactive/hidden owners settle or suspend without an idle frame loop.
Finite owned idle deadlines are rearmed from current activity, canceled on
retirement, and checked against the current owner before waking. A detached task
must not retain an obsolete application tree. Each native scrollbar has bounded
state; quota admission accounts for metadata and its fixed native state budget.

The bridge uses a checked bounded description with no native handles or callbacks.
Existing operations/record layouts remain unchanged; add the attaching operation
only with paired OCaml/Rust transport, transactional validation and quota checks.
Reject unknown tags, nonfinite/oversized dimensions, invalid gradient/color data,
invalid labels, truncated input and trailing data. Source snapshots stay unchanged.

## Verification and delivery

First test the value/codec domain with independent byte fixtures, optional-state
inheritance, theme resolution and boundary/malformed cases. Then test native
geometry against a simple reference across small/large extents, both axes,
overscroll, zero travel and resizing during drag. Exercise actual native paint,
track click, captured dragging, wheel/follow ownership, reduced motion, keyboard/
accessibility, owner eligibility and teardown. Integrate ordinary containers,
managed lists/trees and tables with retained-child regressions.

The Collections gallery must demonstrate both axes, the three visibility modes,
per-part styling, finite motion and changing content while maintaining position.
Run fresh installed-consumer and platform-required checks. TestPlatform evidence
is distinct from physical macOS input/VoiceOver/GPU and release acceptance; full
Linux desktop qualification remains deferred OCH-47.

## Retained timing model

The native timing model uses caller-supplied monotonic elapsed time. It owns no
executor, callback or scroll handle. Scrolling and Hover modes both reveal on
actual scroll activity; Hover additionally reveals from its eligible edge target.
Hovering an already visible Scrolling bar, dragging, or explicitly focusing its
native range keeps it visible. Leaving hover/focus or releasing capture begins a
fresh idle hold. Idle zero means no post-activity hold. Always mode skips visibility
motion but still allows finite width motion; reduced motion snaps every visual
channel while preserving the idle visibility policy.

Only an accepted paint starts or commits a visual transition. Layout previews may
compute a candidate without changing timing or last-painted values. Interruption
starts from the last accepted paint, with reveal/hide duration scaled by remaining
opacity/slide distance; width changes use the configured expansion duration.
Fade uses no positional travel, while Slide_and_fade translates toward the outer
edge. Thumb-hover entrance selects the reveal choreography in Hover mode, not a
separate perpetual animation. It does not restart an already revealed bar.

A scrolling idle deadline uses an opaque activity identity that survives ordinary
paint frames and rejects old activity, configuration, eligibility and owner epochs.
The adapter retains at most one cancellable idle task plus one weak pending frame
wake per axis (at most two of each per viewport). Settled visible or hidden owners request no frames; an idle hold requests a
single deadline rather than repeated polling. Hidden/inert/disabled/modal-excluded,
zero-geometry, axis-removed and inactive owners clear interaction, pending activity
and visual history immediately. Re-enabling them cannot replay an old capture or
scroll event. Unmount/close permanently retires the model; a new generation gets
a new identity. Actual native handle drag hooks remain the renderer's responsibility
and must be balanced whenever eligibility changes.

## Native widget boundary

The standalone widget owns a strong `Rc<dyn gpui_base::ScrollbarHandle>` for the
existing viewport, two native focus handles and two timing owners. Its rendered
elements and event listeners hold only weak widget references. A checked config
update preserves the handle and offsets. Native policy supplies visibility,
enabled/modal eligibility and pointer eligibility; it is checked again when an
event executes. A window-activation subscription immediately cancels capture and
timing. The Host must explicitly close removed owners before dropping them, so
pointer capture and range focus are released while the Window is still available.

The widget's geometry can come from the handle's viewport or an explicit layout
viewport. The latter is needed for the managed table's unpinned horizontal range
and header-excluding vertical range. It never replaces the handle's content size
or invents its own scroll offset. All offset updates clamp against current
measurements and preserve the perpendicular axis. Wheel input continues to use
the existing scroll owner and nested routing.

Thumb pointer capture retains its pixel grip and balances `start_drag/end_drag`
exactly once. Every captured move remeasures current content; resize continues
with current travel rather than applying stale content ratios. Track clicks jump
the thumb center without taking editor focus. Pointer dragging also preserves
existing editor focus. Native range focus is explicit via Tab or accessibility.
Only a focused range consumes unmodified matching-axis arrows, PageUp/PageDown,
Home/End; Escape releases its capture. Arrows use the current native line height;
pages use one viewport extent, and all destinations clamp. Modifier shortcuts
and unrelated keys propagate to their existing owners.

Each eligible axis has an AccessKit ScrollBar role, orientation, a label derived
from the configured area label and axis, and positive numeric range 0..max_offset.
Focus, Increment, Decrement and finite NumericValue SetValue actions all use the
same live geometry/policy gate. Fade-hidden eligible ranges may be focused to
reveal them; geometry-hidden or disabled ranges cannot accept actions. Actions
never synchronously call OCaml. Explicit range focus remains discoverable while
its appearance changes; disabling/hiding/removing it retires its tab stop and
restores or clears focus through the normal Host focus policy.

## Scoped managed-table integration

A native-only optional table presenter renders each axis over its existing handle.
The adapter keeps layout ownership: the vertical viewport excludes headers, while
the horizontal viewport excludes pinned columns. Typed Host owner keys distinguish
these two presentations from ordinary/list viewports. Resetting metadata restores
the Base renderer without replacing TableState, selection, editor children or
scroll handles. The existing Boolean visibility flag still wins.

Each table axis uses a one-axis config and reads the other native handle only to
reserve a corner while that sibling actually overflows. This reservation shortens
the painted track and pointer travel, not the semantic viewport or numeric range.
It caps at half the axis length in tiny viewports. Empty tables omit the vertical
list; its cached handle metrics cannot reserve a corner. All-pinned columns omit
the horizontal range. Renderer callbacks retain weak presentation references;
there are no cross-runtime callbacks or copied offsets.

Pointer dragging keeps the original table/editor focus. GPUI dispatches bound
key actions before element key listeners, so unmodified Escape cancellation uses
the existing window-scoped keystroke interceptor before application shortcuts.
It consumes Escape only when cancelling a live scrollbar drag, preserving table
selection and balancing native capture. Focused range keys remain local to their
range; ordinary table/tree navigation retains its existing exact-focus gate.

List measurement configuration changes are distinct from scrollbar appearance
updates: the existing managed-list adapter may replace its native ListState.
Before that replacement, the Host closes the old scrollbar presentation while its
Window and handle remain available, cancelling pointer capture, retiring range
focus and balancing the old handle's drag hooks. The next render binds the new
handle. The list adapter restores the stable logical anchor; scrollbar metadata
alone continues to retain the existing owner and focus.
