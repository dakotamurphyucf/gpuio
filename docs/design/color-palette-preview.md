# Native color palette preview

OCH-41, 2026-10-02. Applies to inline `Gpuio_eio.Color_input` and the same native
input inside `Gpuio_eio.Color_picker`. No new wire operation, callback or timer.

Hovering an editable, allowed palette entry temporarily displays its swatch and
hex spelling. The caption occupies a reserved line, so entering or leaving a
swatch does not move the palette or channel controls. Transparent colors retain
the existing checkerboard rendering. Repeated colors remain distinct slots.

This is inspection state owned by the Rust control. It does not update the color
model, revision, snapshot, event queue, focused field, text selection, undo history
or marked composition. The hex editor continues to display its own draft. The
passive preview caption is excluded from accessibility; the group's value and
radio selection continue to describe the actual native color. Palette labels and
keyboard activation remain available independently of mouse hover.

Pointer leave clears only the matching slot. A saved callback from a replaced
handler or palette entry cannot install or clear a current preview. Policy and
configuration changes, handler replacement, channel editing, successful
Set/Reset/Cancel/Focus, hiding, disabled/inert/pointer gates, modal blocking,
window inactivity, removal and native faults retire the transient state. Read
and rejected commands preserve it. Pointer capture prevents a palette preview
from replacing the display of an ongoing channel drag. Hover alone does not pin
a managed row or keep an owner alive.

The pinned source writes hover hex text into its input. GPUIO deliberately uses a
separate caption to preserve active typing and composition. Selecting a palette
entry still uses the established native edit path. Popup application values
still require explicit Apply; hovering never confirms a popup draft.

Grouped/featured palettes and internal appearance are described in the
[presentation contract](color-presentation.md). Palette/HSLA tabs remain separate
OCH-41 work. See the [source review](../catalog/calendar-color-review.md)
and [local evidence](../evidence/color-palette-preview-och41.md).
