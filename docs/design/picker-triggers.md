# Date and color picker triggers

OCH-41, 2026-10-02. Both Eio pickers support `view_with_trigger`, alongside the
existing plain-label `view`. Each renders one native button inside the same
popover composition. Applications supply passive content and an accessible name;
the controller owns activation, disabled policy and the open/cancel action.

Use formatted date text, a color swatch, decorative images/icons or another
passive presentation. The existing rich-button validator bounds the tree to
4096 nodes and depth 128 and rejects nested controls/callbacks. The accessible
name must be nonblank UTF-8 without NUL and at most 1024 bytes. It is independent
of visible formatting, so applications should include relevant value/context in
localized names where useful. Construction returns `Or_error.t` and has no picker
side effects.

`trigger_style` styles the button; the existing `style` parameter styles the popup
panel. The plain-label helper also accepts `trigger_style`. Calendar appearance,
date presets, Apply/Cancel labels and native draft behavior are shared between
plain and rich helpers. Use one helper per controller, not duplicate placements.

Changing only trigger content retains the native button and open calendar/color
input draft. Replacing a plain trigger with a rich trigger keeps its button kind
and identity. The popup's existing dismissal, fresh-opening identity and explicit
Apply rules remain unchanged. Content does not become a second focus owner.

Clear belongs beside the trigger as a separate application action. The gallery
cancels the current opening and uses its controlled-value setter to clear the
committed value, with those buttons disabled under its read-only policy. Cancel
also discards a draft when the committed value was already empty. Updating an application value also
invalidates an incompatible open draft through the existing controller contract.
This is distinct from the date picker's Clear preset, which changes only the open
draft and still requires Apply.

These triggers retain the established native Button role, rather than claiming
the pinned base DatePicker's ComboBox behavior. Shared popover metadata now adds
expanded state and the AccessKit dialog-popup kind, as described below. No extra
Rust input owner or callback from native rendering into OCaml is introduced.

## Shared popover trigger state

`View.popover` marks its existing composite container with Op88 `Set_popover`.
The accepted structure has one anchor and, while open, a second child carrying
the existing nonmodal Popover overlay. A direct Button or CommandButton anchor
exposes `expanded` and AccessKit `HasPopup(Dialog)` on its existing semantic node.
Names, Button role, native focus, keyboard/mouse/AX activation and application
callbacks remain unchanged. Both plain and rich date/color triggers benefit
without application changes. Arbitrary custom anchors remain supported; there is
no descendant search that could select the wrong nested activation owner.

Expanded follows the surface's native eligibility: absent, display-hidden, inert
or disabled popup surfaces report false. Disabling only the trigger does not
report a visible sibling popup as closed. A queued open/dismissal callback changes
nothing until its tree update is accepted. Nested popovers report independently.
Closing removes popup resources. If focus is still inside, a direct button anchor
is the preferred return target while it remains eligible, including after semantic
activation or replacement of another popup. Otherwise previous-focus fallback
applies; closing does not steal focus after it has moved outside. Custom anchors
retain previous-focus restoration. Removing the marker on the same nodes clears
both semantic properties and that explicit return target.
No new Expand/Collapse actions are advertised: activation is the existing,
application-controlled callback. Late AX Clicks after a button is disabled are
consumed rather than synthesized into pointer clicks that could dismiss a popup.

Core reconciliation emits only marker changes. Native admission validates the
final composite shape, including child-only overlay changes, and rolls back
invalid transactions. The marker adds one fixed-size boolean, no strings, timers,
queues or owners. The bridge requires matching unpublished epoch-3 builds; old
operation encodings are unchanged. macOS has an AXExpanded getter for this state;
physical VoiceOver announcements and platform-specific popup-kind presentation
still require OCH-17/OCH-47 qualification.
