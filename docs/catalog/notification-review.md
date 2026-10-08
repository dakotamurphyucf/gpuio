# Pinned notification and toast review — OCH-41

Reviewed on 2026-10-03 against gpui-kit
`84f57fdfcb4910623fb0bb7f795b077e249f9271`. Exact `base-toast.rs.txt` and
`component-notification.rs.txt` snapshots are in `sources/`, with source paths
and SHA-256 in its manifest. They were extracted from that revision's Git objects,
not copied from a possibly edited worktree. The source inventory audit verifies
the snapshots; it does not establish implemented behavior.

## Functional mapping

| Pinned behavior | GPUIO contract and current evidence |
| --- | --- |
| Stable IDs, ordered push, replacement and clear | Application-owned keyed `View.toast` values inside `View.toast_stack`. Update an existing key to preserve a native session; use a new key to start a fresh lifetime. Application removal/clear is explicit. Upstream `push` replaces/restarts an existing ID; ordinary GPUIO rerender must not restart timeouts. |
| Active-time timeout, persistent toast, pause for pointer/focus | `Toast.Timeout`, retained native clock and stack hover/focus pause. Hidden/modal-blocked items pause. Exact time arithmetic and native tests are recorded in [OCH-11 evidence](../evidence/native-toasts-och11.md). |
| Bounded visible set | `Stack.max_visible` is 1–8 and submissions are capped at 32. Overflow dismisses older sessions rather than retaining an unbounded hidden queue. These are explicit safety bounds, not the upstream default of ten. |
| Title, description, tone icon, custom body and action | Ordinary typed text/icon/layout/button content under `View.toast`, styled with theme tokens. `Timeout.persistent` explicitly implements persistent action notifications. Upstream severity names are presentation helpers rather than distinct native state. |
| Close, Escape, callback and programmatic removal | Typed `Dismissal` for timeout, close, Escape and overflow, plus explicit application removal. Native composition gets first Escape; a subsequent Escape dismisses. Body actions can dispatch application removal through ordinary controls. Do not imply a built-in whole-toast click or middle-button dismissal policy from the upstream convenience component. |
| Four corner placement, width and independent stacks | `Toast.Corner` and `Toast.Stack.create` provide four corners and bounded width; separate keyed stacks own independent timers/focus. |
| Center anchors and configurable window margins | `Toast.Placement` now supplies all eight anchors and checked per-edge window margins through `Toast.Stack.create ~placement`. Native geometry clips the usable rectangle, preserves child owners and gates zero-area focus/input/accessibility. Core/codec, production Host and installed-consumer checks pass; physical/release qualification remains open. See [placement evidence](../evidence/toast-placement-och41.md). |
| Measured collapsed layers, peek/scale, pointer/focus expansion | `Toast.Stack.Layering` now supplies checked peek/gap/width reduction and visible layers. Current-frame variable-height layout supports pointer/focus expansion, a named non-autofocusing Group entry, bounded wheel/keyboard scroll, decorative-card input/AX shielding and retained editors. The expanded column remains the default. See [layering evidence](../evidence/toast-layering-och41.md). |
| Interrupted reflow, entrance/exit and lifetime phases | `Toast.Stack.Motion` now supplies independent native spring reflow and finite slide/fade. Entry starts on child paint and precedes active expiry; accepted dismissal retires input/focus immediately and publishes once after exit. Current-frame anchored measurement preserves streaming geometry and painted velocity. Reduced/hidden/inactive policy settles, and unmount/overload/window close discard pending events. Core/paired protocol, production Host and public gallery checks are recorded in [motion evidence](../evidence/toast-motion-och41.md); physical qualification remains open. |
| System-only and combined delivery | `Gpuio_eio.Notification` owns capability/permission-aware OS posting, stable tags and actions; in-app Toast stays a separate API. Applications compose both and route actions explicitly. No automatic title/body extraction or global callback replacement is required. OS acceptance is recorded separately under OCH-28/OCH-17; Linux desktop qualification remains OCH-47. |
| Public animation/style tokens | GPUIO theme styles can style ordinary toast content. Checked `Layering` and `Motion` metadata configure stack geometry and animation without callbacks into OCaml from native paint. |

Feedback demonstrates ordinary timeout/dismissal, all eight anchors and insets,
plus three persistent layered sample cards of different heights, expansion and
individual dismissal/restoration. Native Host tests exercise retained editable
children. These local checks do not close physical/release acceptance.

The [2026-10-08 public macOS walkthrough](../evidence/notification-gallery-och41.md)
now covers 34 settled anchor/theme/motion/inset cases, collapsed semantic shielding,
keyboard expansion and dismissal, retained identity and page teardown. It fixes
the sample application's Show-during-exit race with fresh batch keys and stale
dismissal guards. Smoothness, VoiceOver, IME, hover expiry, reduced motion and
resource acceptance remain separate; this is not full-family qualification.

Layered geometry, interrupted reflow, finite lifecycle and accepted-dismissal
tokens now have [native foundation evidence](../evidence/toast-presentation-foundation-och41.md).
Those models now connect to the production Host, Core API and public gallery;
see [motion integration evidence](../evidence/toast-motion-och41.md). The remaining
qualification is physical macOS and release validation.

## Accepted presentation constraints

The expanded, immediate-removal default is preserved. Checked optional Layering
and Motion use stable node generations for measurement and animation. Collapsed
visibility, accessible content, keyboard traversal and expansion follow the
[presentation contract](../design/toast-presentation.md); opacity alone never makes
a toast noninteractive.

An animated dismissal needs an explicit noninteractive Ending phase. Specify
when the single typed terminal observation is delivered, how ordinary config
updates affect it, and how application removal/close cancels it. Do not keep a
removed OCaml subtree alive by silently cloning its native owners. Separate
active-time expiry from motion settlement and preserve the existing stale-event
checks, modal policy and composition handling.

Native frame work must stop on settlement, reduced motion, hidden/inactive owner,
unmount and close. The pinned component advances every 50ms while nonempty;
that polling strategy is not a GPUIO requirement. Use the existing bounded native
deadlines/frame scheduling. Native measurement, painting and input remain in Rust.

The linked placement, layering and motion evidence now covers paired codecs/
validation, measured variable-height geometry and interruptions, lifetime ordering,
retained child input, policy settlement, the public gallery and a fresh installed
consumer. Real macOS input/VoiceOver/GPU/resource checks remain under OCH-17.
No physical platform result is claimed by this source review.
