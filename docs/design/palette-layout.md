# Grouped command palettes

`Command_palette.Config.create_entries` preserves the existing registry-backed
command model while adding ordered `Command`, `Group` and `Separator` entries.
`Config.create` remains the flat constructor. A group has a distinct, stable
`Group.Id`, optional passive heading and ordered command references. An empty
group is valid. Commands are unique across the whole palette; group IDs are
unique within it. Neither is a callback or a Rust resource handle.

```ocaml
let tasks =
  Command_palette.Group.create
    ~id:tasks_group_id
    ~label:"Tasks"
    ~commands:[ run_command; stop_command ]
    ()
in
let%bind.Or_error tasks = tasks in
Command_palette.Config.create_entries
  ~label:"Commands"
  ~entries:[ Group tasks; Separator; Command copy_command ]
  ()
```

The application supplies validated IDs and declares command labels, enablement,
checked state, shortcuts and actions in its enclosing `Command.Registry`.
Grouping does not change invocation, captured document-editor ownership, queued
OCaml events or dismissal ordering. The native query and its editing history
survive layout changes. Legacy flat palettes do not send a layout operation.

## Admission and transport

At most 1024 top-level entries and 1024 commands are admitted. Group IDs are
nonblank UTF-8 without NUL, at most 256 bytes; optional headings follow the same
text rule with a 4096-byte maximum. Blankness uses Core's existing byte-whitespace
contract. Group IDs/headings, query keywords, command references, palette label
and placeholder share one 256-KiB metadata budget.

Appended operation 126, `Set_palette_layout`, carries an optional presentation:
command index; group ID, optional heading and indices; or separator. Indices must
flatten to exactly `0..N-1` in declared command order. They are internal wire
references, not the public identity API. The decoder bounds total command slots
across all groups before allocating each index vector. Native transaction
admission additionally checks coverage and the shared search/layout budget against
the final palette configuration. Failure rolls back the transaction. `None`
resets to flat presentation without remounting the palette or its query.

## Rendering and navigation

Native filtering produces two projections: matching commands for selection and
visual rows for rendering. A group heading is absent when none of its commands
match. Leading/trailing separators and consecutive separators are suppressed
after filtering. Headings expose passive accessibility text; separators have no
activation action. Arrow/Page/Home/End navigation continues to use eligible
commands only, including when headings or disabled commands intervene.

GPUI's measured `ListState` retains row measurements and a logical scroll anchor.
Heading and command keys use separate identity domains; separators use structural
entry positions. Structure changes splice the affected interval, preserve the
visible key and intra-row offset, and use the next surviving row (then previous)
when that key disappears. Keyboard reveal maps the selected command to its visual
row. An unchanged selection does not force scrolling every frame. Visible work
is virtualized with bounded overscan. Command row height follows the existing
appearance contract; headings and short dividers are measured independently.

This addition does not complete command-palette catalog parity. Rich command
content, interactive header/footer/empty content, query/highlight controllers,
loading, and persistent embedding remain separate implementation work. Tests of
native state or AX structure do not establish physical IME or VoiceOver coverage.
