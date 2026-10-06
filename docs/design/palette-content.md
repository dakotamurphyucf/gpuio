# Rich command-palette content

`View.with_palette_content` decorates a direct `View.command_palette` with
optional header, footer and empty content, plus passive rows keyed by
`Command.Id`. The Bonsai View API exposes the same operation. Unknown or duplicate
command IDs are errors. Calling it again replaces the complete content mapping;
calling it with no content restores the built-in presentation.

## Ownership and representation

The native command row owns its registry label, checked/enabled state, action,
keyboard selection and accessibility semantics. Composed icons, text, layouts
and animations decorate that row; they cannot install nested controls, handlers,
focus scopes or alternate actions. Disabled rows inherit disabled presentation.
The command label remains the accessible name even when the visual content uses
additional text. Commands still revalidate their route and eligibility on
activation, and native edit commands retain the document target captured when
the palette opened. No OCaml callbacks run during Rust layout or paint.

Header, footer and empty content are ordinary interactive Views. Their controls
retain native owners through ordinary reconciliation. Search is the initial
focus target unless a child explicitly requests focus. Default Tab order follows header, query, empty-state controls and footer;
explicit control tab ordering still applies. Header/footer controls
participate in ordinary focus traversal; their typing, directional keys and
Enter belong to the focused control. An unhandled Escape dismisses the palette
after child handling. Marked-text cancellation and nested modal dismissal take
precedence. This does not turn row decorations into independently focusable
controls.

There is no additional wire operation. A palette has either no children (legacy
presentation) or three structural slots for header/footer/empty, followed by one
slot per declared command. Each slot has zero or one content child. Row slots use
stable command IDs for reconciliation; header/footer/empty have distinct fixed
structural keys. Native admission validates the final tree atomically, including
mutated descendants. All slots together are bounded to 4,096 nodes and 128 levels;
row content additionally follows the existing passive-content contract.

## Measurement and lifecycle

Rows use retained measured GPUI list state. The configured row height is the
minimum for rich rows; rows without custom content keep that fixed height. The
rich viewport grows with content up to `max_visible_rows * row_height`, further
bounded by available window height. Thus `max_visible_rows` supplies a height
budget rather than a promise that that many arbitrary-height rows are visible.
Header/footer content adds its own normal layout height.

Content and inherited font/rem/scale changes invalidate measurements without
replacing the list owner or resetting the logical scroll anchor. Choice pickers
and palettes share the same inherited-metrics observer. Native query changes
refresh the filtered projection and hidden-content gates before queued actions
can be delivered. Filtered row content and inactive empty content cannot receive
input; hidden animated content stops requesting frames. Removing content retires
its native resources through ordinary tree lifetime handling.

The query and list remain Rust-owned. Query/highlight controllers, explicit
loading presentation and persistent embedding are separate remaining catalog
work; this content API does not establish their acceptance.
