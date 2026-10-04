# Structural tables

OCH-41 implementation contract; [local validation passes](../evidence/structural-tables-och41.md).
Physical macOS qualification remains open.
The pinned Base/styled structural table is ordinary view composition, separate
from managed `Table_data`/Bonsai tables. It supports named tables, header/body/footer
sections, rows, rich header/data cells, column spans and visible captions.

`Gpuio.Table_view` composes an ordinary keyed View tree from opaque `Cell`,
`Row` and `Section` descriptions. The root takes an explicit accessible label and
column count, optional header/footer/caption, and body sections. Captions remain
visible descriptions, never a substitute for the accessible name. Section/row/cell
keys retain embedded native controls through content changes and reordering.

Rows use shared equal-fraction grid columns. A cell's positive column span determines
its actual grid placement and accessibility column span; each row must cover the
declared columns exactly. This avoids the pinned styled layer's ordinal cell-index
shortcut after a spanning cell. Row indices are zero-based across all sections,
including header and footer rows. Column indices are zero-based logical tracks.
The pinned structural API exposes column spans, not row spans; this API does not
pretend a semantic row span changes layout.

All nodes remain mounted: use managed tables for large/remote datasets. Public
composition bounds are 1..64 columns, at most 4,096 rows, 4,096 body sections and
16,384 cells; ordinary
View node/byte budgets additionally apply, including arbitrary child content.
Construction validates sibling keys and row coverage before allocating wrappers.
An empty body is valid and does not manufacture a data row.

Cell content accepts ordinary Views, including buttons/editors and nested tables.
The table introduces no selection model, Tab stops, arrow-key controller, timers,
native owner or synchronous foreign callback. Embedded controls retain their usual
input behavior. Styles expose background/borders/padding/typography/alignment;
the builder owns row grid tracks and cell placement so semantic and visual column
positions agree. Custom decorations must not disguise the declared structure.

The accessibility vocabulary appends typed Table, Row_group, Table_row, Table_cell,
Column_header, Row_header and Caption roles to the existing role encoding. Checked
count/index/span metadata is independent of styles and maps directly to AccessKit.
Existing role tags and the Set_accessibility operation remain unchanged. The
low-level vocabulary describes supplied geometry; callers composing arbitrary
Views themselves own hierarchy/count consistency. `Table_view` checks its complete
structure and derives those properties. Platform behavior and VoiceOver acceptance
must be recorded separately from protocol/model/layout tests.

For example, a merged summary row uses an actual two-track cell:

```ocaml
let summary text =
  let open Core.Or_error.Let_syntax in
  let key = Gpuio.Key.of_string_exn in
  let module T = Gpuio.Table_view in
  let%bind cell =
    T.Cell.create ~key:(key "summary") ~span:2 [ Gpuio.View.text text ]
  in
  let%bind row = T.Row.create ~key:(key "summary") [ cell ] in
  let%bind body = T.Section.create ~key:(key "results") [ row ] in
  T.create ~columns:2 ~label:"Run summary" [ body ]
```

`Cell.Kind.Column_header` and `Row_header` distinguish header cells from ordinary
data. The gallery's Structural table demonstrates two header rows, body row headers,
merged footer content, caption, row reversal and native buttons. A pure alias is
also available as `Gpuio_bonsai.Table_view`; it introduces no separate Bonsai state.
