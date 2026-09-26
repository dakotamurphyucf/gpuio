# Read-only data tables (OCH-39)

Status: column-schema foundation implemented; native adapter evaluation in progress.
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
editable grid transactions. The final public widget, resource API and paired
wire contracts remain to be implemented; these paragraphs constrain them rather
than imply their existence.

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

The production choice between the styled dependency and a provenanced extraction
remains open pending the retained-data delegate and native ownership experiments.
An equivalent implementation cannot be justified merely by the macro lookup
failure, because the isolated patch resolves it. Any extraction must preserve
provenance and acceptance scope; any full reuse must isolate additional theme,
initialization and dependency requirements.

## Remaining acceptance

The model tests are one foundation, not a replacement for these gates:

- Compile the actual selected production adapter against both platform targets.
- Deliver revisioned in-memory and Eio paged resources, bounded native cell
  descriptions and queues, and stale-result/cancellation handling.
- Integrate public Core/Bonsai views, native column interaction, stable selection,
  keyboard, copy, context actions, accessibility and lifecycle behavior.
- Exercise 100,000 logical rows with measured active/cache bounds and full
  traversal/revisit, horizontal and vertical behavior, resize/reorder/sort during
  paging, removed selected rows, Unicode copy, focus, empty/error and teardown.
- Add a public example and showcase in OCH-46, then consolidated local/hosted
  macOS and Linux checks and merge. Linux GUI acceptance remains OCH-17.
