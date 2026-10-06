# Cursor-following chart inspection

OCH-41, 2026-10-06. Implemented with [scoped local qualification](../evidence/chart-cursor-inspection-och41.md); broader catalog and release gates remain open.

`Chart_inspection.Placement.Cursor` places the card beside the latest native
pointer position relative to the plot, using the existing gap, edge flipping,
clipping and maximum-height rules. It does not move the inspected data marker or
crosshair. The inspected mark and its original source provenance remain unchanged.

Pointer movement inside the same mark must move the card: mark identity alone
cannot suppress repaint. Motion stays entirely native and does not publish an
OCaml observation or upload a new chart source. Corner/Anchor cards do not gain
continuous pointer-driven redraws from this option.

Keyboard inspection uses the mark anchor. Leaving pointer inspection, cancelling
capture, focus loss, disabling, source reset/replacement, hidden retirement and
unmount discard the remembered pointer. If a committed selection remains after
pointer departure, its card uses the anchor until a new pointer preview begins.
Captured motion outside the plot follows existing preview rules and clamps the
card position; release outside still does not commit selection.

The placement discriminator appends Cursor as tag 2. This feature originally advanced parent chart style
to schema -6; [guide spans](chart-guide-spans.md) subsequently advance it to -7.
Earlier schemas are rejected; envelope sizes do not change.
Matching packages are required. Constructors retain existing defaults and bounds.

This extends card placement only. Rich custom rows and per-datum annotations remain explicit catalog work.
Independent guide spans are documented in the separate contract above. Do not infer those capabilities
or platform qualification from the new constructor.
