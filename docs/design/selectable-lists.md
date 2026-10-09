# Persistent selectable/searchable lists — OCH-41

Status: pure collection identity/selection and the paired native bridge/semantic
metadata foundations are implemented, with [model evidence](../evidence/selectable-list-model-och41.md)
and [bridge evidence](../evidence/selectable-list-bridge-och41.md). Production native input and scoped accessibility adaptations now have
[local regression evidence](../evidence/selectable-list-native-input-och41.md);
the [public Core callback API](../evidence/selectable-list-core-input-och41.md) is now
implemented. The [managed Bonsai component](../evidence/selectable-list-managed-och41.md)
now has local lifecycle and selection evidence; [Eio search and gallery](../evidence/selectable-list-search-och41.md)
are implemented locally. This is the standalone list mapping in the
[pinned collections review](../catalog/collections-review.md); popup pickers do
not satisfy it. The managed API and search/gallery are available; physical native
acceptance is not complete.

## Ownership and public model

Keep application records in `List_collection`; query tasks live in Eio scopes
outside transient rows. Reuse managed virtualization, both axes, stable anchors,
active budgets, shared scrollbars and generation-fenced controllers. No native
layout/search callback calls into OCaml synchronously.

Add opaque collection `Item_ref` values containing source identity, application
key and membership lifetime. Point updates, reorder and atomic replacement of a
surviving key preserve the lifetime. Removing then reintroducing a key, or creating
a new collection, retires old references even if no renderer observed the intermediate
state. A metadata-only `Identity` snapshot contains order/membership and is shared
by point updates; eligibility/query models must not scan all application data for
every streamed value or keypress. These references are process-local, not serialized
persistence IDs or native node handles.

`List_selection.Catalog` combines Identity with an optional visible order and
explicit disabled keys. Construction validates duplicate/absent keys and bounds;
headers/footers can be nonselectable entries in the flattened order. Query filtering
changes visible membership without deleting loaded application records. Selected
hidden items survive; deleted/reincarnated selections do not. Build a catalog only
when identity/order, visibility or eligibility changes.

`List_selection` holds separate logical cursor, selected memberships, range anchor
and context target. Cursor is not a claim of OS focus. Previous/next/first/last skip
disabled entries; Stop/Wrap are explicit boundary policies. Navigation without a
selection gesture changes only the cursor. Single/Multiple control selection;
replace/toggle/range are explicit gestures, and AX selected setters are idempotent.
Programmatic selected values may include hidden/disabled loaded items. A context
request never silently replaces selection. Confirmation validates the current
eligible cursor/reference and distinguishes primary from secondary activation;
it emits an intent and does not secretly toggle selection. Cancel clears cursor,
range anchor and context while preserving committed selection. Query reset is an
application decision. Changing catalog repairs a removed/hidden/disabled cursor
at its previous visible position (next eligible, then preceding), without selecting
an unrelated item. A clear cursor stays clear until navigation or a request.

Navigation on an unchanged catalog must not scan all selections or records.
Catalog construction/reconciliation is bounded by loaded metadata; range selection
visits the range it changes. Active native rows and Bonsai models remain separately
bounded by Virtual_list, not by the total loaded data size.

## Native input and search integration

Use dedicated native list input and ListBox/Option semantics around the existing
virtual-list owner, with selected/disabled/position metadata and a logical cursor
that can be revealed/materialized. Native focus, retained handles, composition,
child-control precedence and synchronous key consumption stay in Rust. Send ordered
relative navigation/confirm/cancel and generation-fenced target intents to OCaml.
Cursor navigation must not silently become a selection snapshot. Pointer primary,
secondary activation and independent context targeting need equivalent keyboard
and accessibility paths. Tab leaves the list normally; row controls remain usable.
Orientation selects navigation axis; secondary modifier follows the platform.

Expose checked Core configuration, typed requests and Bonsai controller/output,
with caller-owned row/section/empty/loading content and styles. An Eio search
example must debounce/cancel superseded work and reject stale completions by query
identity/generation, including filter changes while selected items are hidden.
Independent source replacement retires controllers. Search input uses the real
native editor and preserves IME; Escape semantics must respect composition and
focused child editors before list cancellation.

## Completion evidence

Core expect/property checks: source/item reference fencing, typed identities,
query visibility, disabled/empty/all-disabled order, wrapping, cursor vs selection,
range anchors, context, primary/secondary confirm and cancellation. Include large
loaded metadata with bounded per-navigation work and point-update identity reuse.
Paired codecs/admission, native focus/IME/keyboard/AX/pointer ordering and lifecycle
checks follow the typed contract. Bonsai/Eio tests must prove row retention,
controller/query lifetime, failure/retry and no unbounded per-row tasks. A public
persistent list gallery demonstrates all of these choices, with installed-consumer
and physical macOS evidence. Linux non-GUI gates remain required; OCH47 desktop
qualification is deferred. None of the remaining adapter/gates is completed by a
passing pure reducer test.

## Paired bridge contract

The bridge foundation appends Op110 `Set_list_input` and Event74 `List_input`;
older tags/record layouts stay unchanged. Configuration is fixed-size: positive
interaction generation, optional logical cursor ID, optional query editor node,
selection-on-navigation policy, disabled and busy flags. There is no native copy
of the full selected set. Native admission requires a handler and ListBox role on
VirtualList, and excludes tree/table input on the same owner. Either list axis is
valid. A cursor belongs to the current logical order; when mounted, it must have
enabled Option metadata. The cursor may be unmounted pending reveal/materialization.

A query editor is a real single-line Input sibling with one list owner, within
a common container. Raw node IDs are an internal adapter concern, not an
application API. Removing/reparenting an editor or changing option metadata also
revalidates the unchanged list owner; a rejected transaction preserves the old
tree and generation. Updating query ownership, disabled state or navigation
selection policy requires a strictly newer interaction generation. Cursor and
busy updates may use the current generation, preserving queued relative arrows.
Clearing input retains a generation watermark: reinstalling must advance it.
The managed adapter must also advance the generation when source/query identity
changes without changing the physical query node.

Requests distinguish navigation with an optional selection gesture, keyed focus,
selection, confirmation, context and idempotent selected setters, plus relative
active-selection/confirmation/context and cancellation. Primary/secondary confirm
is explicit. Session admission checks window, handler, revision, generation and
current enabled mounted Option targets. Relative requests carry no stale cursor
snapshot and remain ordered in the bounded mailbox; they are never coalesced.
Busy is semantic state, independent of whether input is disabled.

Accessibility roles append ListBox(multiple) and OptionItem(index, known/unknown
count, selected, disabled). Index/count describe logical options rather than the
mounted subset and are bounded by one million. Native metadata preserves selection
independently of cursor/native focus, and disables actions on disabled options.
The Core row wrapper transfers option semantics onto the actual virtual row.

## Production native input contract

The enabled owner retains one native focus handle. Its visible logical cursor is
an accessibility active descendant, independent of selected metadata. The explicit
query editor may retain keyboard focus while lending accessibility focus to the
cursor, whichever sibling paints first. Composition restores the editor's own
accessibility focus. The GPUI adaptation holds one per-frame claim and rolls it
back with rejected prepaint attempts; it does not move keyboard focus.

Window routing honors explicit Override shortcuts first, then list input before
editor-bound Enter/Escape. List routing declines inactive windows, composition,
hidden/disabled owners and unrelated child-editor focus. Queries retain ordinary
text, caret, Home/End, Space and Tab behavior; Up/Down navigate results. A focused
list uses its configured axis and Home/End. Shift requests a range, the platform
selection modifier requests cursor-only navigation, and ordinary arrows follow
selection-on-navigation policy. Space toggles the active option; Enter confirms,
platform-modifier Enter confirms secondarily, Shift-F10 opens context and Escape
cancels. Tab follows ordinary focus traversal, including embedded row controls.

Matched primary clicks select once; exactly the second click confirms without
another selection toggle. A matched secondary click requests context without
moving focus or selection. Embedded controls retain precedence. Row interaction
IDs include handler and input generation, so an old mouse-down cannot complete
after an interaction epoch is replaced. Actions also validate the current owner,
handler, epoch and target at delivery. No native selected-set mirror is introduced.

Accessibility supports focus, select, desired selected setters, primary/secondary
confirmation and context as distinct actions. The macOS adapter opts into desired
setters only for enabled selectable ListBoxOptions declaring both custom selection
IDs and their handler. It queues every requested value, even if equal to the
rendered snapshot: a preceding opposite setter may still await OCaml reduction.
Ordinary options and other roles retain their existing behavior.

Cursor changes request bounded native reveal/materialization while retaining the
owner's focus. A logical cursor is not an OS-owned row eviction veto: replacing it
must work even with an active-row budget of one. Actual focused child editors and
composition keep the existing managed-list retention rules. Busy/cursor updates
preserve the input epoch; disable, removal and input clearing retire native focus.

Public callbacks, query-key resolution and the managed Bonsai component now have
local regression evidence. Eio search and gallery composition are implemented;
physical macOS acceptance remains required before closing the standalone list capability.

## Core callback and query reference API

`List_input.t` maps relative/keyed native requests to typed application callbacks.
`View.with_list_input` attaches `List_input.Config` and a callback to an existing
ListBox virtual-list view. Option semantics remain on the row descriptions; the
attachment does not infer labels, disabled membership or selected preferences.
The Bonsai View facade specializes callbacks to effects, without introducing a
second scheduler or a native callback into OCaml.

The checked configuration uses an application epoch key, optional cursor row key,
optional direct sibling query View key, selection-on-navigation, disabled and busy
flags. Query lookup resolves after all siblings mount and requires a unique Input
owner. A missing, nested, multiline or shared query rejects the whole preparation.
If `View.with_key` changes an editor key, use that View key for the query reference.

The reconciler assigns native generations from accepted mounted state, including
a watermark across clearing/reinstallation. Cursor and busy changes preserve
queued input; policy, query ownership/controller, mode and application epoch
changes retire it. Preparation remains speculative until native acknowledgement.
Callbacks therefore capture the application epoch too and reject obsolete work
inside the reducer if state changes while a native commit is pending. Native
row IDs map only through the current logical identity; removed targets are dropped.
Use incarnation-safe row keys when removal/reinsertion can coalesce before rendering.

This is the low-level composition API. The managed component supplies coherent
row/state/controller/query lifetimes and themed presentation so ordinary
applications need not build these mechanisms themselves.

## Managed component API

`Gpuio_bonsai.Selectable_list.component` accepts an application collection,
`List_rows.Layout`, reactive viewport configuration and an `Interaction` policy.
The policy names the current query epoch and selection/navigation mode. Busy
updates preserve queued requests. Source replacement is detected from collection
identity; callers do not supply a parallel generation counter.

The component owns the `List_selection` model. `Output.state` observes it;
`Output.target` captures incarnation-safe references and `Output.controller`
provides explicit focus, selection, navigation, confirmation, context, cancellation
and clear-selection effects. Filtering preserves controllers and hidden selection;
source replacement and unmount permanently retire them. Initial selected keys seed
each source activation once. Application preferences intended to survive unmount
must be stored outside the component.

Row labels and optional `render_row`/`row_style` callbacks customize bounded
mounted content. `Row.item` exposes the original target/data/decoration kind;
selected and cursor flags describe independent state. Option semantics transfer
to the native row; decoration content keeps its own semantics and controls.
Themed defaults reserve cursor border space to keep geometry stable.

`before` and `after` contain ordinary views, including an optional real single-line
query editor referenced by View key. They are siblings of the actual native list,
inside its existing layout wrapper. Updating query/status/empty/footer content
does not replace the keyed viewport. Native composition and text editing stay
with the query editor. Eio search tasks must live outside transient row lifetimes.

Keep `List_rows.Layout` stable until membership/order, visibility, eligibility or
decoration classification changes. Point updates retain the source identity and
use persistent-map sharing, including while native commits coalesce. The component
does not scan selected records on each arrow or create Bonsai graphs for all
loaded items. Logical cursor changes use native bounded reveal; actual child focus
and composition keep the existing native retention rules.

An accepted metadata-only policy checkpoint advances native interaction epochs,
including when Stop/Wrap changes without changing the query. Already queued OCaml
callbacks also recheck their captured policy inside the current reducer. Controller
commands deliberately use current policy; obsolete/foreign targets are ignored.
Neither controller capture nor an action requires retaining an old payload snapshot.

## Eio search and fetched results

`Gpuio_eio.List_search` owns a loaded collection and one current source/query
request in an application/window scope. Create it outside Incremental evaluation
with an explicit monotonic clock and Eio producer. Debounce is cancellable; new
queries, refresh, structural edits and independent source changes cancel old work
and suppress queued completions. Results publish on the UI domain through `value`
and optional `on_change`, never from layout or a producer's foreign domain.

`Page` contains upserts and the explicit visible order. Fetched rows append to the
same source; existing row membership and selected hidden records survive. There
is no automatic eviction. Invalid duplicates, unknown visible keys or loaded-row
capacity violations reject the whole result. Applications explicitly edit the
source to evict records. Payload byte limits remain application-owned; the search
controller bounds retained record count and result metadata.

Point edits with `refresh=false` explicitly declare that matching/order is
unchanged. They preserve query/visibility metadata and do not restart work. An
older result cannot overwrite these newer values. Structural edits always restart
and fence requests even with that flag; hidden or removed/reincarnated targets
follow the collection contract. Use ordinary refresh for searchable field edits.

Snapshots distinguish ready, debouncing, loading, failed, cancelled and closed,
and mark retained previous results stale. Feed the snapshot epoch/busy flags into
`Selectable_list.Interaction`; disable list input while stale unless previous-query
interaction is an intentional application policy. Native query editing remains
enabled. Feed committed editor snapshots only, withholding composing text.
Epoch-checked retry/cancel and source-checked query events prevent delayed controls
from affecting unrelated work. Closing the owner or parent scope cancels work and
releases cleanup registrations; transient row eviction has no effect on search.
