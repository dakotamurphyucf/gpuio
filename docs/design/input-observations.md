# General native input observations — OCH-41

Implementation in progress. The domain and codec work described here does not yet
advertise a runtime capability or provide a mounted input region. The catalog gap
remains open until native routing, reconciliation and public gallery tests pass.

## Public contract

`Input_region` describes subscriptions and immutable observations. The planned
`View.input_region ~config ~on_event children` is a styled container, independent of
`View.pointer_area` (which captures a gesture). It can enclose ordinary text,
containers or native widgets without replacing their owned editor/control state.
Core callbacks return arbitrary actions; Bonsai callbacks return effects. No OCaml
callback executes inside native dispatch, layout, paint or an OS delegate.

Subscriptions opt into click, auxiliary click, mouse down/up/move/enter/leave,
outside-down, key down/up, focus/blur and wheel observations. One subscription per
kind, at most thirteen. Subscription order is not event order. Raw down/up/move,
key and wheel listeners choose capture or bubble phase and a static native policy:
observe, stop propagation, prevent default, or both. Default is bubble/observe.
Clicks, hover transitions, outside-down and focus changes are derived/native
notifications; these require bubble/observe rather than pretending that an async
callback can cancel an earlier event. Native controls may consume bubbling input;
capture observation is explicit when an application needs to observe descendants.

The region has a nonblank accessible label, disabled state and explicit focus
policy: none, pointer/explicit focus, or pointer/explicit focus plus Tab traversal.
Key observations can observe a focused descendant without making the region a Tab
stop. Direct region focus/blur subscriptions require a focusable region; they do
not mean focus-within, application activation, or child-editor focus. Native modal
scopes, hidden branches and disabled state remain authoritative for eligibility.

Mouse positions use logical pixels, both window and current region-local
coordinates. Negative/outside local coordinates are valid. Down/up/click samples
include button, click count and modifiers; movement has an optional pressed button.
A click requires a matching native down/up on the eligible region, not any mouse
release. Auxiliary click excludes the primary button. Captured dragging remains a
separate API. Hover/focus observations are edges, not per-frame samples.

Wheel payloads preserve `Pixels` versus `Lines`, both axes, modifiers, position and
touch phase. Do not invent a fixed line-height conversion or conflate wheel input
with a viewport change. Zero deltas remain valid for phase boundaries. Key samples
preserve GPUI key/optional character/modifiers; repeat applies only to key-down.
Key observations are not committed text or IME composition; use the native editor
for text input. Raw click observations carry mouse data, not invented keyboard
coordinates. Use semantic buttons/commands for accessible keyboard activation.

## Routing and lifetime requirements

A subscription/configuration change must retire the prior observation binding;
ordinary callback closure updates use the latest accepted closure without a new
binding. Native window/node/handler generations fence queued observations and
retained callbacks. A node keeps its native identity and child editor state when
configuration changes. Hiding, disabling, modal blocking, branch departure,
window deactivation and close must clear pending clicks/hover and release owned
focus subscriptions. No stale callbacks after removal, reset or handler reuse.
No requirement to deliver synthetic blur/leave after unmount: an observation is
not a resource-lifecycle hook.

Only consecutive movement samples for the same live route, tree revision,
modifiers and pressed-button state may replace each other in the bounded mailbox.
Do not coalesce across another event or erase click/key/focus/wheel edges. Wheel
deltas are not replaceable samples. Reuse the transport's existing explicit
window-overload behavior; no unbounded event history, per-frame OCaml polling or
extra runtime timer. Native policies execute even if the callback has not run.

## Implementation and acceptance sequence

1. Validated public types and independent bounded Rust/OCaml codecs. Share pointer
   button/modifier representations without changing existing pointer wire bytes.
2. Append protocol envelopes and negotiate a capability only when the host can
   honor them. Reconciler/session checks, atomic rejection and binding retirement;
   opt-in mounted region with native hit testing, focus and current modal gating.
3. Mailbox resource/coalescing tests and native tests for nested capture/bubble,
   all buttons, click cancellation, hover/outside, scroll units/phases, focus/keys,
   child editor/IME isolation, updates/removal and multiple windows.
4. Public gallery, actual macOS keyboard/pointer/accessibility checks, retained
   child state and teardown. Update all thirteen event rows with measured evidence.
   Linux build/unit checks remain required; Linux desktop acceptance stays OCH-47.

Pinned reference: GPUIX `18e695ed0ee8121a7793413ca795e08eda2a13df`,
`packages/native/src/renderer.rs` event dispatch and focus subscriptions. This
provides functional coverage with typed contracts; its release-as-click fallback,
implicit down+move capture, and fixed 20px line conversion are not adopted.
