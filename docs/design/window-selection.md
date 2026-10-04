# Window-wide text selection

OCH-41 implementation contract. These operations target the registered read-only
text/document selection layer of one exact native window generation. They do not
read the clipboard or inspect editable input/password/OTP selections.

`App.Window.has_text_selection` reports an active geometric or renderer-local
selection in the active selection scope. `selected_text ?max_bytes` collects
nonempty fragments in logical document order, separated by a newline; whitespace
fragments are preserved. No selected text returns an empty string, but an empty
string alone is not proof that no geometric/local selection exists.

The default limit is 65,536 UTF-8 bytes; allowed limits are 0..262,144, matching
the existing maximum text payload. Separators count toward the limit. Invalid
limits return Invalid_request. Oversized results return Limit_exceeded, with no
partial text and without changing selection or clipboard. UTF-8 scalars are never
split. Queries use the existing 64-request window lane and are observations at
native execution, not retained snapshots for later conditional mutation.

`clear_text_selection` clears window geometry and registered renderer-local
selections, including inactive registered scopes, following the pinned Base
window operation. `end_text_selection` ends a drag/auto-scroll while preserving
its visible selection. Both act on the current selection when executed. Neither
is an editor command. Closing the exact window rejects/finishes requests under
the existing window request contract; a replacement window cannot receive them.

The bounded native collector snapshots ordering/callbacks before invoking them,
with no borrowed selection entity held through a callback. It bounds aggregate
fallback copies and merged output, invokes callbacks one at a time and stops at
the first oversize result. Legacy renderer callbacks still return owned strings:
one callback may materialize its complete fragment before the collector can
check its length. This is a bridge/output and collector bound, not a whole-process
allocation guarantee. Renderer-specific copy normalization remains owned by the
registered renderer; no simplified alternate Markdown/plain-text copier is used.

A small maintained Base patch provides the bounded collector; retain exact source
reconstruction evidence. Paired codecs, native scope/order/clear/end/limit tests,
mailbox size accounting, Core validation and a public gallery example are required.
Real desktop selection, IME/accessibility and Linux compositor qualification are
separate acceptance gates.
