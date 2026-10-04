# Rich tabs — OCH-41 design and source review

Status: in progress. Decorative labels, static native target variants/styles,
interactive tab parts with bounded width, native viewport/reveal and fixed frame/
trailing content are implemented. The all-tabs menu now has full-name rows and
native choice behavior and decorative icons. Optional native indicator springs
and Pill selected-foreground fading are implemented with local native evidence;
see the final section. Physical desktop qualification remains open. Earlier
checkpoint sections below describe the staged implementation history.

## Pinned behavior

Sources are the pinned `component-tab-tab.rs.txt`, `component-tab-tab_bar.rs.txt`
and `base-tabs.rs.txt` snapshots in `docs/catalog/sources`, with provenance in
that directory's manifest. The styled layer defines **Tab (default), Outline,
Pill, Segmented and Underline** variants. The base supplies pointer/semantic tab
behavior; GPUIO already adds compound keyboard navigation and application-owned
selection through `Choice.Config`.

Each styled tab accepts a label, a separately supplied accessible label, icon,
prefix, suffix, arbitrary children, disabled/selected state and styles. A suffix
can be a close button. The maximum whole-tab width truncates the label while preserving
prefix/suffix dimensions; icon-only tabs are exempt. Bar-level prefix/suffix and
trailing space have separate layout roles. Application callbacks own activation.

The bar's `menu(true)` implementation renders an all-tab menu whenever enabled,
although its method comment describes overflow. The menu preserves complete
labels/icons, selected checks and disabled options. Do not infer an overflow-only
visibility threshold from that comment. `track_scroll` explicitly does **not**
reveal a selected tab automatically; callers request reveal on the tracked handle.
The styled bar also supplies measured sliding indicators for Pill, Segmented and
Underline, synchronized selected-text color transitions and retained animation
state. Static colors alone do not establish equivalence to those features.

## Current compatible foundation

`View.tab_bar` stays the simple controlled interface. The new
`View.tab_bar_with_labels` takes the same `Choice.Config` and callback plus a
partial mapping from `Choice.Id` to decorative view content. Names, disabled
state and stable identity remain in the config. Omitted labels fall back to the
configured string. Core rejects unknown/duplicate IDs, interactive content and
unbounded trees. One keyed Container slot per option gives native measurement
and descendant resources a stable owner; slots are either empty or have one
validated label subtree. Native admission rechecks child-only edits atomically.
The native option remains the only semantic/action owner and renders each label
exactly once. No new wire tag, mutable selection owner or synchronous callback is
needed for this foundation.

Decorative content allows icons, badges and formatted labels. It deliberately
cannot contain a close button; that belongs to the interactive-part work below.
It does not reinterpret label styles as styles for the native tab's hit target.

## Required richer interface and ownership

Draft the final typed interfaces before each implementation stage. Preserve the
simple constructors and the existing `Choice.Id`/config selection contract:

- A checked presentation value should describe the five variants, tab size,
  optional maximum tab width, native indicator motion and theme-aware styles.
  Per-tab styles apply to the actual tab target, including selected/disabled and
  pointer/focus states; label styles apply to label content. Avoid two overlapping
  style owners for the same measured geometry.
- Typed content slots must distinguish decorative label/icon content from
  independently interactive prefix/suffix controls. Keep those controls' normal
  focus, command, tooltip and callback lifetime. Activating a close control must
  not additionally select its tab. Tab-list arrow routing must not take keys from
  a focused descendant control or editor. Explicit accessible names remain
  mandatory for icon-only or customized content.
- Bar prefix/suffix and trailing space need explicit structural relationships.
  They must not alter logical tab indices or become part of the scroll-reveal
  indexing. Application-owned adjacent controls can remain ordinary composition
  where that preserves the source behavior.
- A bounded native overflow menu must use the current configured IDs, full names,
  disabled state and accepted selection. Reorder/removal and stale callbacks must
  not choose an obsolete item. Its public selection event stays the same typed
  request as pointer/keyboard activation.
- Native scrolling owns the actual offset. Expose generation-checked explicit
  reveal of an item ID rather than a Rust `ScrollHandle` or pointer. Programmatic
  selection alone need not scroll. Keyboard navigation should keep its active
  target reachable while preserving the distinction between focus and selection.
- Indicators animate from accepted painted geometry, retain identity by stable
  item ID and respond to resize/reorder/removal without stale-index jumps. Use
  existing native motion/deadline infrastructure, reduced-motion handling and
  teardown rules. Do not stream per-frame geometry into OCaml.

The root controls one compound tab-list focus owner. Native part controls may
have their own ordinary focus stops; full keyboard/AX relationships must be tested
rather than inferred from visual composition. Native renderer state may retain
focus/scroll/animation, but application state remains the sole selection owner.

## Acceptance still required

Core validation and expect tests; paired admission/codec checks for any new wire
settings; actual native layout/hit-testing, keyboard and AX regression tests;
public gallery variants, close actions and narrow overflow cases; installed-package
consumer builds; resource/idle tests and physical macOS keyboard/VoiceOver/GPU
qualification. Reordering, theme/style updates, removing the selected tab, disabled
choices, teardown during animation and stale menu/reveal commands need explicit
evidence. Linux automation remains required; graphical desktop qualification is
tracked separately under OCH-47.

## Native target appearance checkpoint

`Tab_bar.Variant` names Tab, Outline, Pill, Segmented and Underline.
`Tab_bar.Appearance.create` validates target height (16..256 logical pixels),
inter-tab gap and horizontal padding (0..128), shared target styles and up to 128
unique Choice-ID overrides. At most 256 declarations are admitted across all
parts; even empty raw wire blocks consume work quota. Absent override IDs are
ignored so filtering or reordering can reuse the same presentation value.

Both simple and rich-label constructors accept `?appearance`. Omission preserves
the legacy native styling; removing a previously supplied value sends an explicit
reset. The appearance is independent of selected ID and active keyboard choice.
Theme references resolve on the OCaml side and only changed resolved values cross
the bridge. Updates neither replace label owners nor reset native focus.

The native renderer supplies static shape/selection and enabled hover feedback for
all five variants using neutral foreground-derived tints. Explicit root styles
win over bar defaults. Shared and per-ID styles apply to the actual tab target;
per-ID declarations override shared declarations within each state. Base, Focused,
Hovered, Pressed, Selected and Disabled are supported. Box dimensions, padding,
margin, flex sizing/alignment, borders, color, opacity, shadows, text and cursor
presentation are permitted. Visibility, scrolling, positioning and input policy
remain outside these styles. Selection and disabled eligibility still come from
the choice config; target styles cannot create callbacks or extra focus stops.

This is GPUIO theme-aware presentation, not a dependency on the styled layer's
internal theme or exact pixel defaults. Height can be overridden by explicit box
styles. Label styles retain their separate ownership and may override inherited
text colors. The structured-content checkpoint below adds independently retained
prefix/suffix and maximum whole-tab width. Overflow/reveal and animated indicators
remain required work.

Validation for the static target checkpoint: [tab appearance evidence](../evidence/tab-appearance-och41.md).
Physical acceptance and the remaining overflow/motion scope are open.


## Structured content checkpoint

`View.Tab_content.Label` distinguishes `Default` configured text, `Custom` passive
content and `Hidden` visible label. `Tab_content.create ?prefix ?label ?suffix ()`
checks decorative content, and `View.tab_bar_with_content` accepts partial Choice-ID
content overrides. Unspecified IDs keep their configured label. Both Core and Bonsai
expose the API. Unknown/duplicate IDs, blank configured names and unbounded combined
content are rejected. Choice IDs key one structural slot and three fixed part slots,
so reorder and changed labels/presentation preserve descendant native owners.

Prefix/suffix are independently interactive hit areas: clicking them does not select
the surrounding tab. Ordinary callbacks, editors and focus remain with their views;
only the root tab-list focus receives compound arrow/Home/End keys. The application
still owns selection. Choice disabled flags disable selection while independent
controls remain available (for example closing an unselectable tab). Explicit
ancestor `Disabled`/`Inert` applies to every descendant as usual.

`?max_width` caps the whole native tab, matching the pinned source. The finite
1..1e6 logical-pixel bound excludes Hidden/icon-only tabs. Default text ellipsizes;
custom content clips within the remaining label space. Prefix/suffix do not shrink.
A cap smaller than their combined dimensions can clip controls, and padding or
explicit minimum sizing can impose a larger native box. Fully clipped controls
are excluded from ordinary Tab traversal; partially visible controls remain eligible.
Internal native clipping participates in the host focus paint path. This feature
adds no timer or per-frame bridge data.

The Navigation gallery's independent-controls preview supports closing, restoring,
reordering and truncating long tab names. Overflow menus, explicit reveal, native
indicator/color animation and physical macOS qualification remain open. See
[structured-content evidence](../evidence/tab-content-och41.md).

## Native viewport and reveal contract

`Tab_bar.Viewport` opts any of the three tab constructors into horizontal native
scrolling. The viewport owns direction, no-wrap and overflow, including dynamic
root style states; tab targets do not shrink to avoid overflow. Root dimensions,
colors and other presentation remain available. Adjacent bar controls remain
outside this viewport when composed in an ordinary row.

`Tab_bar.Reveal_request.create choice_id ~serial` creates a positive, increasing,
one-shot request passed to `Viewport.create ?reveal ()`. It reveals the actual
measured target by the least movement, without changing focus or selection.
Programmatic selection alone leaves the user's offset unchanged. Native arrow/
Home/End and assistive focus reveal their active target without waiting for an
OCaml selection echo. Independently focused child controls use the common native
focus/scroll path.

Serials are consumed once per viewport lifetime. Missing IDs are consumed and
cancelled; reusing or lowering a serial cannot reveal a newly occupying index.
Withdrawing an unexecuted application request cancels it. Hidden or zero-size
viewports wait for usable layout without polling. Retained hidden/unvisited nodes
keep request history and offset. Before layout, the adapter saves the last painted
offset so GPUI's hidden/zero-size clamping cannot erase it; usable paint restores
that offset once before applying any pending reveal. Withdrawing the viewport itself retires them;
a new viewport lifetime can execute a retained request again. Normal NodeId
generations reject requests for retired placements.

Native measurement uses logical choice order, independent of helper elements.
Current layout is recorded after transaction updates; reordering and resizing
therefore cannot reuse old option indices. An exact-sized bounds array and pending
ID are covered by native retained-memory reservation. Scrolling owns no animation
clock. A changed post-paint offset schedules one follow-up frame because GPUI
intentionally ignores `refresh()` inside paint; settled requests schedule none.
These are asynchronous requests, not acknowledgements of final screen geometry.

The production-host regressions pass on TestPlatform, including native wheel,
keyboard/assistive reveal, offscreen child editor focus, retained drafts, reorder,
resize, hidden/zero-size waiting, dynamic styles, request lifetime and idle cleanup.
These do not qualify physical keyboard, VoiceOver or GPU behavior. The all-tab
menu, bar relationships and indicator/color animation remain required stages.
See [viewport evidence](../evidence/tab-viewport-och41.md).

## Tab frame contract

`View.tab_bar_frame ?key ?style ?menu ?prefix ?suffix ?trailing tabs` is a checked
Core/Bonsai wrapper for any direct, unframed tab bar. Prefix and suffix are
independent ordinary views outside the horizontal viewport. Trailing content is
inside the scroller after the logical options; its default is 12 pixels of space
when a suffix or menu exists, empty otherwise. Explicit trailing content is always kept
and may be interactive. None of these slots becomes a tab or a reveal index.

The outer row uses the supplied style, and defaults to the input tab bar's key.
The tab viewport retains its original styles with grow/shrink/min-width defaults;
bound the frame's width to produce overflow. An existing viewport/reveal config
is preserved; an absent viewport opts into `Tab_bar.Viewport.default`. Keep the
frame mounted to retain the same native descendants. Stable structural keys
separate internal slots from user Choice IDs. Changing a prefix/suffix/trailing
view or switching plain/decorated/structured tab presentation does not replace
unaffected owners. The checked wrapper bounds the combined view to 4096 nodes
and 128 levels.

Empty prefix/suffix slots are hidden from layout, so a frame gap is charged only
between present elements. Their structural identities still remain stable.

Op101 declares the final structural trailing slot. Native admission excludes its
ordinary descendants from label passivity while validating its structure and
bounding the complete tab subtree. Rendering iterates only logical options and
renders trailing content once afterward. Native focus reveal uses the actual
outer box of simple controls, including borders, rather than the absolute marker
child’s padding box. No timer or per-frame bridge observations are introduced.

Core, protocol/admission, production-host tests and a fresh installed-gallery
consumer build pass; physical qualification remains open. This frame
is the layout foundation for the all-tabs menu and does not complete the menu or
indicator/color motion requirements. See [frame evidence](../evidence/tab-frame-och41.md).


## All-tabs menu contract

`Tab_bar.Menu.create ?label ?style ?appearance ()` configures an optional fixed
menu trigger via `View.tab_bar_frame ~menu`. It appears before the fixed suffix,
regardless of whether tabs currently overflow. The default accessible trigger
name is "All tabs"; its visible content is a caret. Frame structural keys preserve
the viewport, prefix and suffix when the menu is inserted or removed.

The menu projects the current tab `Choice.Config`: order, full configured names,
selected check, individual disabled flags and whole-control availability. Selection
uses the same callback and stable Choice IDs, including whitespace IDs accepted
by Choice. No conversion through command IDs or array indices occurs. Changing
selection alone does not reveal the native tab viewport. The application can
separately submit `Reveal_request` when desired.

Rust shares bounded popup rendering, typeahead, highlight/reveal and immediate
input with Select, using an explicit menu presentation. The trigger has Button
semantics with expanded state; the surface is Menu and items are MenuItemRadio
with toggled state. The trigger retains keyboard focus, rows use the active
descendant and do not add Tab stops. Arrow/Home/End skip disabled entries; typing
uses current names; Enter/Space chooses; Escape, Tab, outside pointer or loss of
availability closes. Highlight alone does not select. Popup dimensions and styles
use the checked `Choice.Appearance`; only visible uniform rows are rendered.

Op102 `Set_choice_menu` is an internal Select presentation flag, not a new OCaml
control family. Native admission is atomic and Select-only. Event routes retain
node/handler generations and validate against current choices; OCaml dispatch
also checks the current model. Reorder/relabel keeps native owners. Removal
retires the popup and old callback; no permanent timer or synchronous OCaml
layout/paint callback is added.

Full text names remain visible even for icon-only/hidden tab labels.
`Tab_bar.Menu.create ~icons` adds explicit `(Choice.Id.t * Icon.Decoration.t)`
entries beside those names. IDs must be unique and present in the current tab
choices; the map is bounded by the choice limit and the complete frame's content
budget. Icon styles use the passive-label restrictions. Arbitrary interactive tab
descendants are never copied into the menu.

Menu slots follow Choice IDs through reorder. Replacing an asset replaces only
that mounted image binding. Removing an icon releases its reader; clearing the
map removes the slot vector, and re-adding creates fresh placements. Closing the
popup retains accepted readers, so releasing a registration does not invalidate
already mounted icons. Window close or menu removal releases those readers.
Native row factories create fresh passive elements only for visible rows, with
current choice/child identity checks and a weak owner. This avoids sharing
one-shot native elements across virtualizer calls or invoking OCaml from layout.
See [menu evidence](../evidence/tab-menu-och41.md) and
[icon evidence](../evidence/tab-menu-icons-och41.md).


## Native indicator and foreground motion

All three tab constructors accept `?motion:Tab_bar.Motion.t`; omission preserves
static presentation. `Motion.default` enables interruption-preserving native
indicator springs for Pill/Segmented/Underline and a 200 ms Pill selected-text
fade. Selection and scroll reveal remain separate. The gallery's closable tabs
expose an explicit animation toggle. See the [motion contract](tab-motion.md) and
[local evidence](../evidence/tab-motion-och41.md); physical qualification remains
part of OCH-17.
