# Internal document styling — OCH-41

`Document.Style` supplies checked presentation data to
`Document.Config.create ?text_style`. It applies to Markdown/HTML; supplying it
for Code/Diff is an error. Ordinary outer `View.document ~style` and source-editor
appearance keep their own contracts. No synchronous OCaml styling callback crosses
into native layout, measurement or paint.

Colors name the foreground, muted foreground, links, selection, code background
and borders. Theme tokens resolve on submission, including hidden content. A
six-value heading map replaces a native heading-size callback; values are logical
pixels. Paragraph gap explicitly uses rem. Inline code supports foreground,
background, weight, italic/normal, underline, strike and fade with validated units.

Code blocks, tables, table headers and cells accept Base-only presentation
refinements: colors, borders/radii/shadows, typography, width constraints, padding
and margins. Visibility, clipping, fixed heights, absolute placement, interaction
and independent scrolling are owned by the reader, not by its styled parts.
Unsupported declarations return an error. This restriction does not change the
outer View style vocabulary. No raw native style callbacks are exposed.

The append-only Op118 transports an optional resolved configuration. Removal
restores defaults; clearing it and changing document mode are one atomic tree
transaction. Reconciliation caches the resolved value so theme-only changes are
submitted without republishing source. No configuration epoch or callback is
needed for this presentation-only operation.

Native updates retain prepared text, source identity, selection and worker jobs.
They invalidate rich-text measurements and owning managed-list row geometry when
the effective style changes. Logical selection remains valid while hitboxes and
selection geometry are rebuilt. Native configuration, decoding and Core constructors
all enforce bounds; malformed direct bridge values cannot bypass public validation.
