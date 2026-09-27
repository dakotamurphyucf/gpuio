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
