# Managed table presentation

`Table.Appearance` describes native striping, named part colors and shared or
per-column header/body padding. It is an immutable value supplied through
`Table.Config.create ~appearance` or `Config.with_appearance`. Defaults preserve
existing unstriped rendering, native padding and root-style-derived colors.
Ordinary root View styles continue to own the outer surface, border, radius and
inherited text. Color overrides affect their named internal parts, including the
header's grouped rows and row-header gutters.

Parts name native roles: header background/foreground, stripe and hover fills,
selected fill/border, row and column borders, sort hover/pressed fill and foreground,
drag border and context border. Omitted colors inherit;
transparent values explicitly override. All colors resolve through the submission
theme, including colors not currently visible. Missing tokens reject the whole
candidate. Duplicate parts reject; there are thirteen bounded parts.

`Appearance.Padding` has four finite edges in 0..4096 logical pixels. Padding
changes content space, never configured row height or column width; oversized
padding can clip content. Per-column padding replaces shared padding, which in
turn replaces the native default (4 vertical, 8 horizontal). The description
allows at most 64 unique column overrides, and Config requires their IDs in the
current schema. Clear obsolete padding references before removing columns.

Striping uses logical row index, including native filler rows below a short table.
The native table paints row backgrounds across pinned areas and empty row space;
this is distinct from painting each mounted cell's child View. Header background
is painted once by the header surface; its clipped pinned/scrolling regions inherit
it instead of layering the same translucent fill twice.

Appended Op112 `Set_table_appearance` carries optional resolved metadata and leaves
the original Config, Column and Behavior wire representations unchanged. Clearing
it restores defaults. Native admission checks ownership, bounds, schema membership
and retained-byte quota against the final transaction snapshot. Shared/per-column
padding changes require a fresh schema revision to retire geometry-dependent input.
Pure colors and stripe changes retain that revision and transient column geometry.
Both retain the native table, keyed selection, scroll owners and surviving Bonsai
cell models. Schema/padding refresh uses the existing keyed anchor preservation.

This API does not provide custom native header/group/row render slots, visible-column
observations, structural table semantics or editing. Those catalog requirements
remain separate. GPU color/alpha and physical interaction evidence must be recorded
separately from compilation or TestPlatform model/layout checks.
