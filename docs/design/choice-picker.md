# Choice picker extension direction

OCH-41 design, 2026-10-01. **Validated Core data types are implemented; the
standalone configuration/event codecs, native transition state and a grouped
row/GPUI list-state adapter also exist.
Atomic native tree admission, accepted-tree ownership and an initial native popup
renderer are implemented. Public View/reconciler, Bonsai alias and Eio query
controller now compile with a gallery example. Native input/accessibility/culling
and installed-consumer acceptance remain incomplete.**
The [pinned source review](../catalog/choice-review.md) identifies required choice
functionality beyond the current single-ID Select/editable Combobox. This draft
makes the next implementation concrete. Kind53 and Op72 extend the current
unpublished paired epoch 3; no capability bit is reused.

## Additive control and ownership

Add a trigger-and-popup choice control under a distinct `Choice_picker` domain,
with Core value/configuration types, a pure View, and an Eio/Bonsai controller for
query commands. Keep existing `Choice`, `View.select`, radio and editable Combobox
contracts compatible. Reuse stable `Choice.Id`, text-input leases, native popup
positioning and bounded list infrastructure. Do not retrofit a multi-selection
list into the single `Choice.Config.selected` field or conflate query text with
committed selection.

Rust owns the trigger/popup lifetime, focus transfer, native search editor,
composition, highlight, scrolling and geometry. Bonsai owns committed selected
IDs and the supplied collection. Each rendered control has one generational
native owner. Query snapshots belong to its current search-editor lease; hiding
and remounting the search editor must never let an old replacement command target
a new generation. There is no synchronous OCaml call from paint, layout or input.

The implemented public data domain expresses distinct Single and Multiple modes, with
validated selection values and unique stable IDs. The config includes disabled,
search policy, trigger/search placeholders and clearability. Popup geometry is
part of the checked render description below.
Selected disabled options remain representable. Mode changes require an explicit
valid selection in the same update; never silently choose an arbitrary member.
Limits must account for all selected IDs, group metadata, rich content and native
editor state, not just the number of visible rows.

## Events and current-model decisions

Multiple activation submits a stable-ID toggle intent, not a replacement vector
copied from a rendered snapshot. Two rapid toggles of the same ID must apply in
order to the current application model. Single activation submits a select
intent. Clearing submits an explicit clear request; closure submits a reason.
The current model decides whether unavailable/removed items may change. Native
input also applies the latest accepted disabled/modal/composition gates.

Expose a single ordered event stream for selection intents and closure so an
application reducer can interpret close after preceding toggles. Do not promise
that arbitrary asynchronous callback effects finish before a close callback.
The native displayed selection follows accepted application state; do not invent
a second optimistic selection owner merely to copy the upstream synchronous API.
An application can implement veto/transform logic in its reducer without a Rust
callback into OCaml. Programmatic selection updates do not emit user intents.

Draft events need explicit open-state observations and dismissal reasons, with
mount snapshots, duplicate suppression and overflow behavior consistent with
other GPUIO controls. The data interface selects `Managed { initially_open }` or `Controlled bool`,
consistent with other anchored surfaces. Native integration must distinguish a
controlled open request from actual visibility, with the application deciding
whether to accept requests. Do not replay `initially_open` on every render.
The implemented event/transition contract is recorded below. Native focus,
layout, transport and owner lifetime integration remain required before the View
is exposed.

## Search, grouping and rich content

Local substring search stays native, Unicode-lowercase and bounded. Optional
application filtering uses query snapshots and Eio tasks with explicit current
query identity/cancellation. It must not admit stale remote responses or discard
selected values simply because a filtered result page omits them. Decide the
collection-versus-selected-value catalog contract before coding remote search;
reusing today's "selected ID must be in the complete collection" invariant is a
valid starting point, but document it explicitly.

Groups need stable identity, nonselectable header semantics and navigation that
skips headers/disabled options. Rich option content is a submitted passive View
subtree with its own text accessibility label. Native rows remain the only
selection targets. Custom trigger content similarly keeps a stable native
combobox owner and a separate accessible committed-value string. An integrated
clear affordance has an explicit action rather than an ambiguous nested hit area.

Custom empty content and footer are typed slots. Footer actions may be interactive
and therefore need explicit focus/Tab/escape behavior inside the popup; they must
not become option-selection targets. Rich rows/group headers need an explicit
height/measurement model that preserves virtualization. Do not drop virtualization
or accept synchronous OCaml per-row render callbacks to implement these slots.
The exact slot shape, limits and resource ownership must be drafted as `.mli`
contracts and validated in native admission before exposure.

## Implementation and acceptance sequence

1. Finalize types and examples for mode/selection, groups, slots, open requests,
   ordered events and query ownership. Inventory decoder/allocation/cache limits.
2. Implement validated Core/protocol/native domain types with independent binary
   fixtures and atomic rejection. Choose the paired protocol extension explicitly;
   the positive legacy capability mask is already full.
3. Implement the native owner with single/multiple requests, controlled selection,
   popup search editor, clear/open/close, focus and composition behavior. Connect
   public Core/Bonsai/Eio APIs and current-model callback fencing.
4. Add grouped virtualized rows and rich trigger/option/empty/footer slots with
   accessibility and disposal. Preserve selected IDs through filter/reorder and
   validate removed-value policy rather than silently truncating selections.
5. Publish a gallery and independently installed consumer example. Test rapid
   toggles and close ordering, same-label/different-ID options, disabled transitions,
   empty and maximum collections, async query races, IME, clipping/occlusion,
   keyboard/AX activation, two windows, culling and repeated mount/close resources.
   Measure idle and bounded workload behavior. Run actual macOS acceptance and
   required Linux build/unit/consumer checks; keep deferred Linux desktop evidence
   separate.

The draft deliberately leaves unresolved choices visible. It is an implementation
entry point, not evidence that all source features are mapped or a reason to mark
OCH-41/OCH-17 complete.


## Validated Core domain checkpoint

`lib/core/choice_picker.mli` now defines data values only:

- `Group.Id` has a namespace distinct from `Choice.Id`. Groups have accessible
  text labels and existing choice collections; empty groups are valid.
- `Collection.flat` and `grouped` retain ordering. Grouped collections admit at
  most 256 groups, 4,096 total items and 262,144 aggregate bytes across group/item
  IDs and labels. Group and item IDs must be unique in their respective namespaces.
  Count/text limits are checked before building the flattened lookup collection.
- `Selection.single` admits None or one ID. `multiple` preserves ordered unique
  IDs, capped at 4,096. A cached typed string set supports membership. Config
  validation requires all selected IDs in the complete supplied catalog, even
  when disabled or filtered out of the current presentation.
- `Config` validates labels/placeholders and carries search, open, disabled and
  clearable preferences. These values perform no I/O and open no native window.
- `Config.apply_request` is an optional pure reducer against current state.
  Select and Toggle apply to their respective modes and reject missing/disabled
  items. Disabled controls reject all requests. Clear is explicit, requires
  clearability, preserves mode and removes every selected ID, including disabled
  selections. Applications may implement a stricter policy in their own reducer.

These data-level guards supplement native session identity checks; mounted input
must also honor modal/composition gates and callback retirement. The codec below
is now carried by the paired bridge operations described later. Data validation
alone does not establish rendered-control behavior.


## Standalone configuration codec checkpoint

`Choice_picker_wire` and Rust `choice_picker::Config` encode fields in this order:
label, collection, selection, disabled, search, clearable, open state, trigger
placeholder, search placeholder. Item records contain ID, label and disabled.
Group records contain ID, label and item list. Bin_prot constructor tags are:

| Domain | Tags and payloads |
| --- | --- |
| Collection | Flat 0 (items), Grouped 1 (groups) |
| Selection | Single 0 (optional ID), Multiple 1 (ordered IDs) |
| Search | None 0, Substring 1, Application 2 |
| Open state | Managed 0 (initial Boolean), Controlled 1 (requested Boolean) |

The independent grouped fixture includes Unicode, a selected disabled item and
selection order differing from item order. The flat-empty fixture exercises
Single None, disabled, application filtering, managed initial-open preference and
empty placeholders. Both languages encode exactly those fixture bytes.

The Rust decoder caps group and cumulative item counts before allocation. It
charges each group/item ID and label to the shared 262,144-byte catalog budget
while decoding, not after constructing all groups. Selected IDs have a separate
262,144-byte decode budget; valid unique member IDs necessarily fit within the
catalog's ID budget. Scalar strings have their own bounds, UTF-8 is checked, and
Booleans/tags/trailing bytes are validated. Maximum standalone payload size is
786,432 bytes, conservatively covering both text budgets plus bin_prot overhead.
Final validation rejects duplicate IDs, absent selections and invalid labels.

Core `Expert.to_wire`/`of_wire` preserve the typed invariants; conversion from
wire values revalidates before constructing typed collections and selections.
Generated OCaml readers are used for the fixture check; no incoming configuration
message path into OCaml is introduced. The native bound-aware decoder is the
admission boundary for Op72. The later sections describe Kind53, Event69 and
current native tree/session ownership. The codec itself does not establish that
the picker renders or responds to input.

## Ordered events and native popup state

`Event.Selection_requested` carries a `Selection_request` containing Select ID,
Toggle ID or Clear plus the exact query snapshot when search is enabled.
`Selection_request.request` feeds the current-model reducer; `query` supplies the
native editor lease/revision for a conditional replacement after acceptance.
`Open_requested` carries the desired Boolean plus Trigger, Keyboard, Escape,
Outside_pointer, Focus_left or Selection. Trigger permits either direction;
Keyboard opens; the other reasons close. `Visibility` distinguishes a subscription
Snapshot from a Changed edge with Interaction(reason), Application or Unavailable
cause. Unavailable can only close. Wire/domain validation rejects impossible
direction/reason pairs and malformed selection IDs.

The standalone event encoding has a 262,656-byte bound: at most 262,144 bytes of
query text plus 512 bytes for IDs, editor metadata and bin_prot overhead. Event
tags are selection 0 (request, optional query), open request 1, visibility 2 and
query changed 3 (query). Request tags are Select 0, Toggle 1 and Clear 2.
Open reasons use the order above (0..5). Visibility is Snapshot 0 (Boolean) or
Changed 1 (Boolean, cause); causes are Interaction 0 (reason), Application 1 and
Unavailable 2. `choice-picker-events.hex` specifies independent bytes, including
Unicode IDs. Core conversion validates before producing typed events. These are
standalone payloads carried by Event69 as described below.

A query payload contains its child NodeId and the existing editor snapshot:
revision, text, byte-offset selection, optional composition and focused flag.
`Editor_wire` holds the shared OCaml representation, transparently aliased by
`Wire.Editor`; existing editor field ordering and bytes are unchanged. The picker
query validates single-line UTF-8 without NUL, nonnegative revision, selection
scalar boundaries and ordered composition boundaries. Query_changed may describe
composition; Selection_requested may not. The additional independent query
fixtures cover composing observations and selection/clear snapshots.

The intended mounted search editor is a distinct accepted child Input node. Its
identity survives popup close/reopen, but removal/remount retires its generation.
Native state binds that child identity with the accepted configuration and rejects
missing or mismatched query snapshots. Search None requires no child; Substring
and Application require one. Core conversion receives the envelope's window and
the current expected child identity and rechecks presence/generation before making
a `Text_input.Snapshot`. Equal revision numbers across remounts do not establish
identity. Exact query capture occurs before selection/close; closure must not
replace text. A later explicit reset uses the existing conditional editor command
to reject newer typing or a replacement lease. Never substitute the controller's
last Bonsai observation for this activation snapshot.

The widget must read the accepted child editor synchronously on Rust's UI thread,
then enqueue the captured payload; it must not ask OCaml to read the query after
the selection event. The child's normal editor observation path supplies the
Query_changed payload to the same ordered picker stream. Tree admission and retained host ownership now validate and retain that child;
closed focus gating is connected. Mapping editor observations into the public
picker callback, native selection capture and IME/input behavior remain work.

`rust/native/src/choice_picker_state.rs` owns validated configuration, query-child
identity, eligibility and actual open state. It returns at most three events per call and retains no
queue. The following rules apply:

- Subscription samples actual visibility, including a closed popup. Only actual
  edges produce Changed; repeated configurations do not repeat it.
- Managed initial-open is read only on creation. Controlled requests emit intent
  without mutating actual state. A later accepted configuration changes visibility
  with Application cause. Repeated unaccepted gestures remain retryable.
- Single activation orders Select, close request, then actual closed for managed
  state. Multiple activation emits Toggle and stays open. No optimistic selection
  vector is stored; the latest accepted application selection remains authoritative.
- Switching Controlled to Managed preserves actual state, ignoring unaccepted
  requests. Disabled/hidden/culled eligibility closes with Unavailable, without
  inventing a user request. Recovery reopens only an accepted Controlled true;
  managed state stays closed until another gesture.
- Current disabled/missing items and composing activation are rejected. Explicit
  clear requires availability and clearability, preserves popup state, and remains
  an intent even when selection is empty. Invalid configuration is rejected before
  any state or eligibility change.

This pure state is not native input acceptance: the native interaction adapter must handle
IME key consumption, modal/focus/occlusion eligibility, query editor leases and
accepted node/handler generations before dispatch. A retired owner sends no final
callbacks. Returned requests enter the existing bounded FIFO; overflow must fault
the stream rather than silently drop selection or close intents. Transport wiring,
overflow integration, subscription replacement and lifecycle acceptance remain open.

## Grouped row projection and variable-height list state

The native `choice_picker_rows::Projection` retains the complete accepted catalog
and builds bounded row locations for the current query. At most 4,096 items and
256 headers exist. It keeps item/group lookup maps, enabled row indices and cached
selection flags; query/highlight changes do not copy a replacement selection.
Build a projection when the catalog/query changes, not inside paint or a per-row
callback. Rendering and navigation share the same immutable projection.

Group and item keys have distinct namespaces, even with identical ID strings.
Native Substring search matches Unicode-lowercase item labels and preserves source
order, with no normalization or accent folding. It hides groups with no matches;
an unfiltered projection retains empty headers. The empty-content slot uses item
count rather than total rows, so headers alone do not suppress the empty state.
None and Application search modes leave the supplied catalog unfiltered. Selected
IDs outside native search results remain committed in the retained full catalog.

The native cursor stores stable item identity. Filter/reorder preserves an enabled
visible active ID. Initial opening, or loss of that ID, chooses the first enabled
visible selection in selection order, then the first enabled item. Navigation
wraps through enabled options and supports first/last; headers and disabled items
are never pointer-highlight or keyboard-selection targets. This row traversal
does not itself route keyboard events or establish accessibility semantics.

`choice_picker_list::State` uses GPUI's variable-height `ListState` with a positive
finite height estimate (1..1,000,000 logical pixels) and bounded overscan
(0..4,096). Estimates initialize scrollbar geometry; actual rendered rows are
measured. Structural updates splice the differing middle span, preserving common
prefix/suffix. The visible row's key and intra-row offset survive regroup/reorder.
If the anchor disappears, use its old next surviving row, then the preceding
survivor, starting at offset zero.

Content/selection/disabled changes with stable keys remeasure affected rows without
resetting scroll input. Rich subtree changes can invalidate an explicit group/item
key. Revealing the active option is an explicit navigation action, not an
every-render operation that would fight wheel scrolling. All updates happen before
layout; the native render closure receives a cloned handle and immutable projection
and must not reborrow ListState while GPUI owns its layout borrow.

The headless GPUI fixture retains one render-view entity and verifies 18/28/52-pixel
rows, fewer than 32 rendered rows for a 4,096-item catalog, and stable visible key/
offset when a preceding row shrinks or an earlier item is removed. This tests the
list adapter, not a rendered picker or actual OS input. Atomic rich-slot admission
and accepted-tree ownership are described below. Connecting this list to popup
rendering, row/section accessibility, geometry, query input and the public View
still needs implementation and acceptance.

## Checked render description and slot admission

`Choice_picker.Query` supplies a stable controller key and validated single-line
mount seed. `Description.create` ties configuration and appearance to optional
query, trigger, empty and footer content plus keyed group/option overrides. It
rejects duplicate or absent IDs and requires a query exactly when search is
enabled. Content is generic to avoid a dependency cycle with View; this constructor
alone does not establish passive View safety or create a control.

`Option_content` defaults to the native selected indicator. `Checkmark.Custom`
suppresses that indicator so the passive row content can draw a custom icon or
other selected presentation from the committed application model. The catalog's
text label remains the native accessible name regardless of rich presentation.
Only the footer may contain interactive descendants. Trigger, option, header and
empty content must be passive visual subtrees; this includes decorative progress
and animation but no callbacks, text selection or independent scrolling.

The new Appearance has explicit popup width/max-height, estimated row height and
overscan rather than a uniform row-height promise. Defaults are 320/320/32/64
logical pixels. Width/max-height are positive finite values up to 1,000,000;
estimate is 1..1,000,000 and overscan 0..4096. The popup is additionally clamped
to the native window. Popup/option/header/empty paint and text styles use the
existing Choice vocabulary and share a 128-declaration budget. Header and empty
allow Base only; popup also allows Hovered; option additionally allows Focused,
Pressed, Selected and Disabled. Layout/input properties remain on appropriate
Views, not on these internal paint parts.

The standalone presentation fields are configuration, width, max-height, estimate,
overscan, empty label, popup/option/header/empty style lists and slot descriptors.
Slot tags are Trigger 0, Query 1, Empty 2, Footer 3, Group 4 (ID) and Option 5
(ID, Checkmark Native 0 or Custom 1). Slots follow that fixed-part order, then
the supplied group and option order. A future View uses separately keyed wrappers
so caller keys cannot collide with structural slot identity.

Both runtimes match independent presentation bytes, including different values in
all four style parts. Native decoding caps the entire payload at 1 MiB, slot count
at 4,356, repeated slot-ID text at 262,144 bytes and cumulative style declarations
at 128. These are payload bounds, not a promise that every combination also fits
tree, batch or retained-memory quotas.

`choice_picker_admission` validates native nodes during atomic tree admission. Each role maps to an unstyled, unbound one-child container. The
validator checks distinct child ownership/parent links, the 4,096-node/128-level
slot-forest limit, passive visual content, and an Input child matching the picker
label, search placeholder and disabled state. The query editor has no autofocus
or submit-on-enter and exactly one row; its native popup owner will manage focus
and choice confirmation. Its child handler remains required for ordinary editor
observations. Interactive footer content stays subject to ordinary tree validation.

The validator rechecks descendant callbacks and input styles, not only structure.
Kind53 `ChoicePicker` and Op72 `SetChoicePicker` carry the checked presentation.
The tree requires a root handler, empty root text and the matching configuration.
Dirty-ancestor traversal revalidates the entire slot forest after descendant
changes, including nonstructural Bind/SetStyle/SetEditor operations. Rejection
leaves the accepted revision, nodes and retained payload budget unchanged.
Picker/query disabled and label settings must change together in one transaction.

The tree charges retained catalog/selection/slot strings, collection storage and
all four style parts; child nodes are charged separately. Removal releases that
payload. The accepted-tree owner additionally reserves a second presentation-sized budget
for its cloned Config, 4 KiB for owner/map metadata, and 256 bytes per structural
slot for focus visibility and accepted-content maps. Wrapper arrays and Presentation
remain Arc-shared. Lazy projection/list storage reserves 512 bytes per catalog row,
copied group/item ID bytes, 64 KiB fixed metadata and two maximum query buffers
when searchable. This reservation applies even when closed; it is conservative
logical accounting, not a measurement of RSS. Ordinary editor memory remains
under its existing native contract. Tests exercise real atomic Tree updates,
rollback, budget rejection and release as well as direct admission checks.
The native popup and public reconciler are connected to these owners.
The public View constructor performs matching passive-content and aggregate
node checks before submission; see its integration checkpoint below.


## Bridge events and accepted-owner validation

Event69 `Choice_picker_event (window, node, handler, tree_revision, event)` now
carries the typed picker event. Both languages match independent envelope bytes
for selection/clear and ordinary/composing query observations. The OCaml event
reader checks nonnegative tree revisions and all nested picker invariants,
including UTF-8 boundaries, single-line queries and noncomposing selections.

`Session::choice_picker_event` checks the accepted window/node/handler, revision
range, overload state, current catalog/mode/disabled/clear policy and the exact
currently mounted query child. Rebinding the handler or remounting the query
invalidates old requests. An ordinary model commit preserves older-frame intents;
it must not discard a rapid second toggle because the first changed selection.
The current application reducer still decides how accepted intents affect its
latest model. Read-only query observations and actual closure observations can
complete while the picker is disabled; new open/selection intents cannot.

The mailbox retains these events in FIFO order without coalescing. Selected IDs
and query text count toward its existing byte ceiling. Draining stays within the
encoded batch limit; overflow returns failure for the native owner to report
through the existing explicit window-overload path. Tests verify repeated toggles,
closure ordering, byte pressure, bounded batches and released queue accounting.

This is transport/session integration, not mounted-widget acceptance. The native
interaction adapter must capture the exact live editor snapshot, apply modal/focus/visibility
eligibility before user intents, and handle transport failure. The public View
reconciler must install callbacks and revalidate their query lease at dispatch;
Eio must observe relevant editor snapshots before invoking application handlers.
The Eio window dispatcher recognizes the envelope, and the public picker View
binds the root callback to its accepted query identity as described below.


## Accepted-tree host ownership and closed-content focus

The production host now retains one picker owner per accepted node generation.
It synchronizes after tree/editor updates and before rendering, and removes an
owner only when that identity leaves the tree. It does not copy Select's
visited-only state eviction. Managed `initially_open` is evaluated once; ordinary
rerenders or a transition from controlled to managed state do not replay it.
Mount and handler-rebind snapshots, plus configuration/eligibility visibility
changes, use Event69 and the bounded transport with explicit overload faults.
This connects the native state model, not yet the popup renderer.

An independent focus-manager set hides closed popup structural slots while
keeping the trigger eligible. It participates in input/focus visibility and
visual-search eligibility without overwriting avatar/container-query visibility.
Nested pickers are synchronized in tree-depth order, even when a child was
allocated first. A closed parent footer cannot briefly advertise an open child.
Disabling a picker also blocks interaction in its descendants.

The query remains the same native editor entity while closed. Focus/Submit
commands are rejected by the ordinary editor gate, and a previously focused
query/footer is blurred on closure before the next layout. ReadSnapshot and
explicit programmatic text commands retain their ordinary editor contracts;
closure does not reset the draft. Trigger focus restoration, IME handling,
actual painted popup visibility/culling and gesture routing remain part of the
pending renderer/input integration. Retaining an owner is not proof that its
popup has been presented on a desktop.

Three TestPlatform host tests verify the actual retained editor focus handle and
text across open/close updates, blur-before-layout, mount-only managed state,
nested eligibility and independent visibility sets, exactly-once snapshots for
zero-slot pickers over 200 syncs, observer rebinding and owner/editor teardown.
The full feature-enabled native library suite passes 517 tests with two existing
ignored tests. These are headless host/focus checks, not OS keyboard/IME/AX or
rendered-picker acceptance.


## Native popup rendering and keyboard checkpoint — 2026-10-01

The host now renders the accepted trigger plus a deferred popup with retained
query editor, grouped variable-height virtual rows, passive rich options, empty
content and interactive footer. Popup width and total height are bounded by the
configured geometry and viewport. Row estimates initialize list geometry; they
are not imposed as actual minimum heights. Local filtering uses the current
native query. Selection gestures capture its exact lease/snapshot and submit
ordered requests without optimistic selection changes. Matched editor Enter
bindings are handled as actions, not solely raw key events.

Option-navigation and activation keys belong to the trigger/query, not arbitrary
footer descendants. Footer buttons keep their own Enter callback. The nearest
nested picker owns its keys, so Escape in an inner popup does not dismiss its
parent. Tab orders eligible popup descendants by submitted child order (and
explicit local Tab indices), inserted after the trigger among surrounding stops.
This handles deferred and cached paint ordering without a modal focus trap.
Leaving focus closes a managed popup without refocusing the trigger; Escape
restores trigger focus. Closure keeps the search draft and editor identity.
Controlled visibility remains application-owned, with separate close requests.

Closed owners reconcile retained row caches after accepted configuration changes,
even without redraw. The headless regression shrinks a 4,096-item catalog and
verifies the old configuration Arc is released, accounting decreases, and removal
releases picker/editor owners and tree payload accounting. These are bounded
logical-lifetime checks, not process-RSS or repeated real-window measurements.

Five TestPlatform host tests cover owner lifecycle and the rendered interactions;
the complete feature-enabled library suite passes 519 tests with two existing
skips. The rendered fixture checks fewer than 32 rows visited from 4,096 items,
a 52-pixel rich row, the whole-popup height bound, rapid nonoptimistic toggles,
filtering, footer activation, Tab exit, Escape/draft retention and nested key
ownership. TestPlatform creates no OS desktop window. This is not physical
macOS keyboard/IME/VoiceOver or installed-consumer acceptance.

Remaining native work includes measured clipping/culling and controlled recovery,
complete accessibility semantics/activation and clear-keyboard policy, composition
and native query-change delivery, geometry/ancestor-style invalidation and broader
resource scenarios. Public Core View/reconciler/Eio/Bonsai and the first gallery example are wired
below; independent consumer and actual macOS acceptance are still required.


## Public View and query-controller wiring — 2026-10-01

`View.choice_picker ~on_event description` validates passive slots and the total
wrapped child forest before creating a View. The wrappers are genuinely unstyled
containers, keyed by slot namespace; supplied content keys cannot collide with
structural slots. Structural identity is a separate namespace/ID pair in the
reconciler, not a concatenated public Key: valid 256-byte group/item IDs work
without truncation, hashing or namespace collisions. The query is a single-line native Input with its own placement
key and mount-only seed. Its disabled/label/placeholder policy changes atomically
with the root configuration. Query keys participate in the ordinary window-wide
controller uniqueness check. The Bonsai View module exposes the same constructor.

The reconciler emits Kind53/Op72, retains equal native identities across updates,
and binds the root callback to the actual query child NodeId after mounting its
children. Prepared updates cannot mutate an accepted callback's query identity.
Dispatch rechecks current mode, item availability, clearability, query identity,
window/handler generations and revision. Ordinary selected-value updates retain
queued toggles; disabled transitions rotate the observer. A remounted query
rejects old selection snapshots even if the root and its observer remain mounted.

The native editor recognizes its accepted picker query slot and sends
Query_changed through the root Event69 route, with its exact child identity and
snapshot. It does not also send an ordinary Editor_event for that query. The
internal input handler remains a generational command lease; direct editor
callback events are discarded by the picker query binding. Query observations
therefore share the same noncoalescing bounded FIFO as selection/open/visibility.
Initial query publication is delayed until after the owner's visibility snapshot;
rebinding or replacing the query republishes its current snapshot once. Native
focus/selection/composition changes use the same stream. This preserves ordinary
editor commands without introducing a second application observation channel.

`Gpuio_eio.Choice_picker.create window ~config ~on_event graph` owns the Bonsai
editor controller. The default root key uses its stable Bonsai placement identity.
`view` accepts appearance and rich slots and returns an
Or_error for invalid composition. Its event adapter observes query snapshots
before scheduling the user's callback. `replace_if_unchanged` uses the exact
snapshot captured in a selection request; it never guesses from a recent draft.
No-query requests return Not_mounted; remount and revision races retain the
ordinary Stale_editor/Stale_revision contracts. Disabling search removes the
query placement; closing the popup keeps it.

The Pickers gallery now includes a grouped multi-selection capability chooser
with rich rows, a disabled option, native substring search and an interactive
clear footer. Its reducer applies each intent to current selection. This is a
public API build example, not yet a visually reviewed/native-input-qualified
example. Full Core tests and the gallery/backend build pass at this checkpoint;
current installed-consumer, OS input/IME/AX, culling and release acceptance remain
open. The headless composition check verifies first-Escape cancellation without
closing, and stale callback Tab cannot move focus after observer rebinding.


### Measured trigger visibility — 2026-10-01

The retained owner now samples trigger visibility from the completed paint,
including deferred nested footer content. A positive intersection of the trigger
bounds and current content mask counts as visible; an omitted or fully clipped
trigger does not. Before the first measurement the accepted tree supplies initial
eligibility. The host combines subsequent measurements with its existing ancestor,
modal and disabled gates.

A visibility change runs the existing state transition and requests one redraw.
Fully clipping an open trigger reports `Changed(false, Unavailable)`, hides popup
focus targets and preserves the accepted owner, query editor and managed state.
Restoring a trigger does not replay managed initial-open. An accepted controlled
open preference recovers with `Changed(true, Application)`. Repeated unchanged
paint is silent and does not request another frame. This works for native scroll
offsets without an OCaml transaction, as well as accepted layout changes.

A TestPlatform regression covers complete and partial clipping, once-only closure,
managed versus controlled recovery, and native scroll-away/scroll-back at the same
tree revision. It exercises production layout and paint; it is not evidence of
physical desktop presentation, OS accessibility or the unresolved black startup
window. Cross-window, OS occlusion and broader nested resource scenarios remain
part of release acceptance.


### Clear keyboard control and semantic metadata — 2026-10-01

A clearable picker owns a separate retained native focus handle for its Clear
button. Tab order is trigger, Clear, query (when open), then footer descendants;
Shift-Tab reverses that order. Popup descendants are placed after the compound
trigger parts using accepted child order, independent of deferred paint order.
The search editor's Tab/Shift-Tab actions are intercepted only while that query
owns focus; footer editors retain their own editing actions. Existing current
handler, nearest-picker and marked-composition guards apply before navigation.

Enter or Space on Clear emits an ordinary clear selection intent with the exact
current query snapshot when present. It preserves popup state, query draft and
committed selection until the application updates its model. Removing clearability
while Clear is focused restores the eligible trigger; disabling or hiding its
owner removes that focus. Clear has a keyboard focus indicator and a separate
accessibility Focus/Click target. It does not use query Backspace/Delete shortcuts.

The native semantic wrapper preserves each element's identity, layout and action
routing. Listboxes expose multiselectability; options expose selected/disabled
state, and disabled options and clear controls advertise no actions. Disabled
triggers no longer register a Click accessibility action. Direct production
Element metadata checks and TestPlatform keyboard fixtures do not establish
VoiceOver or external AX delivery. Group-to-option semantic relationships,
active-descendant focus from the query, virtualized collection positions and real
macOS accessibility acceptance remain required work.


### Virtual row identity and logical membership — 2026-10-01

Native rows now use the full accepted item/group ID plus a distinct kind tag as
GPUI element identity. A row index is not an identity: filtering or reorder must
not give a different option the previous row's native state or AX node identity.
There is no truncation or concatenated public-key limit; the two namespaces remain
distinct even when a group and item have the same 256-byte ID.

The immutable projection records filtered item counts for each group (at most 256
counters). It exposes zero-based logical position, count and optional group for
options; headers have no option membership. Disabled options count toward the
logical set. Native semantic metadata converts to one-based positions.
Filtering/reorder updates positions; scrolling/virtualization does not. Flat
collections use the filtered flat collection's positions/count.

The list's accessibility prepaint captures the actual mounted row IDs and wraps
each contiguous group in a named synthetic Group. Existing options keep their
IDs, actions and rich child subtrees. The synthetic group's ID derives from the
list identity and full catalog group ID; scrolling its visual header away does
not remove the group context. Visual headers are hidden decorative nodes inside
that group, avoiding a second group announcement. Group names are no longer
repeated as option descriptions. Flat collections retain direct option children.

The capture resets before each list prepaint. Only current direct children are
eligible; stale capture records cannot introduce new children. Grouping rejects
invalid, noncontiguous or duplicate members and occupied IDs without changing the
tree. No offscreen options are materialized. This is the implementation contract;
external AX/VoiceOver acceptance is still required.

The renderer reservation now charges three ID-text copies per catalog row for
projection keys and GPUI current/previous frame identities. It also reserves four
copies of each display/semantic label across frame/AX metadata. Group text is
charged per group: mounted rows hold projection indices and share its immutable
catalog rather than copying group IDs/labels per option. Fixed and per-row
metadata allowances cover the capture map. Group counters fit within the per-header
metadata reservation. These remain conservative logical charges, not RSS claims.

Structural slots render exclusively in their picker-owned trigger/popup locations.
They are excluded from the host's generic child traversal. Accessibility-enabled
testing exposed a duplicate query/footer mount through that traversal; the repair
keeps one input node and one native focus owner for the query.


### Explicit search-focus accessibility ownership — implementation contract

The search input and popup list are siblings in the native accessibility tree.
GPUI's ancestor-only active-descendant mechanism cannot associate a row with that
input without changing real focus or fabricating ancestry. The scoped adaptation
adds `aria_active_descendant_for(&FocusHandle)` for the highlighted row. It holds
a weak handle and only claims accessibility focus when that exact live handle
still has window focus and identifies the accessibility tree's real focused node.
The input must already have been exposed in the current prepaint; the picker puts
its query before its list. Missing/unfocused/retired owners and missing/self targets
are ignored. Existing ancestor-only behavior and multiple-claim safeguards remain.

GPUIO supplies the search handle only while that query is focused and has no
marked composition. Otherwise the ordinary ancestor rule applies, keeping query
composition and footer/Clear focus with their real controls. Selection and native
keyboard focus do not change when the highlighted row is announced. Current row
identity, observer fences and filtered-catalog activation checks remain in force.
This section defines the implementation being validated; external macOS
AX/VoiceOver acceptance is still required.

### Controller sequencing and query retirement

The Eio public adapter keeps its existing `App.Window.t` API. Internally, the
private Bonsai component receives a typed editor-command capability, bound in
production to `App.Window.Expert.editor_command`. This keeps exact editor leases
at the transport boundary and permits deterministic testing without a native window.

Query and selection observations are injected before the application effect.
A callback using Bonsai's effect-time `peek` sees the updated controller snapshot;
an immutable controller value captured earlier still describes that earlier render.
Older observations on the same lease cannot roll the snapshot backward. A delayed
successful command reply cannot replace a newer observation, or an observation
from a different editor generation. The command's own completion result remains
available to its caller even when it is too old to update the reactive snapshot.

Disabling search makes `snapshot` absent and command methods return `Not_mounted`.
This includes `replace_if_unchanged` with a selection captured before search was
disabled; no command is sent. After a replacement query is observed, that old
selection returns `Stale_editor`. Conditional replacement otherwise sends the
selection's exact query lease and revision, letting native code reject later typing
or composition. A command effect created before remounting remains bound to its
original lease and cannot silently target the replacement editor.

The private inline expect tests run the real Window_driver, reconciler and Bonsai
scheduler with a controlled command capability. They verify sequencing, revision
monotonicity, query retirement/remount, stale routed event rejection, late replies
and delayed effects. They do not simulate native command validation, Eio I/O,
physical input, OS focus or application-window shutdown; those require their own
integration evidence.

### Inherited text metrics and retained row measurements

The native list now observes its resolved inherited text style during
`request_layout`, inside the popup's ancestor style context. Reading it earlier
while constructing the View missed ancestor-only changes: visible rows were
remeasured by GPUI, but offscreen rows retained their previous heights.

A per-owner cache records the resolved text metrics, base rem size and display
scale. A change marks retained heights for remeasurement before entering the
list's layout borrow. It uses absolute item/offset preservation, without replacing
ListState or resetting the visible item. Paint-only color/background/decorations
are excluded from this cache comparison. Popup overrides participate in the
resolved style; an ancestor change masked by an override does not invalidate it.
Normal unchanged frames retain cached offscreen measurements.

The actual native View regression first reproduces stale offscreen heights after
an ancestor line-height change. After repair it verifies visible heights, retired
offscreen measurements, remeasurement on return, and the same row plus five-pixel
scroll offset through both growing and shrinking text. It also checks popup line
height overriding later ancestor changes and cache retention across idle frames.
This is TestPlatform layout evidence, not physical scrolling or display-scale
qualification. Wider native resize/placement/theme and input acceptance remain open.


### Controlled visibility and user focus

A controlled open request leaves visibility and keyboard focus unchanged until
OCaml accepts it. The native owner remembers at most one focus handoff associated
with that gesture. When the matching visibility change arrives, an open moves
focus to the retained search field when present; Escape or single-selection
closure restores the eligible trigger. Outside-pointer and focus-leaving closes
do not restore focus.

The handoff uses a weak reference to the exact initiating focus handle and a blur
subscription. Moving away cancels it even if focus later returns to that handle.
Observer replacement, query replacement, disablement, ineligibility and leaving
controlled mode also cancel it. Ordinary unchanged updates retain it; it is consumed
once when visibility changes. Owner retirement drops both the handoff and its
subscription. No application callback runs synchronously from native layout/input.

Initial visibility and unsolicited programmatic changes never claim keyboard
focus. In particular, restoring a clipped controlled popup cannot steal focus from
another control. Applications wanting explicit programmatic focus can use the
existing query controller after receiving its mounted editor observation.


### Viewport resizing and wrapped row content

Option and group content occupy a shrinkable flex region with a zero minimum
width; the native selection mark keeps its width. Plain labels can therefore wrap
within the actual remaining row width instead of forcing one-line content past
the popup edge. Rich submitted content gets the same available region while
retaining its own explicit style constraints.

The retained GPUI List invalidates measurements on width changes. Native viewport
resize and scale changes keep the logical item plus pixel offset; they do not
need an OCaml transaction or scroll an unchanged highlight back into view. The
popup clamps its total size and resolves placement from current trigger geometry.

The TestPlatform View regression narrows from 600 to 240 logical pixels, reduces
height to 140 and restores the original viewport, checking wrapped row heights
and item 50 plus a five-pixel offset throughout. It also exercises simulated scale
changes from 1 to 2 and back. With a query and a 32-pixel footer added, both remain
inside the popup, the list keeps positive height, and query identity survives
resize. Three settled draws schedule no frame callbacks. These are specific
layout/idle-scheduling checks, not all possible rich-slot sizes, OS resize/input,
external AX, physical display scaling or process-level idle CPU acceptance.
