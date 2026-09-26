# Read-only data tables (OCH-39)

Status: column/data/paging foundations and initial extracted native adapter implemented.
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
editable grid transactions. The public widget and paired wire contracts remain
to be implemented. The Core and Eio resources below are implemented, but are not
yet connected to the native table or a Bonsai table component.

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
Before integration, remaining positional events and callbacks captured by old
layouts need stable identity or generation validation. Visual viewport anchors,
resize reconciliation and retained cell-cache policy still require implementation.

## Remaining acceptance

The model tests are one foundation, not a replacement for these gates:

- Compile the actual selected production adapter against both platform targets.
- Connect the tested in-memory/Eio resources to bounded native cell descriptions,
  viewport demand, query generations and stable selection/anchor reconciliation.
- Integrate public Core/Bonsai views, native column interaction, stable selection,
  keyboard, copy, context actions, accessibility and lifecycle behavior.
- Exercise 100,000 logical rows with measured active/cache bounds and full
  traversal/revisit, horizontal and vertical behavior, resize/reorder/sort during
  paging, removed selected rows, Unicode copy, focus, empty/error and teardown.
- Add a public example and showcase in OCH-46, then consolidated local/hosted
  macOS and Linux checks and merge. Linux GUI acceptance remains OCH-17.
