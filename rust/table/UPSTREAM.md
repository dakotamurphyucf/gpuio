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

Before host integration, positional double-click/context/column events and
callbacks captured by older layouts need stable identity or generation fences.
Viewport anchors, resize reconciliation, protocol/public widgets, accessibility,
and full OCH-39 acceptance remain. No table capability is advertised yet.
