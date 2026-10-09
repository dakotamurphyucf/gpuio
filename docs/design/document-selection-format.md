# Document selection copy format — OCH-41

Implementation contract: `Document.Selection_format.Plain_text` (default) or
`Markdown`, selected through `Document.Config.create ?selection_format`.
The option affects the native Copy selection command in rich Markdown. Select-all
in Markdown mode returns the complete original Markdown; a partial selection
reconstructs Markdown from selected parsed nodes, not an exact source byte slice.
Plain mode copies rendered text. HTML always copies selected plain text, even
when Markdown format is requested; explicit Copy source retains original HTML. Code/diff and explicit source fallback already
display source and retain their existing copy behavior. Whole-source and fenced
code/table Copy buttons are independent and unchanged.

Changing the format retains source registration/generation, parser work, native
entity and selection geometry. It does not grant selection to a disabled/non-
selectable subtree or bypass modal ownership. Unmount/recycled node identity drops
the setting, with Plain_text as the native default.

Append-only Op116 `Set_document_selection_format (node, markdown)` transports a
validated binary Boolean on DocumentView nodes only. Existing Set_document bytes
are unchanged. The final retained node value is applied to the TextView state
before copy dispatch and to subsequent rendering; only changing this option
must not submit another document/parser update. Admission remains atomic.

Required evidence: paired codec bytes/malformed input, node-kind/stale-generation
admission, Core reconciliation updates/clearing without document republish,
native selected content across format switches and source/mount teardown, public
gallery controls. Real desktop clipboard/keyboard acceptance remains a separate
OCH-17 gate; TestPlatform evidence cannot substitute for it.
