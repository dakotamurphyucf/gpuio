# Table adapter provenance

The implementation is adapted from Apache-2.0 gpui-kit commit
`84f57fdfcb4910623fb0bb7f795b077e249f9271`, `crates/component/src/`.
Original copyright remains with its contributors; see LICENSE.

Original input SHA-256:

- `table/column.rs`: `93718c19df34de48a60f587a6172e2dbf66adc527fce0e51b7c71bb9e2bb4be0`
- `table/data_table.rs`: `e6944194a27b53e1fe267d25ec3b7e73e7eea90e9b0d563e90c72360363b9172`
- `table/delegate.rs`: `4ca43e3cde812ec8f80287374598de4ce49838054050fc2241208e0b41270fce`
- `table/state.rs`: `cbf9c42fb0e8f4b2a2aecac24a6a4197ac28ef7d85b6b88cd1b80d4408f838e8`
- `sizing.rs`: `40bb45aca760df0efd7e79a0f81c3cee321218bde0d6912a12bc0e096083ef48`

GPUIO changes: per-instance appearance replaces global styled theme; reuse
existing gpui-base scrolling/actions/style helpers; render sort indicators without
external icon assets; context actions use emitted events for the host menu; default
empty/loading elements are overridable retained native delegates. Structural
Table and unrelated styled components are not imported.

Selection now uses retained row-lifetime keys and column keys. Refresh quietly
resolves those keys after source/schema changes and clears absent targets;
explicit replacement rejects invalid targets atomically without scrolling or
emitting user-selection events. Column movement also remaps selection.

This is an internal adapter, not a compatibility promise for gpui-component.
The macOS native test samples 100,000 rows and 64 columns, tests keyed selection
through reorder/removal, and checks that closing the window releases its table
entity. It does not establish full-history cache bounds or input acceptance.

All emitted events now capture stable row/column keys. Sort emits an application
request; the adapter never sorts the loaded subset. A delegate validates and
updates retained column order atomically before the keyed move event is emitted.
Source refresh retires old row listeners; schema replacement and accepted column
moves additionally retire column drag payloads. Unchanged column descriptions
preserve optimistic width/sort state and column gestures through row updates.
Deferred cell/row generation, header bounds and page demand check layout identity.

Native dispatch regression covers fresh clicks, double/context clicks, sort,
resize and reorder, and rejects stale frame clicks and drags spanning refresh.
`update_source` preserves keyed vertical/horizontal pixel anchors through native
source updates, with nearest-position fallback on removal and keyed explicit
command precedence. Native tests exercise reversal, prepend, removal and empty
sources, and row arrivals during active column gestures. Full Eio paging/query
reconciliation, protocol/public widgets, accessibility and OCH-39 acceptance
remain. No table capability is advertised.

The retained native host now uses this adapter through a local crate dependency.
Additional adaptation: per-delegate live input eligibility, keyboard focus checks,
a native event hook before deferred subscribers (capturing host route generations),
and anchor preservation across configured row-height changes. The hook only queues
native transport input; no synchronous OCaml call is permitted. The host's actual
painted-body viewport observation handles empty/single-row demand independently of
the upstream visible-range measurement callback. Public widgets and full native
acceptance remain separate from this extraction/integration evidence.

GPUIO input adaptation now focuses pointer selections, leaves Tab to the host,
and binds Enter/context/Copy to keyed intents. `ActivatedRow`/`ActivatedCell`
unify double-click and keyboard activation. Copy uses a retained-only delegate
hook and a bounded quoted-TSV encoder; missing complete selections preserve the
clipboard. Exact table focus is required so embedded editors retain their keys.
See the host acceptance ledger for native clipboard/command policy coverage.

The host can opt into inherited root text refinements with `inherit_text_style`.
Its outer box owns border/radius/background while internal header/body layers
stay transparent; this avoids covering gradients or applying alpha twice. The
standalone adapter defaults remain unchanged. Host GPU tests cover geometry,
clipping, root state precedence and preserved pinned-column painting.

Selection fills now paint behind row/cell content; transparent outlines remain
above it. Actual GPU tests caught opaque extracted overlays hiding selected text
and now require that text to stay visible for both cell and whole-row selection.

GPUIO now gives painted rows/cells/headers logical accessibility positions,
selection and idempotent selection/focus actions. A native focus-owning Group
supports selected-descendant accessibility focus; cell/header identities use
stable column keys. Retained accessible cell values are supplied by an optional
delegate hook, without a host-language callback. Sorting has a separate accessible
button. The shared Cocoa adapter changes are tracked separately under
`vendor/accesskit-macos/table-state.patch` with original adapter provenance.

Current acceptance supersedes the early checkpoint limitations above: the public
presenter, Eio paging, native AppKit input/semantics and full-history checks now
pass locally. The complete scope is mapped in
`docs/evidence/data-tables-och39-audit.md`; managed tables negotiate their own bit40.
Hosted macOS/Linux checks and merge remain pending. This does not expand the
upstream compatibility claim beyond the selected extraction.

The chat integration exposed a held-pointer accessibility edge: a selected row's
retained handle can take direct focus on mouse down. Composite active-descendant
semantics are now emitted only while the table container owns focus, preventing
an invalid node from reporting itself as its own active descendant. The actual
host AX regression draws the intermediate held-down frame, checks direct row
focus, then releases and restores ordinary cell focus.

The optional `DataTable::scrollbar_presentation` hook now supplies native-only
per-axis elements while retaining TableState's original scroll handles. It is
scoped to one table, leaves the default Base renderer intact when absent, and
keeps visibility flags, header exclusion and pinned-column positioning in the
adapter. The GPUIO Host injects weak presentation owners; this hook never calls
OCaml. Shared geometry reserves an overflowing sibling's corner without changing
semantic viewport lengths or offsets. See
`docs/evidence/scrollbar-table-och41.md` for TestPlatform coverage; physical
qualification of the custom presentation remains separate.

OCH-41 managed table behavior now exposes row headers, Stop/Wrap and per-header
selection. Adapted left/right and Home/End navigation skip nonselectable headers;
the user column-selection setter also checks global/per-header eligibility.
Search is bounded by retained column count, including an empty eligible set.
Cell navigation remains independent. This fixes an extracted path that could
select a forbidden header even though pointer and accessibility entry points
rejected it. See `docs/evidence/table-behavior-och41.md` for a reproduced Host
regression and the follow-up tests; physical desktop qualification is separate.

OCH-41 presentation uses retained per-table colors, stripe mode and native column
padding. The header surface now paints once: pinned/scrolling header containers
and the header's row-gutter cell no longer repaint its background. This preserves
opaque results while avoiding repeated alpha compositing. Body row-header gutters
retain their own background. Host delegate styling overrides explicit header
foreground without disabling root font inheritance. These changes do not add a
synchronous OCaml renderer callback or a dependency on the styled upstream crate.

`TableState::column_viewport` now measures horizontal column bands independently
of buffered rendering ranges. It uses laid-out pinned/scrolling header panes,
current native widths/order/offset and table/ancestor/window clipping, including
empty data, single columns and all-pinned schemas. Layout invalidation retires
the snapshot. The root measurement canvas now has explicit top/left positioning
within a relative root; its former static position was below the table content.
The Host publishes asynchronously after painting; no delegate crosses into OCaml.
See `docs/design/table-column-viewport.md` and its local validation evidence.

The retained Host can now override `TableDelegate::render_group_header` with the
zero-based group level and absolute leaf-column range. The default method calls
`render_group_th`, preserving existing delegates. Pinned and scrolling header
panes supply the same coordinates. GPUIO resolves exact canonical member IDs
against its retained schema and renders already-submitted header Views; repeated
labels are never identities and no layout callback enters OCaml. This extension
leaves native header sizes, sort affordances and column gesture ownership intact.

Scoped row presentation adds `RowPresentation` and default `finish_row` after
native selection decoration. Native hover is carried as a refinement into that
hook and installed once; hosts can compose state styles without registering a
second GPUI hover handler. The default preserves normal native hover. Pointer
policy gates hover registration. The Host applies checked base paint/font fields
through `render_tr`, with explicit selected/focused/hover/pressed/disabled layers
in the final hook; native row geometry, outlines and accessibility stay owned by
the table. Filler rows do not receive application row presentation.

The optional test observation wrapper is applied after `finish_row`, then receives
the final row accessibility decoration. Delegates keep the same concrete
`Stateful<Div>` input/output contract with or without `gpui-base/test-support`;
turning on observation must not change the delegate's row type.

The horizontal column-visibility observer now converts its child-canvas bounds
back to the unscrolled header-pane origin before applying the column offset.
`ElementExt::on_prepaint` observes a child that moves with the pane's contents;
using that translated rectangle as the viewport applied scrolling twice and
reported columns as clipped/off-window on longer horizontal traversals. A real
Host/TestPlatform regression covers all 64 column commands at the reference
viewport width, including the previous column-17 failure, and drains the bounded
observation queue as an actual client does.
