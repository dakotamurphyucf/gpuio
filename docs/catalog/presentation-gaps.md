# Remaining presentation contracts — OCH-41

Source review at GPUIO `f8c2a22`, gpui-kit
`84f57fdfcb4910623fb0bb7f795b077e249f9271`. The initial review identified gaps and
proposed implementation directions; source inspection alone was not acceptance.
Rows below explicitly record subsequent local acceptance where evidence exists.
All remaining gaps stay open. Existing OCH-33 tests remain evidence only for
their actual public API and behavior.

The linked snapshots were checked byte-for-byte against the manifest-pinned
archive after verifying its SHA256. All 21 files, including the eleven settings
implementation files, are recorded in [the source manifest](sources/manifest.json)
and covered by [the upstream license](sources/gpui-kit-LICENSE). Existing public
interfaces are in [Presentation](../../lib/core/presentation.mli).

## Stateless layout and slots

| Family/source | Reviewed behavior and current gap | Implementation and acceptance direction |
| --- | --- | --- |
| [Separator](sources/component-separator.rs.txt) | Functional equivalent: `Presentation.Separator.create` composes both axes, optional label/color, solid/dashed patterns and independent root/line/label styles. The original single-rectangle helper remains compatible. | [The design and evidence](../design/presentation-separators.md) explicitly use pinned GPUI border dash spacing rather than the source path's exact 4px/2px pattern. Core tests and an installed-library macOS consumer pass 32 layout/identity cases and twelve long-label clipping/reset cases with native paint sampling. No per-dash OCaml children. Required Linux/whole-release checks remain separate. |
| [Empty](sources/component-empty.rs.txt) | Functional equivalent: optional rich slots, media variants, default/custom border styles and proportional description spacing through `Presentation.Empty_state`; the string convenience helper remains unchanged. | Four Core tests and an installed-library macOS consumer pass 16 layout, ten border and twelve typography cases with keyboard/focus/identity and scoped cleanup. See the [source mapping and evidence](presentation-review.md#empty-state-rich-slots). Whole-family/release and required Linux checks remain separate. |
| [Alert](sources/component-alert.rs.txt) | Variants/sizes, optional title, rich body, icon, close action and banner mode. Banner suppresses title. Despite an upstream comment, actual root rendering still applies a border. | Keep existing helper's behavior; add optional rich header and slot styling where needed. Visibility and close remain application state plus an ordinary accessible button. Test title/banner policy, action retirement and explicit live-region behavior. Do not announce every cosmetic update. |
| [Tag](sources/component-tag.rs.txt) | Theme/custom colors, outline, radius, arbitrary children, size and hover. Existing tones/variants/root style/leading/trailing slots cover much of the composition. | Prove custom-style and rich-content use cases; add a content-only constructor only if mandatory label placement blocks them. Test style replacement and interactive child identity. Theme token values and Rust builder spelling are not parity requirements. |

## Chat presentation

[Attachment](sources/component-attachment.rs.txt) includes
Pending/Uploading/Processing/Failed/Complete, horizontal/vertical layouts, sizes,
media/content/action slots and optional card activation requiring an id. Root
status reaches typed title/description unless explicitly overridden. Uploading
and processing titles shimmer; failed descriptions use destructive color. Pending
borders are dashed; images dim during uploading, processing or failure. Vertical
actions sit at the top right. The action cluster stops mouse-down propagation
even in gaps, preventing card activation. AttachmentGroup scrolls horizontally.

The current horizontal preview/name/detail/actions helper lacks those behaviors.
Use typed status and rich slots, with Eio upload/file work owned by the application.
Share dashed borders and text shimmer with separator/marker work. Card activation
needs intentional event/focus/AX semantics alongside independent actions; do not
relax composed Link's passive-content constraint to admit nested controls.
The shared [text-shimmer foundation](../design/text-shimmer.md) now has paired,
validated standalone/live codecs, Core/Bonsai reconciliation and atomic native
tree admission, a native glyph painter and independent retained-clock GPU/lifecycle
checks. Mounted Host/painter and actual keyed-node lifetime integration remain
pending; this does not complete Attachment or Marker behavior.
Native acceptance must cover action gaps, keyboard/AX activation, layout/state
changes, image failure, reduced motion, cancellation/unmount and bounded resources.

[Bubble](sources/component-bubble.rs.txt) provides
Filled/Secondary/Muted/Tinted/Outline/Ghost/Destructive surfaces, optional start/end
alignment, separate content/reaction slots and groups. Ordinary bubbles default
to 80% maximum width; Ghost uses full width with no surface padding/border/radius.
Reactions attach outside top/bottom using a fixed 1.25rem offset, with start/end
placement. Typed buttons get rounded treatment and suppress decorative wrapper
padding; arbitrary elements retain their styles. The current single-column helper
needs variant and reaction composition. Test reaction hit areas, keyboard/AX,
ancestor clipping and managed-row measurement, particularly while streaming.

[Message](sources/component-message.rs.txt) supports independently styled optional
avatar/header/content/footer, root/stack styling and start/end alignment. The
avatar bottom-aligns with the header/content row; footer is outside that row with
an avatar-column inset. End reverses the row. Typed bubble insertion records Ghost
metadata to choose inherited header/footer inset defaults; arbitrary children do
not. Content alignment aligns its column's children; it does not rewrite each
Bubble's own alignment property. The current required-author helper puts avatar
in its header and cannot express this geometry by root style alone.

Draft a rich message composition contract while retaining the simple helper.
Explicit slot/inset overrides must remain predictable. Test absent slots, mixed
bubbles, large Markdown/code, footer growth, selection and streaming in managed
rows. Native input ownership and row identity must survive these layout updates;
the existing scroll-jitter fixes are required regressions.

## Structured descriptions

[DescriptionList](sources/component-description-list.rs.txt) has rich labels and
values, horizontal/vertical cells, label width, size/border, multiple columns,
spans and full-row separators. Upstream clamps columns to 1..10 but accepts
unchecked spans; oversized/zero spans are not a behavior to reproduce. Rows pack
by accumulated spans; border rendering actually applies in either axis.

Current `Description` accepts a string term and View definition, with a single
column of row/stacked entries. Design validated columns/spans and rich terms.
Specify reject/normalization policy before implementation. Verify packing against
independent examples and actual wrapped geometry/AX term-definition reading order.
Changing row parents can remount stateful entries despite stable leaf keys; choose
a structure that preserves the promised identity and test it through regrouping.

## Keyboard labels

[Kbd](sources/component-kbd.rs.txt) formats typed keystrokes with platform-specific
symbols, optional plain appearance and styled keycaps. Its action/context/focus
lookup displays only the first stroke of a binding. Formatting does not register
a shortcut. GPUIO's `shortcut_label` currently accepts strings; `Shortcut` validates
single chords and `Command.Registry` handles declaration shadowing, but neither
offers an effective-focused-binding query.

Provide typed platform-aware display without implicitly adding multi-stroke input.
Declared registry shortcuts and effective native focused bindings are distinct.
If a native lookup is needed, use asynchronous observations with focus/registry
revision fencing, never a synchronous OCaml callback. Test both Mac/Linux formatting,
Primary resolution, Unicode/named/function keys, disabled/shadowed bindings and
real invocation. Any lookup needs explicit missing/stale result behavior.

## Loading markers

[Marker](sources/component-marker.rs.txt) includes Plain/Separator/Border,
styleable icon/content/separators, role/id and Spinner/Shimmer. Role overrides
require an id; default semantics are presentational. Spinner inserts a fallback
only without a typed icon. Typed text shimmers; arbitrary elements in mixed content
remain unchanged. If content has no typed text, the whole rich content instead
pulses opacity from 0.6 to 1.0 times its base opacity. Reduced motion disables it.

The current dot/label helper and rectangle `Loading.Kind.Shimmer` do not provide
glyph shimmer. Add explicit rich marker slots/loading behavior and evaluate a
small shared native text-shimmer primitive for Marker and AttachmentTitle. Existing
animation programs may cover the rich-content opacity fallback. Preserve AX text
and explicit status announcements. Test mixed/text-only/rich-only content, role/id,
icon override, reduced motion and stopping/cleanup under completion and unmount.

## Settings application composition

The [settings module](sources/component-setting-mod.rs.txt) exports a larger
composite than `Presentation.settings_group`: searchable pages/groups/items,
resizable sidebar, selected page/group, virtualized groups, responsive field
layout, reset actions and custom fields. See
[Settings](sources/component-setting-settings.rs.txt),
[Page](sources/component-setting-page.rs.txt),
[Group](sources/component-setting-group.rs.txt),
[Item](sources/component-setting-item.rs.txt) and
[fields](sources/component-setting-fields-mod.rs.txt). Source defaults include a
250px sidebar with 160..360px range and a 480px content-layout breakpoint.

Typed fields cover bool, string, number with min/max/step, dropdown/scrollable
dropdown and arbitrary elements/renderers. Rust get/set closures are not suitable
cross-FFI callbacks. Use existing native controls and Bonsai-owned typed state;
application persistence and schema migration use Eio outside view construction.

Source nuances inform the contract review:

- Search clones pages with matching items/groups. Selection indexes the filtered
  vector; an absent index renders empty. Do not copy positional selection as a
  stable-identity guarantee.
- Reset-all walks the rendered page's groups; the rendered page is a filtered
  clone, so filtering scopes this path. GPUIO must explicitly choose and document
  reset scope, disabled-field policy and selection fallback.
- Custom reset overrides typed default-value comparison/reset. Arbitrary element
  fields without custom reset are not resettable.

Use stable page/group/item IDs and existing sidebar/split/container-query/managed
view APIs. Add a working composite/example before claiming equivalent settings
functionality. Test filtering/empty results, selected-page disappearance/recovery,
reset scopes and disabled/custom fields, in-flight edits, IME/input identity,
responsive resizing, focus reveal and application persistence failures.

## Delivery order

Composed Link, shared dashed borders and Empty/Separator composition now have
scoped native and installed-consumer evidence. Remaining work includes
text-shimmer contracts, rich attachment/chat slots, description layout/identity,
typed keyboard labels and marker/settings composition.
Each family needs public gallery examples and meaningful native acceptance before
its ledger status changes. Reuse ordinary views where possible; introduce native
state or protocol operations only for concrete missing behavior.

This review does not close the other catalog families, fresh installed consumers,
required CI or OCH-17 performance/accessibility/distribution/release gates. Linux
desktop qualification remains deferred to OCH-47; required non-GUI Linux checks
are unchanged.
