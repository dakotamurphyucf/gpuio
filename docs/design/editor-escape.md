# Opt-in clear on Escape

`Text_input.Config.create ?clear_on_escape` defaults to `false` for both ordinary
input modes. Enabling it keeps the native editor, draft, selection, composition,
focus and history. The Editors gallery demonstrates the option together with
read-only state on a single-line title and multiline body.

## Native action ordering

1. Existing composition handling takes precedence. The first Escape ends the
   marked composition using the current engine policy, consumes the action and
   never also clears the field. This retains the previous adapter behavior;
   unmarking can commit or reject provisional text according to formatting/filter
   policy, rather than promising restoration of a particular provisional string.
2. Otherwise, when enabled on the current focused, editable, nonempty field,
   Escape attempts one atomic, undoable replacement with empty text. The current
   exact-text format/filter must accept that replacement. Success moves the caret
   to the start and preserves focus. The native editor's usual observation path
   publishes the change; there is no synchronous OCaml editing callback.
3. That clear attempt consumes Escape even when the filter rejects empty text.
   Rejection preserves text, directed selection, revision and history. It must not
   unexpectedly dismiss a dialog or other ancestor.
4. With the option off, or the field empty/read-only/disabled, ordinary propagation
   remains in force. Existing composition handling still has precedence.

The handler reads the current generation-checked tree node and editor policy at
action delivery, checks the active focus gate and native editability/focus, then
uses the existing bounded replacement path. It does not enable GPUI Base's
`clean_on_escape`: the underlying `clean` helper temporarily allows edits and is
not the right user-action policy boundary. No new vendor patch is required.

Picker query fields reserve Escape for their own dismissal/navigation contract.
Atomic picker admission rejects enabling this override on the query child,
including descendant-only transactions. A normal application search field outside
a picker may enable it.

## Wire contract

Unpublished epoch 3 appends Op79 `Set_editor_clear_on_escape (NodeId, bool)` /
`SetEditorClearOnEscape`. Both decoders require a canonical Boolean. Native
admission accepts only Input/Textarea nodes. The default is false, changing the
flag allocates no extra retained payload, and failed transactions roll back the
flag, tree revision and resource accounting. Legacy EditorConfig bytes remain
unchanged. The reconciler emits only changed flags and never resends the draft.

See [local evidence](../evidence/editor-escape-och41.md). Physical macOS keyboard,
IME and accessibility acceptance is distinct from native TestPlatform dispatch.
