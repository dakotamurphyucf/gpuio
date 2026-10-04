# Button content and interaction

OCH-41 implementation contract, 2026-10-01. The pinned
[button review](../catalog/button-review.md) identifies the source behaviors.
Core/Bonsai and native paths use exact protocol epoch3, which includes this
epoch2 button contract and the later menu observation extension. Physical macOS
acceptance and full button-family coverage remain open.

## Public interface

`Button.Focus.t = Focusable of Tab_order.t | Preserve` separates ordinary native
focus, including explicit skipped Tab stops, from auxiliary pointer actions.
`Focusable Tab_order.default` is the default. `Preserve` keeps the existing sibling
focus on pointer activation and does not offer its own Tab, keyboard or semantic
Focus action. Semantic Click remains available if the button is enabled and idle.
Changing policy retains the keyed owner but removes focus if it becomes ineligible.
Disabled/inert/modal/hidden policies remain stronger than any button policy.

`Button.Config.t` is an abstract value with `loading` and `focus`. Its total
constructor accepts only validated `Tab_order` values. This is per-view policy;
`disabled` remains the existing ordinary-button argument or command availability.
Loading exposes AccessKit semantic Busy on the single button owner without changing its
role, accessible name or normal focus eligibility. It is independent of disabled:
it blocks activation and hover/pressed input
presentation, retires ordinary callbacks and fences command activation against
current per-owner config, while retaining enabled native focus. A busy command
button must not disable other owners or shortcuts for the same registry command.

Public constructors (existing arguments remain compatible):

```ocaml
val button
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?config:Button.Config.t
  -> ?accessible_name:string
  -> ?disabled:bool
  -> ?leading_icon:Icon.Decoration.t
  -> ?trailing_icon:Icon.Decoration.t
  -> on_click:(unit -> 'action)
  -> string
  -> 'action View.t

val button_with_content
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?config:Button.Config.t
  -> ?disabled:bool
  -> accessible_name:string
  -> on_click:(unit -> 'action)
  -> 'action View.t
  -> 'action View.t Or_error.t
```

`icon_button` and `command_button` gain the same optional config. A checked
`command_button_with_content` takes a command ID and one passive content view;
the resolved registry label remains its semantic name. Bonsai wrappers adapt
unit effects consistently with the existing button APIs.

Rich content uses one native action/focus owner with a bounded passive subtree:
containers, text/styled text, images/icons, avatars, loading/spinner/progress and
native animations. Nested interactive controls, callbacks, selectable regions,
scrolling or pointer shields are rejected. Names are nonblank UTF-8, at most
1024 bytes, without NUL. Bound content to 4096 descendants and 128 levels; validate
both construction and complete native tree admission, including late descendant
updates. Hide decorative semantics without suppressing paint or native clocks.

Loading does not invent a network task, timer or asset. The application chooses
its content, spinner/progress and announcement text. The gallery should show a
normal-looking busy control with whole-control dimming, matching the source's
presentation, and demonstrate that changing loading state preserves layout and
focus. A future convenience constructor can supply artwork using the same API;
do not put arbitrary asset ownership or synchronous callbacks into native config.

Application-controlled selected appearance remains ordinary current-model styling;
it is distinct from transient Pressed and `Command.checked` toggle semantics.
Connected group geometry, named visual variants, current-ID reducers and split
menu coordination follow separately. This primitive does not establish their
completion.

## Native and bridge contract

Pair a validated `Policy { loading; focus }` with a wire-only content mode
`Icon_slots | Rich`. Content mode is chosen by the View constructor, not an
ordinary application config. It avoids interpreting existing two-slot decorative
icons as a new rich subtree. Op69 `Set_button_presentation` carries
an optional configuration: `None` resets policy and legacy content interpretation.
Shape changes and resets must be atomic with child changes. Reject incompatible
kinds, invalid Tab indices and malformed passive content without partial mutation.

Reuse native button identity and the window focus manager. Route loading through
current native press/command checks and current Core dispatch policy so an event
queued before the configuration change cannot activate a now-busy owner. Do not
change the registry command's availability merely to block one button. Preserve
current disabled/inert, pointer occlusion, modal, virtual-row and generation gates.
Retained-label resources, loading clocks and command handlers must retire on node
replacement, eviction or window close.

The existing positive Int64 capability mask is full. This project ships OCaml and
Rust as one release and already requires an exact protocol version; it does not
negotiate arbitrary old backends. The button extension introduces **protocol epoch 2**, preserving existing tags and legacy capability
bits, with the new mandatory button behavior defined by the epoch. Epoch1 peers
must fail the handshake before any new operation is submitted. Future wire
extensions beyond the legacy mask require another explicit epoch change; do not
reuse a legacy bit, use the sign bit or silently widen the checkable contract.

This keeps the current compact handshake and explicit compatibility boundary.
The advertised version is paired now that the native/public path is connected.
OCaml also validates the native Welcome version and required mask before enabling
window/tree submission. Historical epoch1 byte fixtures test encoding only.
The historical Rust `v1` module name denotes the existing encoding family; it must
not be presented as a frozen epoch1 compatibility implementation after migration.
Persistent file formats and independently negotiated backend versions are separate
future contracts, not implied by this pre-release wire transition.

## Required evidence

Paired independent bytes and malformed-value tests; old/new handshake rejection;
Core/Bonsai constructors and stale-callback behavior; native complete-tree rollback;
rich/plain reset without replacing the owner; decorative label click/keyboard/AX
ownership; loading versus disabled, per-command-owner independence and late input;
normal/skipped/preserve focus including modal and virtual rows; scoped spinner and
progress cleanup. Public gallery and installed-consumer builds and real macOS
input/AX/GPU checks remain required for catalog acceptance. The value types alone
do not establish a supported feature.

## Native implementation checkpoint

The value types, independent OCaml/Rust config and Op69 byte fixtures, native
complete-tree validation, rich rendering and button focus/loading policy are
implemented locally. Production-View TestPlatform coverage verifies signed Tab
order, rich-label activation, loading focus retention, busy click consumption,
preserve versus skipped focus, retained owner identity through legacy reset and
session-close resource release. Session tests cover per-command-owner loading
and stale activation after a loading cycle without disabling another owner.

Public View/Bonsai attachment and Core queued-command checks are connected with
an accepted-state per-owner map. Both loading and command-reference changes fence
old events; preparations do not change accepted callbacks. The exact current epoch is paired (now epoch3).
These native foundations do not establish end-to-end or macOS GUI acceptance.
AccessKit Busy metadata and rich spinner/progress resource coverage now pass local
tests. The scoped macOS adapter now maps that flag to `AXElementBusy` with passing
headless AppKit getter coverage. External AX/notification delivery and VoiceOver
behavior remain release gates.
Public-gallery/installed-consumer runtime and physical keyboard/AX/GPU validation
remain open.

Local verification at this checkpoint: the native library with
`native-image-tests,native-canvas-tests` passes 487 tests (two existing isolated-bus skips), all three
button admission tests pass, the full Rust workspace and OCaml tests/format/gallery build pass, and strict native/protocol all-target Clippy and formatting pass. These runs
use TestPlatform or pure tests; they do not launch a physical desktop window.

The Controls gallery's “An action with room for detail” card demonstrates rich/plain
content, explicit busy state, shared-command independence and preserve/skip focus.
Its publish action only counts local requests; it has no external side effects.

A fresh independently installed public gallery consumer also builds successfully;
no runtime/desktop acceptance is inferred from that build. Build artifacts and
exact commands are retained in the ticket notepad; no unrelated switch changed.


### Busy metadata and decorative resource lifetime

The semantic wrapper sets AccessKit Busy on the button root without changing
its role, label, toggle value, focus eligibility or disabled state. A production
semantic-builder regression covers false/true/false projection. The scoped
`busy-state.patch` now exposes `AXElementBusy` as a read-only Boolean and queues
`AXElementBusyChanged` independently of ordinary value changes. It extends
attribute enumeration, retains Braille getters and delegates unrelated legacy
queries to AppKit. There is no public modern busy getter in the installed SDK.
See [adapter provenance](../../vendor/accesskit-macos/GPUIO.md) for Apple sources,
reconstruction and the exact native boundary.

The headless `accessibility_busy` executable uses a main-thread unattached NSView
and actual AccessKit platform objects without creating NSApplication or a window.
Button, Link and editable text-field cases pass Boolean type/enumeration,
read-only busy state, ready/busy/disabled/recovery, modern role/name/value/focus,
Braille values, retained identity and stale-node retirement. Enabled text fields
retain AXValue writability; the patch does not change upstream disabled versus
read-only text policy. AppKit's legacy getter does not project modern Role/Value;
the fixture checks their modern selectors directly instead of assuming it does.
The public gallery drivers now require external busy attributes. They remain
unrun; no external AX notification receipt or VoiceOver acceptance is inferred
from direct getters or from queuing a notification.

A production View/Session test embeds an SVG-backed spinner and circular progress
inside a busy rich button. Both clocks advance while busy; hidden, inert and
transparent ancestors settle to no requested frames while retaining owners.
Restoring visibility resumes clocks without replacing the button focus handle.
After releasing the registered SVG, changing to plain content or closing the
window invalidates retained drivers and releases the retired asset lease even
when the View entity is retained. Session close returns retained tree bytes to zero.
This is TestPlatform resource evidence, not process RSS or physical GPU acceptance.

`scripts/test_gallery.py --section buttons` now contains the public macOS
walkthrough for rich/plain identity, shared-command loading, real pointer/keyboard
activation and skipped/preserved focus. The preview names primary/secondary action
groups to distinguish their intentionally shared command label. The driver is
compiled but **unrun**; it now requires AXElementBusy getter assertions while
external notification and VoiceOver gates remain open.


## Independent hover observations — unpublished epoch 3

`View.with_hover view ~on_change` (also in Bonsai View) attaches an asynchronous
Boolean observer to a direct Button, CommandButton or Link. Put it inside a
Tooltip anchor. Unsupported root kinds return `Or_error`; captured pointer
regions and tooltip-open observers remain different APIs. The observer has its
own generation-checked handler and does not replace the click handler or native
focus owner. Removing it and adding it later retires its old queued callbacks;
merely changing its closure uses the latest accepted callback without rebinding.
Prepared but unaccepted Views do not change delivery.

Op71 `Set_hover_observer (NodeId, HandlerId option)` and Event68 `Hover_changed`
(WindowId, NodeId, HandlerId, accepted revision, Boolean) extend the current
**unpublished paired epoch3**. Neither a new legacy capability bit nor a public
version compatibility promise is introduced. Both runtimes must be rebuilt.
The observer emits its last native observed snapshot (false before first hit
testing), then ordered changes. It follows GPUI's hover listener, including
keyboard input modality, press/drag policy, hit testing, clipping and occlusion.
Unavailable/loading/gated or culled controls report false. These events describe
asynchronous history; an earlier queued entry may precede the later exit after
an application update. They are not authorization for actions, and styles do
not wait for a callback/FFI round trip.

Native observation state is keyed by accepted Node identity. GPUI can retain its
hover listener state while a paint owner is evicted. Recreating only our state
with raw=false lost stationary-pointer hover on remount; a failing regression
proved this and now passes with retained observation state. Culled owners report
false; remount reevaluates eligibility using the retained last native observation.
Actual node removal, generation replacement and window disposal clear the cache.
There is at most one state per accepted observed node, bounded by the tree limit;
once opted in, its native listener tracks until node retirement even if temporarily
unsubscribed. There is no idle timer or unbounded list history.

Observations use the existing bounded input queue, including initial snapshots.
Overflow faults the window; it does not silently drop an edge. Enqueueing occurs
immediately, while only fault mutation is deferred until the renderer's shared
Session borrow ends. TestPlatform checks preserve 128 ordered edges followed by
exactly one overload result and reject later ordinary input.

Local evidence covers independent codecs, accepted callback/rebinding, invalid
root rollback, pointer entry/exit/deduplication, loading/recovery, native owner
eviction/remount, click/focus retention, unmount and overflow. The eviction fixture
exercises the lower-level owner path; it is not full managed-list acceptance.
Additional TestPlatform checks pass command-registry enabled changes, Link
loading, disabled/inert/hidden/display-none ancestors, keyboard modality, modal
trapping/recovery, surviving actions and final cache disposal. Native editor-target
command context, clipping/occlusion and actual managed-list/multiwindow workloads,
physical macOS pointer/key delivery and installed-consumer runtime still need
focused acceptance. The public appearance card displays the currently hovered
action, and its extended desktop driver is authored, not yet run.
