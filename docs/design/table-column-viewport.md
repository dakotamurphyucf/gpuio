# Managed table column observations

OCH-41 implementation contract. See the [local validation
evidence](../evidence/table-column-viewport-och41.md) for current platform coverage.

The public shape is `Table.Column_viewport.t`, an immutable snapshot whose
`columns` are ordered stable column IDs. Each entry reports its pin and whether
its entire **horizontal** extent fits the viewport. An entry with any positive
horizontal intersection is included; touching an edge alone is insufficient.
Pinned columns are included and can themselves be partially clipped. Row-header
gutters, group headers, spacer/filler columns and overscan are excluded.

This describes horizontal column bands, not text visibility or occlusion by
another window, popover, scrollbar or child. Clipping the header vertically while
leaving the body visible does not remove the columns. A completely clipped or
zero-area table yields an empty snapshot. Empty data still has column geometry;
a single scrolling column and an all-pinned schema are ordinary cases.

Native measurement uses current laid-out pinned and scrolling header panes,
native widths/order (including optimistic resize/reorder), current horizontal
offset, and the table/ancestor/window clip. It runs after descendant prepaint.
The existing `TableVisibleRange` is a rendering optimization: it includes buffer
columns, omits pinned columns, and can retain stale zero/single-column ranges.
It is not the public observation. Layout invalidation clears the new snapshot
until another matching native layout completes. No synchronous OCaml call occurs.

Public interface:

```ocaml
module Column_viewport : sig
  module Column : sig
    type t
    val id : t -> Table_column.Id.t
    val pin : t -> Table_column.Pin.t
    val fully_visible : t -> bool
  end
  type t
  val columns : t -> Column.t list
end
```

The Core managed-table constructor accepts an optional `on_column_viewport` callback;
Bonsai exposes `Output.column_viewport : _ t -> Column_viewport.t option`.
`None` means no observation for the current source/query/configuration, distinct
from an observed empty viewport. Appended Event75 preserves previous wire layouts.
Native admission validates the current owner, schema/query, bounded unique column
IDs and matching pins. Core delivery requires the current transaction revision;
Bonsai additionally fences captured source/query/configuration callbacks.
The bounded mailbox may replace
adjacent observations for the same owner/generation, preserving input barriers.
The Host publishes after painting the measured table, when column membership,
pin/full-visibility flags or the transaction revision change. Pixel movement
within the same partial-column bands is silent. This reobserves accepted
configuration updates without continuous idle polling. A snapshot describes the
latest rendered layout, not a live OS query. A hidden/minimized window or an
unmounted subtree need not produce a final empty observation; source/query/config
changes invalidate the Bonsai snapshot, and unmount retires its owner.

All active-row cells continue to count against the existing cell budget,
independently of horizontal visibility. This API does not change virtualization,
mount lifetimes, source ownership or allow a native delegate callback into OCaml.
