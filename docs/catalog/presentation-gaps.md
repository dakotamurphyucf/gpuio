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
| [Alert](sources/component-alert.rs.txt) | Functional equivalent: `Presentation.Alert` adds typed variants/sizes, optional rich title/body/icon, validated close configuration and Card/Banner. Banner suppresses title but retains the source renderer's border. Legacy helpers stay unchanged. | [Contract and evidence](../design/presentation-alerts.md): Core style/semantics/40-transition identity checks; normal and installed-consumer macOS matrices cover 20 theme/variant/banner cases, eight size layouts, wrapping, controls and retirement. Consumer GPU checks cover semantic tints and borders. Live defaults Off; visibility and tasks stay caller-owned. Whole-family/release gates remain open. |
| [Tag](sources/component-tag.rs.txt) | Functional equivalent: `Presentation.Tag` provides rich direct children without a required label, typed semantic/custom palettes, independent outline, size aliases, custom radii and native hover refinement. Legacy tag/badge remain unchanged. | [Contract and evidence](../design/presentation-tags.md): Core palette/style/56-transition identity checks; a fresh installed consumer passes 28 native palette/outline GPU cases, size groups, pointer hover override/unset, 30 OS actions, rich-only/empty content and retirement. Named source color scales map to application-selected palettes/tokens; theme values and Rust builder spelling are not parity requirements. |

## Chat presentation

[Attachment](sources/component-attachment.rs.txt) includes
Pending/Uploading/Processing/Failed/Complete, horizontal/vertical layouts, sizes,
media/content/action slots and optional card activation requiring an id. Root
status reaches typed title/description unless explicitly overridden. Uploading
and processing titles shimmer; failed descriptions use destructive color. Pending
borders are dashed; images dim during uploading, processing or failure. Vertical
actions sit at the top right. The action cluster stops mouse-down propagation
even in gaps, preventing card activation. AttachmentGroup scrolls horizontally.

The original horizontal preview/name/detail/actions helper keeps its existing
behavior. The new [typed Attachment adapter](../design/presentation-attachments.md)
implements status, rich slots, horizontal/vertical sizing, dashed borders, native
title shimmer and independent action/trigger controls. Eio upload/file work stays
application-owned. Core and focused public macOS checks now cover status/layout
transitions, retained identities, action-gap shielding, keyboard and pointer
activation, image dimensions and page teardown. Installed-consumer GPU, motion
and real scroll checks now complete the scoped local Attachment functional-
equivalence review; composed Link's passive-content constraint is unchanged.
The shared [text-shimmer foundation](../design/text-shimmer.md) now has paired,
validated standalone/live codecs, Core/Bonsai reconciliation and atomic native
tree admission, a native glyph painter and independent retained-clock GPU/lifecycle
checks. Mounted Host/selection/highlight rendering now has focused GPU/source
AX/lifecycle evidence, including retained tabs, native responsive branches and
managed-row pause/eviction/remount. Public gallery and fresh installed-consumer
checks now pass both themes, widths and directions, actual keyboard Copy/effect
toggling, stable native identity, playback/reduced motion and page remount. A
visible-layout clock defect has a failing-before regression and passing repair.
The [Marker integration](../design/presentation-markers.md) now has scoped native
and installed-consumer evidence. Aggregate-performance acceptance remains pending.
The shared per-window shimmer budget has native limit/fallback/recovery evidence;
that enforces work ceilings without establishing application frame-time budgets.
Attachment's [acceptance record](../design/presentation-attachments.md) now covers
action gaps, keyboard/AX activation, layout/state changes, image failure, native
reduced motion, page teardown and image/source release. Application task
cancellation stays caller-owned; aggregate performance/resource acceptance is
still a release gate.

[Bubble](sources/component-bubble.rs.txt) and
[Message](sources/component-message.rs.txt) now have locally validated functional
equivalents in `Presentation.Bubble` and `Presentation.Message`, retaining the
legacy helpers. The [contract and acceptance](../design/chat-composition.md)
records seven surfaces, independent root/content alignment and styles, typed
reactions, optional message slots, Ghost inset metadata and caller-owned resources.
Root and explicitly aligned surface bounds are distinct; reactions anchor to the
root, and callers reserve measured space for their out-of-flow controls.

Core tests, the public macOS gallery and a fresh installed consumer cover 28
layout cases, fourteen GPU surfaces, 72 control actions, editor identity/focus,
streaming, clipping, slot/page retirement and zero source bytes after departure.
A 100-message managed preview covers bounded rows, monotonic streamed growth,
stationary draft, paused-history updates, reaction spacing, sixteen one-pixel OS
wheel events and warm identity. It exposed and helped repair a native list/parent
double-scroll defect, with failing-before native and passing native/public tests.
The broader native list regression still passes selection/editor lifetimes and
two 100,000-row traversals. Full accessibility, performance and release gates
remain open; this does not establish Linux GUI or clean-machine distribution.

## Structured descriptions

[DescriptionList](sources/component-description-list.rs.txt) has rich labels and
values, horizontal/vertical cells, label width, size/border, multiple columns,
spans and full-row separators. Upstream clamps columns to 1..10 but accepts
unchecked spans; oversized/zero spans are not a behavior to reproduce. Rows pack
by accumulated spans; border rendering actually applies in either axis.

`Presentation.Description_list` now supplies validated columns/spans, rich
term/value slots, fixed/percentage label widths, both axes, four sizes, borders
and separators. Direct keyed cells preserve native owners through regrouping;
invalid counts/spans and duplicate keys are rejected. The legacy helper remains.
The [contract and evidence](../design/description-lists.md) records four Core tests,
75 native layout/reading-order cases, 34 GPU theme/size/border cases, OS actions,
draft/control retention and slot/page retirement in a fresh installed consumer.
This source row is locally a functional equivalent; broader release gates remain.

## Keyboard labels

[Kbd](sources/component-kbd.rs.txt) formats typed keystrokes with platform-specific
symbols, optional plain appearance and styled keycaps. Its action/context/focus
lookup displays only the first stroke of a binding. Formatting does not register
a shortcut. GPUIO's `shortcut_label` currently accepts strings; `Shortcut` validates
single chords and `Command.Registry` handles declaration shadowing. Native queries
are provided separately through `Command_binding`.

Typed display now exists in `Shortcut.format` / `accessible_label` and
`Presentation.Kbd`; `Command.shortcuts` exposes ordered declarations. The
[contract](../design/keyboard-labels.md) records platform aliases, Unicode mapping,
filled/outline/plain styles and the distinction between display and registration.
Core/full Dune, the focused macOS native gallery and a fresh installed consumer
pass. Native context/focus lookup now has an asynchronous mounted observer,
Core callback and Bonsai value adapter, with configuration/epoch fencing and a
separate bounded latest-value mailbox class. `Presentation.Kbd.of_native_stroke`
displays the native domain without expanding registration. The
[observation contract](../design/command-binding-observations.md) records shared
resolution, widget keymap lookup and scoped native/lifecycle evidence. The
public gallery and installed-consumer examples pass 20 scoped live-binding cases,
OS Copy and command invocation, identity and retirement. Expanded native cases
also pass nested ID/chord shadowing, cross-phase precedence, modal focus/restoration,
sparse retained-row suspension/eviction, capacity/recovery and independent-window
close/generation reuse. The source row is a **local functional equivalent** within
the [typed action/focus contract](../design/keyboard-labels.md#native-binding-observations);
consolidated catalog and release gates remain separate.

## Loading markers

[Marker](sources/component-marker.rs.txt) includes Plain/Separator/Border,
styleable icon/content/separators, role/id and Spinner/Shimmer. Role overrides
require an id; default semantics are presentational. Spinner inserts a fallback
only without a typed icon. Typed text shimmers; arbitrary elements in mixed content
remain unchanged. If content has no typed text, the whole rich content instead
pulses opacity from 0.6 to 1.0 times its base opacity. Reduced motion disables it.

`Presentation.Marker` now supplies this composition, preserving the original
dot/label helper. Typed text uses shared native glyph shimmer; rich-only content
uses the native opacity factor on a stable animation root. The [contract and
acceptance](../design/presentation-markers.md) document exact source mappings,
key/UTF-8 validation, explicit semantics, smooth-pulse easing, native owner limits
and native/installed-consumer matrices. The source row is locally validated as a
functional equivalent; this does not complete the presentation family or release.

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

The [Core model and full composition contract](../design/settings-composition.md)
now implement distinct IDs, bounded metadata/search, preferred/effective selection,
expansion reconciliation and explicit reset scopes with latest-state/disabled
guards. Five Core expect tests pass. The controlled Bonsai composite now composes
sidebar/split/managed groups, responsive stable field placements, rich page slots
and reset requests. Three further expect tests check page/row lifetimes,
reconciliation identity, namespace/disabled semantics and localized-label errors.
Typed Boolean/choice/native-field helpers also pass identity/semantics checks;
public editor examples pass native mount-seed behavior and guarded resets.
Numeric draft/value seeds now support atomic unfinished-draft recovery on remount.
A dedicated public gallery now passes scoped native editing, responsive identity,
search/page draft recovery and export failure/cancellation checks. Complete
reset/field edge coverage and full native acceptance remain unfinished. Group
reset controls, unmounted-editor reset lifetimes, export validation and real
native Save/Eio readback now pass scoped repository and fresh installed-consumer checks. A fresh
installed consumer also passes the scoped checks, long-choice keyboard selection
and Unicode paste; this checkpoint does not establish equivalence.

## Delivery order

Composed Link, shared dashed borders and Empty/Separator composition now have
scoped native and installed-consumer evidence, as do shared text shimmer and the
rich Attachment/Marker, Alert, Tag, Bubble/Message and Description adapters.
Typed Kbd display and native binding observations also have scoped native and
installed-consumer evidence. Settings composition remains in this presentation
review.
Each family needs public gallery examples and meaningful native acceptance before
its ledger status changes. Reuse ordinary views where possible; introduce native
state or protocol operations only for concrete missing behavior.

This review does not close the other catalog families, fresh installed consumers,
required CI or OCH-17 performance/accessibility/distribution/release gates. Linux
desktop qualification remains deferred to OCH-47; required non-GUI Linux checks
are unchanged.
