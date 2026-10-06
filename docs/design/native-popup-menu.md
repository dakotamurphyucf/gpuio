# Native popup menus

OCH-41 implementation design, 2026-10-06. Work in progress; this document is not
platform acceptance. Existing drawn menus remain the default.

## Interface and scope

The local implementation adds `View.context_menu ?platform:bool ~menu child` (default `false`) in Core and
Bonsai. `true` selects AppKit's OS popup on macOS and the existing drawn context
menu on Linux. Right-click uses the event's logical window position; Shift-F10
uses the owning wrapper's lower-left corner. No raw native handles or Rust
callbacks cross the OCaml boundary. Menu labels, command references, separators,
checks and nested disabled submenus reuse the existing bounded `Menu.t`.

The wrapper's ordinary View styling still applies. AppKit owns popup appearance,
placement adjustment, keyboard navigation and accessibility. `Menu.Appearance`
applies to the drawn Linux fallback. Arbitrary View row slots cannot become
AppKit controls; reject `with_menu_item_content` for platform context menus on
both platforms so a portable program cannot silently lose content. Passive
section labels remain disabled, non-actionable native rows.

This opt-in route also supports [positioned commands](menu-commands.md) through
`Gpuio_eio.Menu_controller`: typed Show/Close requests use the exact observed
subscription and the same adapter/command validation. [Local installed evidence](../evidence/menu-commands-och41.md)
covers positioning, overlap rejection, close and stale ownership. Native
[decorative SVG icons](../evidence/native-menu-icons-och41.md) now have typed slots,
bounded worker-backed rasterization and AppKit template snapshots. Broader platform
acceptance remains required catalog work.

## Ownership and dispatch

Prepare bounded native menu data while the owner is valid. Start tracking in a
main-run-loop callback after releasing GPUI, View and Session borrows and the
serial main dispatch queue. AppKit runs a
nested event loop; no OCaml callback participates in navigation or selection.
Retain the NSView, menu, and action target through the operation instead of
capturing an unowned integer pointer. The target only records a selected index.

Use one application-wide active popup lease: reject overlapping starts rather
than recursively entering another tracking loop. The retained owner cancels
tracking on disposal; cancellation consumes the result before asking AppKit to
stop, since native calls may reenter. Definition replacement, hidden/disabled/
modal-blocked owners, unmount and window close invalidate pending selection.

After tracking returns, refresh the still-live GPUI window even on dismissal.
Revalidate the owning full NodeId, native state identity, menu definition, focus
gates and current command generation/availability before routing a selection.
Never keep a GPUI borrow across AppKit calls. Native editor targets must remain
explicit rather than accidentally following a newly focused editor. Application
actions use existing asynchronous bounded delivery.

## Required qualification

Paired OCaml/Rust encoding and admission tests cover the added presentation tag,
unknown tags, context-child shape, prohibited rich slots and atomic rejection.
Native lifecycle tests cover delayed start, owner/config replacement, disabled
ancestors, stale commands, overlap, cancellation and cleanup. Actual macOS tests
must exercise right-click, Shift-F10, Escape, nested/disabled/checked items,
repeat opening, invocation, native edit routing and popup outside window bounds.
A public gallery and independently installed consumer must exercise this route.
AppKit model tests do not prove input, pixels or VoiceOver behavior. Linux build/
unit/consumer remain required; real desktop behavior stays with OCH-47.

## Local implementation checkpoint

The paired unpublished protocol appends `Platform_context` / `PlatformContext`
as presentation tag 5; existing tags remain unchanged. Both sides must use the
same repository revision. Independent fixture bytes and unknown-tag checks cover
the new value; native admission rejects rich slots and a missing context child.

The AppKit runner holds a weak reference to the retained GPUIO menu state. A
strong capture would prolong that state after window disposal and prevent owner
cancellation. The runner instead retains only its AppKit objects and command
snapshot. Owner disposal invalidates the result immediately, then schedules
`cancelTrackingWithoutAnimation` on the foreground executor, outside the
borrow that disposed it. Native edit invocation additionally requires the
captured target node and focused-handle identity to remain current.

The standalone menus example has a `--platform-popup` flag and an adjacent
[walkthrough](../../examples/menus/main.md). The Feedback gallery's Advance
button also exposes this platform context menu. The physical test is
`scripts/test_native_popup_macos.py`; it checks AppKit menus directly under the
owned window, keeping them separate from the application's global menu bar.
Its submenu test waits for actual native selected children, because initial
highlighting can depend on the previous pointer position.

Still required before closing the native-popup catalog row: native icon qualification,
multi-window overlap and native editor focus-change cases,
and consolidated gallery/platform acceptance. Window close, owner removal,
stale-command rejection and independently installed consumer interaction now
have local macOS evidence; these do not imply all lifecycle cases are complete.

## Tracking-loop scheduling correction

A local regression opened the real AppKit popup, then issued an OCaml/Eio window
close after three seconds. The initial foreground-task implementation logged the
close request but did not exit within an eight-second wait after the popup was
observed. The
pinned GPUI dispatcher runs foreground tasks on the serial main dispatch queue;
calling the tracking loop inside one such task prevents queued close work from
making progress. Normal menu selection tests did not reveal this failure.

The adapter now schedules a one-shot block using
[CFRunLoopPerformBlock](https://developer.apple.com/documentation/corefoundation/cfrunloopperformblock(_:_:_:))
in common modes and explicitly wakes the main run loop. Apple documents that
this API enqueues and copies the block instead of invoking it immediately.
`AsyncApp` borrows are taken before and after tracking, not across it. The block
captures only main-thread-owned objects, and its one-shot cell borrow is released
before entering AppKit. This adds a direct dependency on the already pinned
`objc2-core-foundation` 0.3.2 with its block feature; no package version changed.
The exact close regression now passes. The versioned driver also passes owner
removal and stale-command selection during tracking, against both the local and
independently installed public consumer. [Evidence](../evidence/native-popup-och41.md)
records the failure, correction, hashes and remaining scope.

## Owner transition qualification

The [lifecycle follow-up](../evidence/native-popup-och41.md#owner-transition-and-recovery-follow-up)
also qualifies definition replacement, hidden/disabled ancestors and modal entry,
including reopening, command delivery and preserved editor content, on a fresh
installed-library consumer. These checks do not establish multi-window overlap
or native editor retargeting behavior.

## Decorative icon contract (implementation in progress)

`View.with_menu_item_icons view ~items` accepts item paths paired with registered
SVG `Asset.Handle.t` values. The receiver is a direct platform context menu.
Unknown/duplicate paths, separators, raster handles and other receivers are
errors. Empty items remove all icon slots. Command, submenu and section labels
remain authoritative for both text and accessibility. Native admission accepts
only direct passive decorative Icon children in these slots; arbitrary rich
View content remains rejected. No new protocol tag is needed, but both bridge
ends must use the same unpublished repository revision.

The icon is 16 logical pixels square, aspect-preserving. AppKit uses a template
image and chooses its tint; the Linux drawn fallback composes the icon beside
the existing text with inherited foreground. View styling does not override OS
menu appearance. Icons are keyed by item position, as with drawn content slots.

Mounted slots acquire encoded leases when the tree transaction is accepted,
before subsequent registration release. The existing bounded image service
rasterizes SVGs off-thread directly to the menu's display density. Opening a
popup observes ready pixels without adding a GPUI atlas copy. Missing, loading,
failed or over-budget decoration is omitted; navigation and command dispatch
remain available. A density change requests replacement pixels through the
mounted lease and temporarily retains the previous raster. A subsequent open
observes the replacement. An already tracking popup freezes its bitmap snapshot.

AppKit receives interleaved alpha-bearing template bitmaps copied from validated
ready pixels, never encoded SVG/image data. Each side is bounded to 256 physical
pixels (16 logical pixels at maximum supported density); aggregate bitmap
payload is bounded to 8 MiB per snapshot, counting duplicate icons. The existing
one-active-popup lease bounds simultaneous snapshots. NSMenu retains its images
through tracking/cancellation; all are released with that native snapshot.
Decoded source pixels remain subject to the existing cache/retired-reader budget.
These are payload bounds, not measurements of AppKit's private allocations/RSS.

Required evidence: paired public transactions and native admission, passive and
atomic rejection, lease retention after registration release, density changes,
loading/failure, no atlas charge, bitmap alpha/size/template/budget, nested icon
placement, selection and teardown on actual macOS and an installed consumer.
Linux fallback needs build/unit/consumer evidence; desktop acceptance stays 07b.
