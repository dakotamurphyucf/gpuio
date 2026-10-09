# Disclosure presentation extensions

OCH-41, 2026-10-02. The accepted `Disclosure` application model and native header
keyboard/focus behavior remain the foundation. This document specifies rich-header composition and native measured reveal.

## Rich accordion labels

`View.accordion_with_labels` in Core and Bonsai is additive to `View.accordion`.
Applications supply passive View labels keyed by `Choice.Id`. Each item keeps
one whole-header native button with the Choice text as its accessible name.
Labels can combine icons, primary and secondary text, and other content allowed
by `button_with_content`. Nested actions, editors and other interactive owners
are rejected; independent header actions still use `disclosure_with_header`.

Unknown/duplicate IDs fail. Omitted labels use the existing string button.
Rich labels share an aggregate 4096-node, 128-level budget, independent of native
whole-tree quotas. Supplied rich labels use the button's nonblank 1024-byte
accessible-name limit; omitted string labels retain Choice's 4096-byte bound.
This difference is validated before reconciliation, not silently truncated.

Each header is an accessible heading containing its toggle. The heading level
is explicit, defaults to three and is validated in 1..6. Native Tab, header
Up/Down/Home/End, Enter/Space, expanded state and collapse-focus restoration
reuse the existing implementation. Current expansion stays application-owned;
callbacks carry stable-ID Toggle requests applied against the latest model.

Shared trigger/panel styles and a pure per-item outer-style callback allow
borders, padding, sizes and native hover/focus states. Title/icon styling belongs
in the passive label. Heading metadata does not imply automatic browser heading
fonts. The helper does not prescribe a chevron asset or pixel-match the upstream
styled component.

Retain builds and preserves native panel children while collapsed; Unmount skips
collapsed content construction and removes native descendants. Neither policy
silently deactivates separately evaluated Bonsai computations or cancels Eio
application tasks. Switching policy is explicit. Header identities remain keyed
by Choice ID while text/styles/expansion change.

## Native measured reveal

Core/Bonsai `View.panel`, `disclosure`, `disclosure_with_header`, `accordion` and
`accordion_with_labels` accept optional `?motion:Disclosure.Motion.t`. Omission
uses `Motion.immediate`. `Motion.standard` uses stiffness 400, damping 40, mass 1,
epsilon 0.1 logical pixel and a two-second maximum duration. `Motion.spring`
accepts an existing validated `Animation.Spring.t`.

Initial mounting is settled. A semantic toggle starts from the last painted
position and velocity; reversing a transition preserves that position. Content
and spring changes retarget without extending the toggle's deadline. Settled
open content uses ordinary layout directly, including wrapping and streaming in
the current frame; there is no previous-height correction cycle.

`Retain` may paint outgoing content behind a shrinking clip. The logical tree
immediately hides it, returns focus to the eligible disclosure trigger and blocks
keyboard, pointer, IME routing and accessibility actions. Outgoing painted AX
nodes remain under a hidden ancestor until they leave layout; their presence in
a raw AccessKit tree is not exposure to assistive navigation. The renderer's
local display override never modifies the logical tree. Closing uses the most
recently expanded display mode.

`Unmount` immediately removes descendants and skips closing motion. Opening can
animate newly mounted children from zero. Changing retention policy or moving a
panel to another parent settles current motion. Separately evaluated Bonsai
computations and application tasks still follow their explicit lifetimes.

### Layout rules

`reveal_layout::Reveal` returns the body's own layout ID when settled open.
During a transition it uses normal GPUI column layout with a nonshrinking body,
clipped in both prepaint and paint. Placement/width constraints move to the
outer box; content padding, border and absolute height constraints remain on the
body. Closed lays out no child. No GPUI fork, recursive measured-layout call or
second measurement pass is involved.

Motion is supported in ordinary nonwrapping vertical flex flow. Parent-filling
flex growth, a non-auto basis, relative height constraints, aspect ratio,
absolute placement and row/grid parents use immediate layout. Shrinking panels
under a height-constrained parent also fall back. Root/native placements that do
not supply a flow parent, and panels or immediate parents with deferred native
interaction styles, use immediate layout rather than guessing geometry. Ordinary
style and layout behavior is preserved in all these cases.

Only the border-box height interpolates. External margins and parent gaps are
removed with Closed; use panel padding and zero parent gap for continuous reveal
spacing. The gallery follows that arrangement and offers an Animate toggle.

### Motion-state checkpoint

The state machine has Natural, Closed and physical Height presentations. Samples
are pure; only paint commits measurements. Configuration epochs and older paint
timestamps fence stale samples. A stale closed paint cannot cancel a newer
opening and does not erase the last known natural height. Both spring completion
and out-of-domain fallback request a final settled-layout frame. The host honors
that request even when the spring is already retired.

Heights above one million pixels use ordinary layout rather than truncation.
Reduced motion, inactivity, hidden/inert/disabled placement, full clipping and
zero opacity settle the owner. A zero-height opening at a visible origin still
measures its child and schedules the first animation frame. No idle timer or
per-frame OCaml callback is involved.

### Ownership and bridge

Op85 `Set_reveal (node, config option)` appends to the experimental protocol. Its
configuration carries expansion, Retain/Unmount and validated spring parameters.
Only Panel accepts it. Final admission checks run for every changed node,
including style-only updates: a collapsed configured panel must remain logically
Display Hidden, and an unmounted collapsed panel must have no descendants.
Malformed or inconsistent transactions reject atomically.

The native host owns one bounded state per configured panel, with the GPUI
executor's monotonic clock and at most one weak pending frame lease. Frame-end
sweeps retire unpainted owners. Removing configuration/node or closing a window
cancels work even if a diagnostic reference still retains the state. Admission
reserves 512 bytes per owner plus the config size within ordinary window/session
budgets; this is logical retained-resource accounting, not process RSS evidence.

Core expect tests, paired codec fixtures, admission tests, deterministic layout
and host tests cover the contract. Physical macOS GPU/input/IME/VoiceOver and
measured resource/release acceptance remain required; see the
[evidence](../evidence/disclosure-presentation-och41.md).
