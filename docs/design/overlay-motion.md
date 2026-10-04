# Modal entry presentation

OCH-41. `View.dialog`, `View.alert_dialog` and `View.sheet`, including Bonsai
helpers, accept `?motion:Overlay.Motion.t`. `Immediate` preserves the default.
`Enter` runs a native entry on each newly visible surface:

- Dialogs and alerts fade the complete modal surface, including its backdrop,
  while moving the panel from 16 logical pixels above its settled position over
  250ms.
- Sheets slide 100 logical pixels from their attached edge over 150ms, clamped
  to their actual extent. Their backdrop is immediately visible. All four edges
  use the same native geometry and input rules.

These are functional entry equivalents to the pinned styled components, not
pixel-identical upstream theme margins or shadows. The gallery's **Animate
opening** toggle demonstrates the public API. No OCaml frame callbacks occur.

The existing focus trap and viewport input blocker apply from the first frame,
including fully transparent paint. Automatic child focus waits on the modal scope
until entry settles, because transient motion can place early controls outside
the viewport. Tab/click may focus a visible child during entry; that deliberate
choice wins and is never replaced by delayed autofocus. Immediate mode retains
the existing immediate autofocus behavior. The panel's visible position, native hitboxes,
logical surface containment and accessibility bounds move together. Animation
changes neither layout size nor the native ownership of child controls. A stable
frame identity preserves their accessibility identities through option/style
changes. Separately deferred child popups retain their own presentation.

The first actual paint starts the native clock. Ordinary rerenders and backdrop,
theme, style or extent changes do not restart the elapsed entry. Disabling motion
or enabling system reduced motion settles immediately; subsequently enabling it
again does not replay that visible mount. A hidden surface is omitted and retires
its GPUI frame state; showing it again enters afresh. Applications that want a
fresh entry can close/reopen the surface. Removal disposes content immediately:
there is no outgoing retained overlay, delayed focus release or exit animation.

The bridge appends Op91 `Set_overlay_motion (NodeId, bool)`; false is immediate.
Admission permits true only on a modal FocusScope in the final accepted tree.
Changing to a popover/removing overlay metadata must also clear motion. Failure
is atomic. Modal accounting reserves 256 bytes for fixed entry/frame bookkeeping,
including the stable immediate-state wrapper; this is not an RSS measurement.

The wrapper lives **inside** the deferred viewport surface. It obtains state
from GPUI's frame identity and requests another frame only after painting an
unfinished entry. There is no detached task, timer, content owner or permanent
idle polling. GPUI may deliver an already requested final frame after removal;
it cannot continue a removed entry. This avoids assuming an outer `View.animate`
wrapper can animate a deferred panel or backdrop.

Managed tooltip transitions have a separate [contract](tooltip-motion.md).
[Point/corner placement](placement-geometry.md) and [sheet insets](sheet-insets.md)
have their own public contracts. TestPlatform validation does not
replace physical macOS input, visual, accessibility or resource acceptance.
