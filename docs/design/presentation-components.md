# Presentation, feedback and form components (OCH-33)

Status: semantic metadata, forms, stateless presentation, loading, avatars and
rating are implemented with local native and public-API acceptance. Capability
`17179869184` advertises this family (aggregate `2199023255551`). See the [milestone handoff](../milestone-5.md) for hosted gates and delivery. See the [evidence ledger](../evidence/presentation-components-och33.md).

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

`Presentation.status_bar` accepts leading, center and trailing views. The center
uses the space left between the ends: centered with both ends, end-aligned with
only leading, and start-aligned otherwise. Omitting it preserves the existing
two-region layout. Stable slot keys preserve controls when other regions change.
The [pinned presentation review](../catalog/presentation-review.md) records this
functional equivalent and remaining badge/label differences; the presence of a
GPUIO helper does not imply every upstream configuration is implemented.

Link presentation uses the existing native button activation path with Link
semantics. Activation calls the application action; URL/file/network I/O remains
an explicit Eio application decision. Keyboard shortcut labels display the
configured shortcut; they never silently install bindings. Static status/alert
semantics have explicit live-region priority and must not announce every frame.

## Stateful behavior still required

Loading implementation contract: `Loading.Config.create ~kind ~label ?animated
?period ()` and `View.loading` use one bounded native leaf. Kinds are Skeleton
(pulsing placeholder), Shimmer (sweeping highlight), and Spinner (twelve radial
strokes). The default period is 1200ms, with a validated 100ms..60s range. The leaf
has indeterminate ProgressIndicator semantics and no value, focus target or event
handler. Foreground/size use normal styles. Static/reduced output remains visible.
Hidden native owners never construct animated children; removal drops GPUI's
animation state. The semantic wrapper also marks hidden structural roots and
leaves hidden in AccessKit, so platform accessibility omits their descendants.
Placeholder corners use the existing computed-radius capture at paint time. No OCaml polling, per-frame protocol updates or worker task is
introduced. Whole-window idle and nested-hidden checks must prove this path, not
merely infer it from the underlying animation helper.

- Avatar reuses scoped asset/image decoding and cache ownership. A failed/missing
  image shows an explicit fallback; stale results cannot replace a newer asset.
  Native selection of image/fallback must not require application boilerplate or
  expose duplicate meaningful images. Keep the existing load error observation.

Avatar implementation contract: `Avatar.Fallback.create` accepts explicit initials
or a symbol (nonblank single-line UTF-8, <=128 bytes, no ASCII controls).
`Avatar.Config.create ?asset ?fit ~fallback ~description ()` uses the existing
Image.Description and defaults to Cover. `View.avatar ?on_change config` is one
native leaf with a stable accessible image label, whether pixels or fallback are
shown. Decorative fallback text is painted without creating a semantic text child.
No asset means no reader and no synthetic image error; supplied assets reuse the
existing generation-checked Image.State observations and mounted leases.

Native SetAvatar atomically updates avatar configuration and the derived optional
image binding used by the existing image host. Source replacement drops the old
binding before acquiring the new one; removal/no-source clears it. Raster and SVG
continue using the current cache/worker/rasterization budgets. SVG fallback must
still allow a subsequent size/fit change to request a new raster variant after a
resize failure. A failed variant must not silently show stale pixels as a success.
The default box is 32x32 with circular corners; caller styles override it. Fallback
text is centered and only shrunk to fit, with bounded shaping work and no OCaml
layout callback. Actual GPU/AX tests cover fallback glyphs, circular image clipping,
replacement, retired-source leases, failure/recovery, synthetic density changes
and disposal. See the presentation evidence ledger for the tested platform scope.

Invalid measured SVG sizes are tracked separately from decode/upload errors. A
later valid size clears only the layout error, including when it returns to the
previous successful request. Paint-discovered status changes defer native view
invalidation until after drawing; the usual source/handler-checked event bridge
then reports them. Notification during drawing alone does not reliably invalidate
GPUI's current render. This preserves asynchronous delivery and avoids an idle
polling loop; the native test checks failure/recovery events and static idle.
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

## Rating contract

`Rating.Config.create ~label ~value ?maximum ?star_size ?disabled ?read_only ()`
validates zero through maximum, with maximum 1..32 (default five), 8..128 logical
pixel stars (default 24), and the usual bounded nonblank accessible label. Zero
means unrated. Ordinary foreground styles color the stars; inactive stars use
reduced opacity. The native leaf draws bounded star geometry, owns only transient
hover state and uses the existing focus/visibility/modal gates.

`View.rating ~config ~on_request ()` is application-controlled. Its
`Rating.Request` values are Set, Toggle, Increase and Decrease. Applications apply
`Rating.Config.apply_request` inside their state-machine reducer, in delivery
order. Steps saturate at the boundaries. Clicking the currently selected star
clears; clicking another star selects it. Keyboard Right/Up increases, Left/Down
decreases, Home/Delete/Backspace clears and End selects maximum. Modified keys
are left to application commands. Hover previews the proposed selection without
committing it or sending an OCaml event; exit/hide/disable/read-only resets it.

Relative requests deliberately avoid stale-value arithmetic when repeated key
presses arrive before an OCaml frame is committed. No optimistic native committed
value, acknowledgement queue or second rating model is introduced. Accessibility
reports the committed value, not hover. The public pure reducer ignores requests
made invalid by a changed maximum or disabled/read-only policy. The event bridge
also checks current kind, handler generation, revision and policy. Requests are
discrete mailbox events and are not coalesced; existing bounded overload handling
applies. An application may explicitly reject a request by retaining its model.

The control is one keyboard stop and exposes a labelled Slider with integer range
0..maximum, unit step and increment/decrement/set-value actions. Individual
painted stars are not additional focus or accessibility children. Disabled leaves
traversal; read-only remains focusable and readable but emits no requests. No
continuous timer or frame callback is needed. New wire kind36/op42/event43 are
append-only; the presentation capability covers the complete OCH-33 family.

Required evidence before acceptance: fast repeated-key reducer ordering, pointer
preview/selection/clear, actual AX numeric/actions/read-only, native focus and
ancestor/modal gating, hide/unmount cleanup and idle, paired strict codecs,
reconciliation callbacks, public example, themes and synthetic density checks.

## Constrained content

Text in tag/badge/marker, shortcut and message-header rows may shrink below its
intrinsic width and wrap. The status bar leading content also flexes. Action and
icon slots retain their intrinsic space; the caller controls the content of
these slots and can supply a wrapping action group where necessary. Form rows
support vertical and explicit 160px-label horizontal layouts; choose vertical
when the containing panel cannot accommodate that label plus its control.

The public `--content-check` fixture and macOS AX walkthrough exercise 19 families
with long, empty and localized text in 440px/800px windows under both appearances.
This is constrained-layout coverage, not a claim of automatic translation or
full bidirectional/RTL layout. See the evidence ledger for actual scale coverage.
