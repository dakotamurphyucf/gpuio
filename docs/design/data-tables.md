# Read-only data tables (OCH-39)

Status: public Core/Bonsai/Eio tables, native admission and retained host rendering
are implemented.
See the evidence ledger for local acceptance; physical input, remaining paging
integration, the chat showcase and consolidated hosted gates remain.
This document does not advertise a table capability or claim ticket acceptance.

## Ownership and scope

The application owns stable row keys, column keys, data, filtering and sorting.
Native GPUI owns layout, scrolling, pointer capture and immediate interaction
feedback. A Rust delegate reads retained cell descriptions and copy text. It must
not call OCaml to render, format, compare, sort or fetch data. Missing descriptions
produce placeholders and bounded, asynchronous demand notifications.

Sort/filter changes establish a new query revision. Producer cancellation and
revision validation both matter: already queued old results must not become new
data. Column resizing/reordering does not itself change the data query. Stable
row/cell selection and viewport anchors must survive reorder without inheriting
an unrelated row's old numeric position. Native optimistic feedback must be
reconciled with application acceptance and explicit replacement commands.

The ticket includes row/cell selection, keyboard navigation, Unicode copy,
context actions, column resizing/reordering/left pinning/sort requests, grouped
headers, empty/loading/error states and bounded paging. It does not include
editable grid transactions. The public widget remains to be connected; paired wire descriptions, retained
admission and the native host renderer are implemented. The Core and Eio resources below are implemented, but are not
yet connected through a public Core/Bonsai table component.

## Column schema

`Table_column` is an immutable Core model with a distinct abstract `Id`, separate
from row identity. IDs are case-sensitive UTF-8, 1..256 bytes without NUL. Labels
are UTF-8, 1..4096 bytes without NUL. They are display data, never identity.

Widths are finite logical pixels. The invariant is
`20 <= min_width <= width <= max_width <= 16384`; defaults are 40/160/4096.
Programmatic `with_width` rejects invalid values. `Collection.resize` implements
a user proposal: the column must be resizable, and a finite proposed width is
clamped to its limits. Application code can change a non-resizable column.

Schemas support at most **64 columns**, **four additional header levels** and
**256 KiB of schema text**, including repeated group member IDs. These are
explicit initial support limits, not a claim of unbounded horizontal
virtualization. Left-pinned columns form a contiguous prefix. Pinning right is
not part of the evaluated upstream API. Empty schemas without groups are valid.

Groups contain stable column IDs. Each header level partitions the schema in
display order; a lower level refines the previous level. Groups do not straddle
the pinned/unpinned boundary. This avoids partial group headings silently changing
membership when their columns are moved or pinned. Adapters derive native spans
and widths from the validated current schema.

`Collection.move ~column ~before` moves a movable source before another column,
or to the end for `None`. Other columns retain their relative order. The operation
fails atomically if a pin partition or group would be split. Valid moves reorder
members and groups without changing keyed membership. A non-movable column
cannot be the source, but another source can move past it. Pin/group restructuring
is an explicit application schema replacement, not an implicit drag side effect.

## Application data and row lifetime

`Table_data` is an immutable source of application payloads with distinct typed
row IDs. It builds on `List_collection`: point updates take O(log n), preserve
the order snapshot and membership, and expose conservative changed-value
invalidation without walking every payload. Structural replacement/splice and
explicit application-supplied reorder take O(n log n). Reordering preserves
payload versions, so it does not invalidate every cell as a value change.

Each `create` establishes an independent source lineage. Revisions are local to
that lineage and cannot identify a source by themselves. `Row_ref` captures one
row's membership lifetime and source identity, without capturing its payload or
source snapshot. Updates/reorder preserve it; separate removal and reinsertion
of the same ID retire it. New additions in two immutable branches are distinct
even when their numeric revisions match. An adapter must also enforce its query
and mounted-handler generation; row membership is not a substitute for those.

Application data may contain up to 1,000,000 logical rows and 64 MiB of key bytes.
Key bytes are counted once per row; this is a logical admission limit, not a
measurement of OCaml heap use. Payloads and persistent data snapshots belong to
the application, independently of the bounded active cells/native cache. The
resource keeps no registry of removed keys. Dropping old snapshots permits old
payloads and membership metadata to be collected.

## Query and paged resource

`Table_paging` is the UI-domain-owned Core state machine. Its polymorphic query
value carries application sort/filter settings. Each request captures that value
at admission. Applications must use immutable query values; the framework does
not compare, serialize or execute the query. The producer/server supplies rows
in display order. There is no automatic sort of a partially loaded query.

There is one request at each of the Before/After boundaries, at most two current
requests. Pages contain at most 2,048 rows, cursors at most 4,096 opaque bytes,
and stored errors at most 4,096 valid UTF-8 bytes without NUL. Initial seed data
uses the larger source limits. Duplicate/oversized/invalid responses fail
atomically and require explicit retry. Empty pages must advance their cursor or
reach End. Obsolete responses are ignored before inspecting their contents.

`reset ~query data ~before ~after` atomically advances the query generation and
retires both current requests, even when reusing the same query value. Invalid
reset parameters leave old data and requests intact. A reordered source from the
same lineage preserves surviving row references, enabling keyed anchor/selection
reconciliation. A fresh source resets identity. Clearing rows and later loading
them again is a new membership lifetime; the future widget must specify any
pending-key preference policy explicitly instead of treating absence as a known
future return. Column resize/reorder and current-query payload updates do not
cancel pages. Appending is allowed only at a known latest boundary.

`Gpuio_eio.Table_paging` supplies the scoped implementation and reactive Bonsai
snapshot. Two lazily created reusable Eio workers bound producer concurrency.
Cancellation does not free a slot until the producer exits, including protected
cleanup, and the UI inbox accepts its completion. Behind occupied workers only
the latest request at each boundary waits. Workers sleep on streams while idle;
there is no polling. Query resets cancel old producer contexts and reject their
already queued results using the Core tokens. Explicit close cancels only this
controller; parent-scope cancellation closes it too. Application data remains
readable after close, while completion publication is retired.

## Upstream evaluation

The source baseline is gpui-kit
`84f57fdfcb4910623fb0bb7f795b077e249f9271`, specifically
`crates/component/src/table/{data_table,state,delegate,column}.rs`.
`table.rs` is the separate structural table wrapper. The GPUI baseline remains
Zed `a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b` and the existing vendored
`gpui-base`. Neither production dependency pin has changed.

The styled table has vertical uniform-list rendering and a horizontal virtual
list per row. It supports fixed-left columns and multiple header levels. Source
presence is insufficient evidence for bounded runtime work. Its selected row,
column and cell are numeric indices. Refresh rebuilds column groups from delegate
descriptions; selection setters emit events and scroll. An adapter must account
for these effects when reconciling application state, suppress feedback loops and
preserve stable anchors. `cell_text` and rendering are synchronous native delegate
methods, so only retained Rust data belongs behind them.

The isolated macOS compile probe substitutes the pinned GPUI crates and existing
base, without modifying the production manifests or lock. The full styled library
initially fails in seven chart derives (14 diagnostics): `IntoPlot` resolves
`gpui-kit` or the package name `gpui-pre`, while this project uses package `gpui`.
A one-line `.or_else(|_| crate_name("gpui"))` fallback in the scratch copy of
`component-macros/src/crate_path.rs` makes the library compile. This is evidence
for the default-feature library on macOS, not all optional features, Linux, or
native behavior. It does not require changing GPUI versions.

The selected integration is a provenanced extraction in `rust/table`, reusing
existing `gpui-base` virtualization, scrolling, actions and style helpers. Its
per-instance appearance avoids requiring the styled library's global theme,
assets and unrelated component initialization. This is an ownership/style choice,
not a claim that full reuse is incompatible: the isolated macro patch resolves
the compile probe. Source hashes and the Apache license are recorded in
`rust/table/UPSTREAM.md` and `LICENSE`. No third-party dependency version changed.

The extracted state retains row-lifetime and column keys for selection. Refresh
quietly remaps those keys or clears missing targets; explicit native selection
replacement rejects invalid targets atomically without scrolling or emitting a
user event. Layout indices remain internal. The host must issue distinct native
row keys for distinct membership lifetimes. This native foundation is not yet
connected to Core row references or mounted/query generations.

The local macOS test confirms sampled virtualization, selection remapping through
row/column reorder, removal handling and native entity release on window close.
All emitted native events capture stable row/column keys, including widths,
double/context clicks and column placement. Sort emits an application request;
it never sorts the loaded subset. Column moves require the retained delegate to
validate policy and update its order atomically before emitting the keyed move.

Refresh and accepted column movement retire the previous row-layout identity.
Old pointer handlers, deferred row/cell generation, header-bound writes and
deferred page-demand work check that identity. Column gestures use a separate
identity: unchanged column descriptions preserve native width/sort previews and
active resize/reorder through row updates. A changed schema retires those
gestures and applies its supplied widths. The host must still distinguish
content-only delivery from mapping/schema refresh. Native identities do not
replace mounted-handler and query generations at the wire boundary.
An explicit `reset_columns` reapplies retained schema even when its value is
unchanged, allowing the application to reject optimistic widths/sort indicators.
It retires column gestures and preserves keyed selection/scroll without event echo.

`TableState::update_source` captures scroll anchors before mutating retained
descriptions and restores after reconciliation, within one native entity update.
It preserves the top row's lifetime key and the first unpinned column's key,
including their intra-item pixel offsets. Missing anchors use the nearest old
position; native layout clamps at content edges. Empty sources reset scroll.
A pending row command follows its target key through reorder, overriding the
painted anchor; deleting that target cancels the command. New explicit scroll
commands run after the update and take precedence. Revealing an already pinned
column leaves the scrolling region unchanged.

These contracts pass native layout and pointer tests, including row arrival
during resize/reorder. Full Eio paging/query races and full-history bounded cache acceptance remain
after the initial retained host integration below.

## Paired bridge descriptions and public configuration

The pure Core `Table.Config` accepts typed `Table_column.Collection` values, an
accessible label and an optional accepted sort column/direction. Defaults are
32 logical pixels per fixed-height row, 64 pixels of overscan, at most 64 active
rows and 4,096 active cells. A smaller cell budget reduces the default row budget;
an explicitly incompatible row budget is rejected. Cells and rows are selectable
by default; column selection is opt-in. Configuration owns no row data, views,
callbacks or native handles. Immutable column/sort replacement validates the new
combination; removing a sorted column requires clearing its accepted sort first.

`Table_wire` and Rust `protocol::table` now define matching schema, config, cell,
selection, command and request data. `Table_column.Expert` converts schemas using
the same validated constructors as application code. Bridge schema revisions and
query generations belong to the mounted adapter, and logical row order retains
the existing list-order revision. Commands carry increasing serials and query
generation; native transaction admission validates these against the final live data.
Requests carry stable row IDs and column keys, never callbacks or borrowed data.

The hard description budget is **16,384 active cells**, with
`max_active_rows × column_count <= max_active_cells`. This counts all retained
cells in active rows even when horizontal painting visits fewer cells. Copy text
is a separate retained UTF-8 string, at most **65,536 bytes per cell**, without NUL;
empty text is valid. Counts do not replace byte limits: native admission must also
account for copy strings and schema metadata within the existing 1 MiB message
and 64 MiB retained-tree budgets. Native transaction admission now charges schema
containers/text and cell copy strings to that existing retained-tree quota.

Rust decoders bound counts and aggregate schema text before allocation, reject
invalid enum tags, nonfinite numbers, malformed UTF-8, truncation and trailing
bytes. Both languages match independent schema/cell/command/request byte
fixtures. The combined fixture covers every command and request tag. Validators
check pin boundaries, nested header refinement and cell budgets.
These payload types are connected to transaction/event envelopes and native
session admission and the host renderer. Public `View` constructors and the Bonsai
table component remain to be connected. Declaring a Copy request does not establish
clipboard behavior.

## Retained transaction and input contract

An internal `Virtual_list` node with `Set_table` metadata specializes the shared
logical row index and active row mapping. A separate native table renderer owns
its GPUI entity; it does not allocate ordinary variable-height list state. Its managed-list configuration must exactly match the table's
fixed row height, overscan, active-row limit and scrollbar setting, with
`Keep_position` anchoring. Table and tree input modes cannot share a root.

Each active row maps to an inert Container. Its children follow schema order,
with one inert Container per column, `Set_table_cell` metadata and exactly one
ordinary child View. Cell metadata fixes the wrapper's column identity; changing
that identity requires a replacement wrapper. The wrapper hierarchy is internal,
not an application API. Final structure and all dirty table ancestors are checked,
so a cell-only edit cannot bypass row/schema ownership validation. Unmounted cell
copy text is released without deleting the logical row index.

A schema revision cannot decrease. Changed columns, groups or accepted sort must
advance it; increasing it with equal values is allowed. Other configuration
changes, such as geometry or disabled state, do not require a schema revision.
Query generation cannot decrease. Advancing it requires replacing the mounted
handler even when row order stays unchanged: ordinary viewport envelopes carry
that handler and therefore cannot cross a query reset. These checks apply to the
final transaction snapshot, independently of Bind/config operation order.

`Table_command` actions are ordered and ephemeral. Each serial must strictly
increase over earlier commands for the same node lifetime, including earlier
commands in the transaction. Query generation, logical row membership, column
membership and selection policy are checked against the final snapshot. A scroll
offset must be less than the configured row height. A failed action rolls back
all serials and other transaction changes. Accepted actions appear in
`Applied.tables`; no historical command retains a row or prevents later removal.
Programmatic commands can operate while user input is disabled.

`Table_input` captures schema and query generations alongside window, node,
handler and tree revision. The native session rejects obsolete routes and checks
logical row membership, live columns, selection mode, resize limits and locks,
move/group/pin policy, sortable flags and disabled state. Offscreen logical rows
are valid targets even when their cell descriptions are absent. Requests remain
ordered in the bounded mailbox; resize column strings count toward its byte
budget and message-sized drains. The OCaml event envelope validates payloads,
but application callback dispatch still awaits the public mounted adapter.

The new operation/event tags append to the under-development protocol without
changing existing tags. No table capability is advertised from admission alone.
Rendering, command execution and native focus pins are now integrated. Clipboard
effects still require integration; broader native
keyboard/accessibility/cache/lifecycle acceptance remains.

## Retained native host

The host keeps one native table entity per admitted root. Its delegate shares the
current row index, config and bounded active child-node references. Cell rendering
uses a weak parent View reference to render retained Rust descriptions. No row
formatter, comparison, producer or OCaml callback runs during GPUI layout/paint.
Missing cells paint placeholders until the UI delivers the requested rows.

A frame wrapper observes actual UniformList body bounds and scroll offset after
prepaint. It clips to the current viewport, reserves focus/composition pins, then
requests visible and overscan rows within the active budget. This handles empty
and single-row data without treating measurement callbacks as visible demand.
The ordinary viewport envelope carries the current handler and logical order.
The table's root focus target is recorded during paint, after the shared focus
manager begins the frame. Selected rows with table focus and focused materialized
rows join the existing live retention checks.

Accepted source or configuration changes preserve keyed anchors through
`update_source_with_size`, including a change to fixed row height. Cell content
updates replace bounded mappings and repaint without resetting column gestures.
Schema/query rebinding and explicit reset commands reapply accepted columns.
Admitted selection, reveal, offset, column, end and reset commands execute after
tree synchronization. Explicit offset commands clear older deferred scrolls.

The adapter invokes a native delegate event hook at event creation, before GPUI's
deferred subscribers. The hook validates and enqueues the captured route; a later
query update cannot relabel already-created input. Existing GPUI subscribers
still receive keyed events. Live input eligibility guards pointer changes and
keyboard navigation; navigation requires the table itself to own focus, leaving
child-control keys with their controls. Pointer-event styles are resolved against
the current retained ancestry. `Context Empty` means clearing the context target,
including the upstream reset emitted after row selection; it is not an instruction
to open an empty context menu.

Root background/foreground, border, hover, focus-highlight and radius styles have
native table equivalents. This is initial presentation integration, not a claim
that every style refinement or accessibility contract has passed acceptance.
The local host test drives actual GPUI layout/paint in a background window; it is
not foreground keyboard, clipboard or IME acceptance.

## Core View and callback adapter

`Table.Cell` holds validated copy text and a typed column ID, separately from the
child View. `Table.Selection`, `Request` and `Target` parameterize row identity;
column identities are always `Table_column.Id`. `Table.Command` adds a positive
serial and query generation. The managed adapter uses these types with bridge
keys internally; application controllers will capture `Table_data.Row_ref`
membership instead of accepting a reused display position.

`View.Expert.managed_table` constructs inert keyed row/cell wrappers and admits
exactly one cell per column, in accepted schema order. It checks active-row and
active-cell limits before reconciliation, sharing managed-list order, viewport
and retention infrastructure. The callback delivers typed native proposals.
Cell copy text can change without replacing the wrapper; changing its column
identity requires a new node. Replacing a table with an ordinary list also
requires a new node, even when their outer kind/key would otherwise match.

Reconciliation generates schema revisions from accepted schema/sort changes.
Query generations must not decrease during a mount; changing one rotates the
handler even when row order is unchanged. The callback binding stores the
accepted schema/query revisions. Delivery rejects stale handlers, removed or
reincarnated row IDs, obsolete schema/query snapshots, disabled input and invalid
current column/selection policies. A column reorder retains keyed cell nodes.
Streaming text/copy metadata updates do not resend logical order or schema.

Command batches contain at most 64 ordered requests. An identical retained batch
is not resent. New batches must strictly advance the mounted serial, including
after an omitted batch or query reset, and use the current query generation.
Targets resolve against the final logical order, including unmaterialized rows;
selection mode and column existence are checked before submission. Offsets must
be smaller than the configured row height. Failed preparation publishes no state
and consumes neither schema revisions nor command serials. Native admission
independently validates the resulting transaction.

This Core adapter is consumed by the public `Gpuio_bonsai.Table` presenter below.
The Expert View constructor is not required for ordinary application code.

## Public Bonsai presenter and Eio paging

`Gpuio_bonsai.Table.component` accepts a `Table_data` source and reactive
`Table.Config`; `Table.paged` accepts a table-pager snapshot and generation-checked
controls. Each active row has a keyed computation, and each column inside it has
its own default-reset lifetime. `render_cell` returns a validated `Table.Cell`
with independent copy text and an ordinary View. Only the requested/pinned rows
create cells; all schema columns count against the cell budget, regardless of
horizontal paint virtualization. Persistent preferences and I/O jobs belong
outside these transient computations.

Table source/order metadata now exposes opaque comparable lineage and membership
identities without row payloads. Point updates share that snapshot. Native row
keys include the membership incarnation, so a removal/reinsertion coalesced before
native acceptance still creates a new identity. A retained controller or delayed
native effect holds no source payload snapshot. `Output.target` captures a
membership; the output itself deliberately retains its current source snapshot.

The outer component scope is keyed by source lineage. A fresh source resets the
widget and its native root. A same-source query-generation increase keeps that
root and surviving anchors/selection, but resets transient cell computations and
rejects older-query effects. Decreasing generations within a mount is invalid.
Selection repairs after row/column removal or a selection-mode change. This
component never sorts data on behalf of the application: sort/resize/reorder and
other native requests are delivered as typed proposals for application handling.

Controller batches contain at most 64 commands and replace an earlier pending,
undisplayed batch. Group selection/reveal when both must execute. Serials remain
monotonic; an after-display acknowledgment clears only its own displayed batch.
The model distinguishes displayed/native selection from a pending selection, so
superseding a batch cannot report a selection that never reached native code.
A newer native selection supersedes an older pending batch, and sequence checks
prevent delayed display acknowledgment from overwriting that observation.

Viewport validity follows query, logical order and config, not row payload
revision. This preserves useful geometry after a point update or empty
cursor-advancing page. A nonempty page waits for new native layout before loading
another boundary; empty advancing pages can continue without an otherwise
unnecessary frame. Automatic demand requests only Ready boundaries. Failure
requires explicit retry; leaving the viewport does not cancel application work.
`Gpuio_eio.Table_paging.controls` rechecks current generation, scope and closure
when an effect executes. Resetting or closing the pager cancels its producers;
resetting a widget alone does not own that application I/O lifetime.

The [Table Lab](../../examples/table/README.md) exercises the public path. Native
keyboard/copy/context/accessibility and the rest of the full OCH-39 acceptance
remain required; the public presenter is not a claim that those gates passed.

## Native table input and clipboard

Pointer selection of a row, header or cell gives the table keyboard focus.
Arrows, Home/End and PageUp/PageDown navigate the native selection. Tab and
Shift-Tab use shared window traversal; the table does not trap them for column
movement. Enter activates a selected row/cell; Shift-F10 or the Menu key sends
its keyed context request. Column selection supports context and copy, without
inventing a row activation. Empty context means clearing the context target.

Native child controls own their pointer and keyboard input. The shared pointer
barrier used by managed trees also protects table children, without intercepting
wheel propagation. Table keyboard handlers require exact table focus. Disabling
or hiding a table gates its retained descendants as well as the root, including
callbacks installed by the previous painted frame.

Primary-C and the existing native Copy command use retained `Table.Cell.copy_text`.
The host never invokes OCaml to format or fetch clipboard content. A cell copies
its exact UTF-8 text. Rows follow displayed column order; columns follow logical
row order. Multi-cell exports use TSV, quoting fields containing tabs, line breaks
or quotes and doubling internal quotes. There is no trailing newline. Output is
bounded to 1 MiB including separators and escaping.

Copy requires the **complete** selection to be retained. A partly materialized
column, missing cell, or oversized output leaves the OS clipboard unchanged; it
never exports a silently truncated viewport. A keyed Copy intent is queued in
both success and unavailable cases; it is not a clipboard-write acknowledgement.
Applications may implement larger exports using their owned data and I/O scope.
There is no hidden synchronous fetch or unbounded clipboard cache.

The shared command manager remembers one native command target per window.
Table focus replaces the previous editor target; an embedded editor takes priority
when it owns focus. A Copy toolbar/menu command can restore that remembered table
focus. Cut, Paste, SelectAll, Undo and Redo remain unavailable for the read-only
table itself. This does not change embedded editors' own commands.

Local tests cover GPUI pointer/key dispatch, actual OS clipboard contents,
Tab exit, toolbar command routing, retained child editors and current hidden/
disabled policy. Separate AppKit keyboard/input-client and public pointer/AX
scenarios now pass as recorded in the evidence ledger.

## Public table styling and mount identity

The public Bonsai table now exposes one native table root. The caller's `key`
and complete `style` apply to that root, so padding, border, radius and opacity
are not duplicated across a synthetic layout wrapper. A separate expert-only
`source_key` participates in reconciliation compatibility. Fresh data lineages
replace the native mount even when the caller's sibling key is unchanged; style,
payload and query updates retain it. This is OCaml reconciliation metadata, not
a new wire field. Caller keys retain their full 256-byte limit.

The native host paints the table surface once. Its internal body and header
layers are transparent over that surface, so solid colors, alpha and gradients
remain visible. The default host surface is uniform across header and body;
header separators and selection/hover feedback remain native. The standalone
extracted adapter keeps its original default header/body appearance.

Root text refinements reach header and body cells, unless a child supplies its
own text style. Focused, hovered, pressed and disabled root refinements use the
ordinary style precedence; focus refers to the table itself, preserving embedded
editor ownership. The Selected state's solid background colors the native
row/cell selection. Selection fills paint behind cell content; only the outline
is overlaid, preserving readable text. Focused and Hovered backgrounds style the root surface rather
than being repurposed as selection colors. Base border color also colors native
dividers. Native row hover feedback retains its default appearance. No editable
grid behavior is implied by ordinary child View composition.

The Table Lab supplies explicit surface/text/border colors and checks a light/
dark update while selection and a keyed pixel anchor are retained. GPU tests
separately verify surface composition, clipping, pinned-column paint, inherited
text and state precedence. Full table accessibility and remaining paging/history
acceptance still apply.

## Native table accessibility

The outer native Table exposes the logical data-row and column counts. Only
painted rows/cells and visible headers become accessibility nodes; an offscreen
100,000-row history is not materialized for accessibility. Data indices are
zero-based and exclude header bands. Native Row, Cell and ColumnHeader nodes
carry current logical positions, and cells use column names plus the retained
`copy_text` as their accessible value. Unavailable data has no value; it is not
reported as an empty string. Embedded controls retain their own semantics.

Accessibility Press and focus select the addressed row/cell/column, with native
keyboard focus on the table and the selected descendant reported as focused.
Repeated cell Press remains cell selection; it does not use pointer reselect
escalation or activate application data. Activation stays Enter/double-click.
Desired selection setters preserve order before the next paint; deselecting an
unselected target leaves another selection alone. Row selection also clears the
previous context target, consistent with ordinary input. Sort has a separately
labelled header button and optimistic sort metadata, subject to application
acceptance/reset. Header/cell element identities use stable column keys.

Actions consult current layout identity and live input policy. PointerEvents
suppression does not disable accessibility. Hidden/disabled tables are inert and
leave the accessibility tree; queued old-object requests cannot change current
application state. The renderer's focus-owning Group supplies the ancestor needed
for GPUI's active-descendant semantics without taking focus from child editors.

The vendored AccessKit macOS adapter has a fourth scoped, reproducible patch for
counts, positions, mounted-row enumeration and opt-in desired table selection.
See its [provenance](../../vendor/accesskit-macos/GPUIO.md). Local AppKit checks
exercise these getters/actions, including Unicode values and logical positions
after a 50,000-row jump. These checks do not claim VoiceOver speech, physical
keyboard/IME acceptance or Linux graphical accessibility acceptance.

## Remaining acceptance

Local evidence now covers the native adapter, public presenter/pager, 100,000-row
full traversal/revisit, bounded retention, horizontal/vertical behavior, native
input/copy/accessibility, context flows and cleanup. Final work remains:

- Audit every live OCH-39 requirement against the complete evidence and settle
  capability advertisement; do not infer ticket completion from a single suite.
- Add the polished chat showcase in OCH-46, then consolidated local/hosted
  macOS and Linux checks and merge, including both selected adapter build paths.
  Linux GUI acceptance remains OCH-17.

## Application context flows

The public Table Lab demonstrates `Request.Activate` and `Request.Context` using
an application-owned inspection dialog. The table provides the stable target;
applications choose their action UI and effects. Right-click does not implicitly
replace ordinary selection. Query generation and row membership guard delayed
actions; current payloads are looked up at presentation/invocation rather than
retaining an obsolete source snapshot. Reveal uses the existing controller batch
to select/reveal the result and native dialog dismissal restores table focus.
CoreGraphics keyboard and public pointer/AX scenarios now verify this path locally
on macOS; the evidence ledger distinguishes them from GPUI-dispatched tests.
