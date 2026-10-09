# Layered notifications and native motion — OCH-41

Status: layering and Motion now connect Core/protocol, measured native rendering,
finite production phases and the Feedback gallery. Local Core/protocol/native,
lint and installed-consumer checks pass; physical macOS and release checks remain open. This implements the work
under the [notification review](../catalog/notification-review.md).
Placement/insets are already implemented separately. Preserve the existing
expanded scrolling column and immediate dismissal when no presentation is enabled.

## Public shape

`Toast.Stack.Layering` and optional `~layering` on `Stack.create` connect to the
measured native renderer. `Toast.Stack.Motion` and optional `~motion` configure
native reflow and slide/fade independently of layering.

`Layering.create ?peek ?gap ?width_step ?visible ()` defaults to 14px collapsed
peek, 14px expanded gap, a 0.05 fractional width reduction per depth and three
visible layers. Peek/gap are finite 0..16384; width step is finite 0..0.1; visible
layers are 1..8. These bounds keep all eight supported layers positive-width.
The existing max-visible/submission limits remain separate (1..8 and 32).
No layering means ordinary expanded layout, preserving oldest-first order and the
8px gap even when Motion is enabled. Layering without motion changes
geometry immediately; adding motion animates reflow and lifecycle.

`Motion.create ?spring ?enter ?exit ?offset ()` uses a checked `Animation.Spring`,
400ms entry, 200ms exit and 96px slide distance. Durations are 0..60 seconds and
round upward to whole milliseconds; offset is finite 0..16384. The default spring
is stiffness400/damping40/mass1/epsilon0.01, bounded to two seconds. Spring updates
retarget current painted geometry/velocity; lifecycle durations are captured when
a phase starts, so ordinary configuration updates cannot restart an entry/exit.
Removing motion completes its current phase immediately.

## Geometry, identity and interaction

OCaml owns toast identity/order/content and removes items after dismissal. Rust
owns measured card sizes, expansion, reflow, entry and exit. All native records
use full node generations, not list positions. Removing a node retires its geometry
and callbacks; no native rendering clone can keep a removed subtree alive.

Newest cards occupy the front edge. Collapsed layout retains the newest visible
layers, reduces their width symmetrically by depth and offsets them by peek.
Expanded layout uses full width, each card's measured height and configured gap.
Hidden deeper layers retain their child owners but contribute no collapsed
footprint. Use actual current-frame measurement; do not infer equal card heights.
Bottom anchors mirror vertical offsets. Center anchors place the resulting stack
inside the checked placement rectangle.

Compose the stack's anchor and each card's offset into one target position before
interpolation. Do not animate stack height and item offset independently: separate
measurement frames can cause the bottom-anchored jitter noted in the pinned source.
Reflow samples are committed only after paint. Interruption starts at the last
painted position and velocity. Layout outside the generic spring coordinate domain
settles exactly rather than panicking or clipping the declared geometry.

For targets discovered during layout, advance the new trajectory from the last
committed paint timestamp to the current frame. Restarting at the current time
and sampling at that same time would stall continuously changing streamed content.
Keep unchanged axis trajectories rather than restarting width when only height
changes. Sample the width target before measuring text; then compose the measured
natural height and anchor into final card-edge targets. Native content keeps its
current measured height. Interpolate the horizontal center and the vertical edge
appropriate to the anchor (top, bottom or center), then derive the current bounds
from that edge, sampled width and measured height. This keeps a growing bottom
card attached to the bottom edge without clipping newly streamed content behind a
lagging height spring. Commit only the final painted sample. Command-time
retargeting remains distinct from this frame-driven measurement rule. No
independent stack-height spring or prior-frame height cache is needed.

Hover over the stack footprint or keyboard focus anywhere inside expands it and
pauses active-time expiry. A named, non-autofocusing stack focus stop lets keyboard
and assistive users expand before traversing older cards; leaving the scope allows
collapse. Motion without layering keeps the expanded column and also provides
the named Group entry for bounded keyboard scrolling. New notifications never steal focus. Only the front card accepts input
when collapsed; deeper painted layers are decorative until expanded. Deeper hidden
content and Ending cards must be excluded from pointer input, Tab, accessible
actions and semantic exposure, not merely made transparent. Existing modal and
disabled policy still wins. Preserve a bounded scroll viewport for an expanded
stack taller than its usable rectangle.

## Lifecycle and event contract

Each mounted toast starts Pending. Its first eligible paint starts entry; when
entry finishes it becomes Present and active-time expiry may run. Reduced motion,
inactive/hidden/zero-area owners settle entry without a recurring clock. Unpainted
owners may not keep requesting frames. Ordinary re-renders, theme changes, movement
and child updates do not replay entry. Enabling motion on an already present toast
does not create a new mount.

A native close/Escape/timeout/overflow accepts at most one dismissal reason. It
stops active-time expiry immediately and starts a noninteractive Ending phase from
the last painted opacity/slide (including interrupted entry). Focus restoration
happens when input eligibility ends, not after a delay. One terminal dismissal
event is delivered after exit completes. With no motion it is immediate, preserving
the old contract. Current owner/handler/revision checks still gate publication;
the observer carries no callback into OCaml from layout or paint.

Validate the reason when accepting dismissal and retain an opaque, single-use
session token through exit. Completion checks the original session, live window/
node/handler generations, revision and overload state. It does not reevaluate a
later timeout setting: changing an already-expiring toast to persistent cannot
undo an accepted terminal decision. The existing Core dispatcher already accepts
terminal observations across ordinary configuration updates. A token cannot be
transferred to another session even if its numeric IDs happen to match.

Ending items remain in layout only while still mounted. Reduced motion, disabling
motion, hidden/inactive/zero-area policy completes an accepted exit immediately
and releases its event once. Application removal, window close or overload discards
pending motion/events and releases owners; it does not synthesize another dismissal.
Once closed, metadata updates cannot reopen the native lifetime. Use a new key to
start a new notification. Overflow still obeys the 32-item hard bound, including
ending cards; do not create an unbounded retirement list.

Reserve native presentation bookkeeping explicitly at admission (stack plus a
bounded per-submitted-card allowance), in addition to existing toast/content
payloads. Resource-owning child objects remain with the production Host. Pure
geometry, lifecycle and reflow records contain IDs/numbers/epoch tokens only.

Active-time accounting, finite entry/exit deadlines and spring frame requests stay
separate. Use explicit native deadlines and weak owner callbacks while needed;
the upstream component's 50ms permanent advancement loop is not required.

## Layering bridge

Internal paired Op106 `Set_toast_layering` carries optional peek/gap/width-step
f64 values followed by a visible-layer int64. It admits only ToastStack owners.
Changing this metadata preserves the tree, editor owners and active-time clocks;
reset returns to the previous expanded column. Native dismissal stays immediate when Motion is absent.

The layered stack has a named Group focus stop without autofocus. Focusing the
Group or any contained control expands it. Page Up/Down and Home/End scroll while
the Group itself is focused, leaving editor key handling intact. Wheel input
bubbles from nested controls to the bounded native viewport. Back-card semantics
are hidden behind an identified ancestor; pointer shielding and Host eligibility
prevent input, focus, editor activity and expiry on decorative cards. Disabled
front content remains exposed as disabled. Child objects remain Host-owned.

Admission reserves the fixed wire record plus 512 quota bytes per configured stack
and 1024 per submitted card, in addition to existing toast/content payloads. These
are conservative bookkeeping quota units, not allocator RSS measurements. The
32-card submission bound also bounds measured descriptors and child wrappers.

## Motion bridge and scheduling

Internal paired Op107 `Set_toast_motion` carries optional spring parameters,
entry/exit milliseconds and slide distance. Admission requires a ToastStack and
reserves the fixed record plus 1024 quota bytes per stack and 4096 per submitted
card, in addition to any layering and child payloads. This bounds phase/trajectory
records and tasks; these quota units do not measure process RSS. Legacy toasts
also commit a Present paint using their existing owner reservation, so enabling
Motion never fabricates a new mount. Metadata reset releases trajectories during
acceptance without waiting for another draw.

The Host uses its native executor's monotonic clock for both active expiry and
finite phases. A phase deadline retains identity across paints. Early wakes clear
the old task before rearming; removal, reset and owner teardown cancel tasks.
The child-paint observer only commits opaque phase previews. The deferred frame
finish schedules timers and completes accepted dismissal tokens outside painting.
An initially transparent/slid-out entry remains eligible through its unslid layout
bounds; transformed bounds and opacity govern input. Hidden/rejected samples do
not schedule frames. There is no permanent advancement loop.

Ending adds a persistent input-retirement gate distinct from frame clipping. It
crosses modal portals and remains active through redraws; ordinary retained content
still paints, but its independent popup surfaces disappear and lose focus traps.
Saved focus is restored at acceptance. Closed keys stay closed after metadata
updates. Unmount/window close/overload discard pending terminal tokens. Changing
an accepted expiring toast to persistent does not retract its dismissal.

## Qualification

Layering's Core/protocol, admission, measured renderer, input gating, retained
editors and gallery are tracked in the [layering evidence](../evidence/toast-layering-och41.md).
The [renderer checkpoint](../evidence/toast-motion-renderer-och41.md) records the
native geometry and child-paint hooks before production integration. Current
production checks cover first-paint entry, expiry ordering, interrupted exit,
focus restoration, composition Escape precedence, nested-modal overflow retirement,
legacy order, reduced/hidden/inactive/disabled/zero-area policy, removal/overload/
window close and actual spring reflow without remounting editors. Public fixture,
Core, lint and installed-consumer checks are recorded in the
[motion evidence](../evidence/toast-motion-och41.md).
Real macOS GPU/input/IME/VoiceOver/resource and release qualification remain
required separately. TestPlatform composition is not physical IME acceptance.
