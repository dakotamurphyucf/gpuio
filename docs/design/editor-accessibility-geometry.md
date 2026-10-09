# Editor accessibility range geometry

Implementation contract for OCH-41/OCH-17. This document does not establish
platform acceptance; the accompanying tests and evidence must do that.

The previous editor AccessKit text runs carried text and selection offsets but
no character geometry. A real macOS probe returned an empty `AXBoundsForRange`
even when the public OCaml range query succeeded for that text.

## Ownership and timing

Base publishes an immutable, Rust-only `BridgeTextLayoutSnapshot` through a
shared `BridgeTextLayout` handle during text **prepaint**. The enclosing editor's
synthetic accessibility children run after descendant prepaint. They consume
that same-frame snapshot, not the previous paint's `last_layout`. No shaping,
OCaml callbacks, timers, extra notification or synchronous bridge request occurs
in the accessibility callback. The handle contains data only, with no entity or
window reference. It is retired with the editor.

Publish geometry only while accessibility is active and the source is unmasked.
The adapter requires a matching source revision and length. Private editors do
not attach the text-run adapter. Empty editors publish an empty caret rather
than their placeholder. Coordinates are window-content logical pixels in Base;
the adapter converts to GPUI's physical AccessKit coordinates exactly once using
the producing frame's scale factor.

## Text and geometry

Keep runs in logical source order, with UTF-8 scalar boundaries and one CRLF
boundary. Shape-derived cells cover each rendered cluster; scalar positions
inside a cluster share its painted extent. Group cells only while they form one
visual row and one contiguous monotonic direction. Split at soft wraps, direction
changes or discontinuities. Runs sharing a visual row also publish
`previous_on_line`/`next_on_line`, so splitting direction does not invent extra
lines for `AXRangeForLine`. Paragraph bidi levels are resolved before splitting
soft wraps. AccessKit character positions advance from the run's
leading edge; widths describe each character's cluster extent. This permits
correct subrange boxes without interpreting a wrapped paragraph as one line.

Newlines have a zero-width cell at the preceding row's logical end. Empty lines
have a zero-width caret. AccessKit's nonempty range does not include a final
empty line after a newline; the public OCaml query includes endpoint carets.
When comparing them, union the AX text box with the separately queried ending
caret. Text outside retained layout remains available for
reading and selection, but carries no invented rectangle. A range spanning
unavailable geometry can therefore return no box. Geometry does not focus,
scroll, mutate selection, text or history. Existing revision/composition/disabled
and live-ownership checks continue to guard accessibility selection actions.

Extract cells in batches from each retained shaped row. Sorting glyph boundaries
is bounded by the retained glyph count; do not scan the entire editor for every
character or create a node for every ordinary character. Stable run IDs include
source revision and byte interval so a reflow cannot reinterpret a stale run's
character index as a different source position.

This is ordinary Input/Textarea and retained read-only text geometry. General
editable code-editor folding, inline completion and LSP remain post-v1 contracts.
The implementation adds no wire opcode or public OCaml requirement. Matching
packages carry the native adapter and reproducible GPUI Base patch together.

## Validation

Required regressions cover current-frame publication, source edits and stale
actions, Unicode/CRLF/empty text, wrapping, bidirectional/cluster geometry,
scrolling/alignment/scales, source privacy and unchanged idle notification.
Actual macOS `AXBoundsForRange` must return nonempty laid-out selection bounds;
compare against the public OCaml query after converting through the AX window
content origin. Distinguish AX text injection from real keyboard/IME/VoiceOver
testing. Full physical qualification is separate from TestPlatform assertions.
