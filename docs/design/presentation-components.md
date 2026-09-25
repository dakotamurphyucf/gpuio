# Presentation, feedback and form components (OCH-33)

Status: semantic metadata, form composition and stateless presentation helpers
are implemented and tested locally. Avatar fallback, loading indicators and rating
remain pending; acceptance is the complete live OCH-33 ticket. No OCH-33 capability
is advertised. See the [evidence ledger](../evidence/presentation-components-och33.md).

## Public organization

Keep semantic data, presentation and application state separate. Public view
helpers remain immutable OCaml descriptions with polymorphic actions; Bonsai
specializations use the same underlying views. Static badges, labels, separators,
groups, descriptions, empty states and chat cards do not acquire controllers.
Explicit content slots accept ordinary views. Size, tone and appearance are typed.
Shared light/dark appearances supply concrete default colors and can accept
application Color tokens; do not add required tokens to existing custom themes.

Reusable helpers cover avatar/fallback, badge/tag/marker, label/link, separator,
group box/grouped settings, description list, empty state, alert/banner, keyboard
shortcut labels, status bar and attachment/message/bubble/tool-result cards.
Extract useful composition from examples/agent_chat/runtime/workspace.ml, retaining
application-specific data and document registration in the application.

## Accessibility foundation and form contract

`Accessibility` holds validated semantic metadata: a constrained role, optional
label/description, live-region policy, or an explicit form-field description.
`View.with_accessibility view metadata` preserves view/native identity and rejects
unsupported role/control combinations. Metadata is admitted with the tree, not
sent through callback objects. Native controls preserve their existing actions,
disabled/read-only state, keyboard handling and generation checks.

`Accessibility.Field.create ~label ?help ?error ?required ()` describes one
control's label/help/error/required state. Empty optional text is represented by
None; strings are bounded UTF-8 without NUL. Field metadata is exclusive of role
overrides. Native semantic labels/help/errors belong to the actual control, not
merely a decorative parent. Required and invalid states and explicit label/
description/error relationships are emitted using AccessKit synthetic text nodes.
The existing GPUIO semantic wrapper can write native AccessKit properties and
synthetic children while preserving the underlying element ID; no GPUI fork is
needed. Application validation remains OCaml state. Updating errors does not
replace or reset an editor's buffer, undo state, focus or IME composition.

The form layout helper displays the same field label/help/error around a supplied
control and applies the metadata directly to it. Initially supported control roots
are native input/textarea/combobox, checkbox/switch, radio/select and rating as it
is implemented. Reject ambiguous arbitrary subtrees instead of guessing which
child owns the field. This contract adds no second form runtime or validation job.
Local macOS checks verify field label/help/error/required output, metadata updates
and reset during active composition, and removal. AccessKit carries invalid and
relationship properties; the pinned macOS adapter does not expose every property
through a corresponding AX attribute. Do not claim a full VoiceOver audit from
these programmatic checks.

`Presentation` exposes Size, Tone, Variant, Axis and an abstract Appearance with
concrete light/dark palettes or caller-supplied colors. Its named content slots
retain stable wrapper keys; adding/removing a header, footer or action does not
change the body control's identity. Action wrappers retain their intrinsic width.
Badge/tag/marker, label/link/separator, group/settings, description list, empty/
alert/banner, shortcut/status and attachment/message/bubble/tool-result helpers
use ordinary views. They accept polymorphic actions, so Bonsai can use them without
a second implementation. `examples/presentation/` exercises these public APIs.

Link presentation uses the existing native button activation path with Link
semantics. Activation calls the application action; URL/file/network I/O remains
an explicit Eio application decision. Keyboard shortcut labels display the
configured shortcut; they never silently install bindings. Static status/alert
semantics have explicit live-region priority and must not announce every frame.

## Stateful behavior still required

- Avatar reuses scoped asset/image decoding and cache ownership. A failed/missing
  image shows an explicit fallback; stale results cannot replace a newer asset.
  Native selection of image/fallback must not require application boilerplate or
  expose duplicate meaningful images. Keep the existing load error observation.
- Skeleton/shimmer/spinner use existing native motion and reduced-motion policy.
  Hidden/unmounted owners stop frame/deadline work. Static reduced-motion output
  remains recognizable and semantically reports loading. No Bonsai polling timer.
- Rating is a bounded whole-number selection (0..maximum) with a default maximum
  of five. Validate rather than silently clamp invalid caller state. Native hover
  preview and keyboard/AX interaction remain responsive and preserve identity.
  Define value/intent ordering before implementation so repeated key events cannot
  be lost behind an OCaml render. Disabled/read-only behavior and clearing to zero
  are explicit. Reuse existing focus/event fences and bounded input mailbox.

## Source and validation

Pinned gpui-kit revision 84f57fdfcb4910623fb0bb7f795b077e249f9271 supplies the
functional catalog. Its rating is whole-number, defaults to five stars, supports
color/size/disable and native hover/click; keyboard/accessibility is a GPUIO
acceptance requirement, not inferred from upstream source. Source references:
[rating](https://github.com/longbridge/gpui-kit/blob/84f57fdfcb4910623fb0bb7f795b077e249f9271/crates/component/src/rating.rs),
[label](https://github.com/longbridge/gpui-kit/blob/84f57fdfcb4910623fb0bb7f795b077e249f9271/crates/component/src/label.rs),
[shimmer](https://github.com/longbridge/gpui-kit/blob/84f57fdfcb4910623fb0bb7f795b077e249f9271/crates/component/src/shimmer.rs).

Required deliverables: typed interfaces and bounded paired codecs where needed;
meaningful validation/reconciliation/event tests; public settings/details/chat-card
example; actual macOS keyboard/AX, long/empty/localized text, image failure, theme/
scale, hidden motion and cleanup acceptance. Retained native state and metadata
count against existing tree/resource budgets. No synchronous OCaml callbacks from
layout/paint. Linux build/unit and consolidated CI remain required, GUI coverage
is reported separately under OCH-17. OCH-46 integrates these families into the
polished chat showcase; it does not replace individual component acceptance.
