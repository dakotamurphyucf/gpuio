# Positioned menu commands

OCH-41 implementation design; acceptance is tracked separately. The public
surface is `Gpuio_eio.Menu_controller`, attached to one `View.context_menu` by
its stable `key` and `observe` callback. This works with the drawn context menu
and with `~platform:true` (AppKit on macOS, drawn fallback on Linux).

`Menu.Position.create ~x ~y` validates finite logical window coordinates in
[-1,000,000, 1,000,000]. The origin is the content area's upper-left corner;
coordinates are not screen pixels. AppKit may adjust placement to fit a display;
the drawn fallback is constrained to the window. Neither route promises the
requested point equals the final popup corner.

`Menu.Command.Show position` and `Close` return asynchronous typed results.
A successful reply acknowledges native admission, not paint, physical display,
user selection or completion of a command callback. `Menu.Snapshot.is_open`
observes native accepted open state, including a queued AppKit tracking lease.
Ordinary registry command invocation remains a separate asynchronous event.
No callback into OCaml runs within native menu tracking.

The controller captures the exact observed window/node/subscription. Definition
or presentation replacement rotates that subscription; detach, removal and window
close also invalidate it. A delayed command cannot target a replacement. Style
changes and callback-only changes preserve subscription identity. Native state
is rechecked before opening and again before selection dispatch.

At most 64 requests may be pending per application. Errors distinguish no local
mount, closed window, stale subscription, unavailable interaction, invalid
position, busy tracking and native failure. Show requires an active window and
an eligible, mounted context owner. It rejects an already open owner; AppKit
also enforces one pending/tracking popup across the application. Close is
idempotent for a current subscription and may retire a hidden or inactive owner.
It cannot close another menu or a replacement subscription.

Use `reset` when removing a controller's view to clear local bookkeeping. Native
identity validation is authoritative even if that effect has not run. Closing
a window completes its pending requests once; wrong-identity or duplicate replies
cannot complete a different request. A successful show can still be cancelled
before tracking starts if its owner becomes unavailable.

The paired unpublished protocol appends message 23 and result event 81; both
packages must use the same revision. Existing menu visibility events carry the
subscription identity. Required qualification includes fixture/truncation/bounds,
controller correlation/close, definition replacement, drawn position/close,
AppKit positioned input/close, stale requests, overlap, and installed consumer
behavior. This document alone is not implementation or platform acceptance.

## Local qualification

[Current evidence](../evidence/menu-commands-och41.md) records paired codec and
identity tests, production drawn-state checks and a fresh installed AppKit
walkthrough. The example includes both ordinary controller usage and a clearly
marked Expert call demonstrating stale-definition rejection. Native icons,
multi-window/editor-target scenarios and release qualification remain separate.
