# Native document actions — OCH-41

This contract adds OCaml-defined action buttons to each rich Markdown/HTML code
block or table. `Document.Actions` configures the two action lists and independent
visibility of the existing native Copy code/Copy table buttons. Defaults retain
both Copy buttons and have no custom actions. This does not substitute for the
separately required arbitrary static native renderer slots and document SDK.

Each action has an ASCII identifier, nonempty accessible label and enabled flag.
At most 16 actions per family, unique IDs within that family; IDs may recur across
code/table families. Labels are at most 256 UTF-8 bytes. Code action rows sit below the glyphs in normal layout, wrap at the available
width and participate in virtual-row remeasurement. Native Copy remains usable
independently of custom-event snapshot bounds. Actions use native button
keyboard/accessibility behavior. The host recognizes focused rich-text descendants
and traverses their native tab stops after TextView's logical links. Offscreen
virtual controls are revealed using measured bounds, then admitted only by a
subsequent visible frame; disabled and preview-clipped controls are excluded.
Production-host [focus tests](../evidence/document-virtual-focus-och41.md) cover
code/table and static profile controls. Physical and broader stress qualification
remain open. They do not execute code, perform
I/O, open URLs or synchronously call OCaml. Applications receive asynchronous
`View.document ~on_action` events and decide what to do with their payloads.
Configuring custom actions without that callback is a reconciliation error.

`Document.Config.create ?actions` accepts nondefault values only for Markdown or
HTML. Source/large-document/code/diff presentations do not invent block actions.
Changing actions does not reparse or replace source, native text, selection or
scroll owners. Action config epochs monotonically advance on a retained View,
including clear/reinstall, so queued old callbacks cannot revive after A/B/A.
Unrelated updates retain the epoch and route through the latest OCaml handler.

An event includes action ID, installed source generation/revision, input activation
metadata and an immutable code/table snapshot. Code carries its language and exact
code text. Tables carry plain header/body cells (possibly ragged while streaming)
and reconstructed Markdown. An optional half-open source byte range is supplied
only when trustworthy; HTML has no such source span. Identical blocks without a
source span intentionally have identical snapshot payloads: this is content/action
routing, not a persistent editable AST identity. Do not apply an old range to newer
source without checking/rebasing. Snapshot content does not cause parsing or I/O
on the OCaml UI domain beyond bounded wire decoding.

Native callbacks capture the installed interpretation and action epoch. Acceptance
checks the live View/entity/source, current allowed action/enabled state, source
generation, modal/window input policy, source/rich mode and installed picture.
A same-generation old picture may remain usable while an append is preparing;
its event honestly identifies the old revision. Generation resets, option reparses,
config clear/reinstall, unmount/window close and hidden/clipped controls reject
obsolete input. Same-generation events already accepted before an append retain their old,
self-contained snapshot; this is not a command to modify current source. Eio
checks the live source registry before dispatch: generation resets and released
sources reject pending observations, while valid older revisions in the current
generation may still be delivered.

Op120 appends `Set_document_actions(node, config)` and Event77 appends
`Document_action(window,node,handler,tree_revision,source,event)`. Config contains
positive epoch, observe flag, independent Copy flags and the two bounded lists.
Default reset preserves the epoch watermark. Final transaction admission checks
kind/mode/handler atomically; retained-byte accounting includes labels and IDs.
Payloads are bounded to 256 KiB total text, at most 4096 table cells and at most 4096
rows; source offsets are within the existing 64 KiB rich-source preparation bound.
Count/text budgets are checked before native admission or OCaml callback delivery;
the Rust configuration decoder checks list counts before allocation. Invalid payloads cannot enqueue;
resource/overload policy remains the existing bounded input queue contract.

Validation must cover paired codecs/malformed bounds, Core callback/epoch/source
fencing, final-state native admission and resource cleanup, real native button
mouse/keyboard/accessibility routing, disabled/config/source/interpretation replay,
preview clipping, stable text ownership and a public gallery example. TestPlatform
coverage is distinct from physical macOS clipboard/VoiceOver/GUI acceptance.
