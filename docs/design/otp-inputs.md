# Segmented OTP inputs (OCH-34)

Status: bounded text policy, atomic edit helpers and a platform-independent
native editing model are implemented. Native segmented rendering, platform
editing integration, snapshot/event/command contracts and
public Bonsai/Eio controllers are not implemented yet. No OTP capability is
advertised. This document supplements [numeric inputs](numeric-inputs.md).

## Implemented text contract

`Gpuio.Otp_input.Policy` validates length 1 through 32 and chooses Digits (default)
or Ascii_alphanumeric. Alphanumeric values preserve case. `Value` is an abstract
canonical ASCII prefix; empty prefixes are valid. Compatibility with a particular
policy is checked explicitly, since a value constructed under a wider/alphanumeric
policy need not fit a different policy. `is_complete` checks compatibility and
exact length; it never means authentication succeeded.

`Value.of_string` maps full-width U+FF10–FF19 digits and U+FF21–FF3A / U+FF41–FF5A
Latin letters to their ASCII equivalents, then checks the selected alphabet.
It does not normalize other Unicode digits, spaces, punctuation or confusables.
`Value.of_paste` additionally removes ASCII hyphens and the six ASCII whitespace
characters (space, tab, CR, LF, vertical tab, form feed). Full-width hyphens and
nonbreaking spaces remain invalid. Raw input is bounded to 4,096 bytes before
normalization, including separators; there is no unbounded separator scan.

Unexpected characters report their original UTF-8 byte offset without embedding
the code text in the error. The entire input/edit rejects on invalid UTF-8,
unexpected characters, capacity overflow or excessive raw bytes; no valid prefix
is accepted implicitly and no truncation occurs. Check raw byte bounds before
UTF-8 validity, then examine characters left to right. Raw unvalidated protocol
policies are rejected before normalization.

`Value.replace` and `Value.paste` take a validated value, policy and directional
selection. Canonical ASCII byte offsets equal cell indices. Both verify the
existing value and selection before constructing a result. A successful insertion
replaces the selected interval and collapses the caret after the normalized
inserted text. Direct empty replacement deletes the selection. Empty or
separator-only paste preserves both value and directional selection, matching
an empty clipboard operation rather than unexpectedly deleting a selected code.
The supplied immutable value is unchanged after any failure.

OCaml implementation: `lib/core/otp_input.{ml,mli}` and
`lib/protocol/otp_wire.ml`. Rust counterpart: `rust/protocol/src/otp.rs`.
The policy encoding is length followed by alphabet (Digits=0,
Ascii_alphanumeric=1). Independent tests pin six digits to `0600` and length-32
alphanumeric to `2001`. This establishes policy encoding only; no OTP message,
node kind, command/result or observation tags are registered yet.

## Native integration requirements

Retain one native editing session for the entire code and draw segmented cells;
never allocate one editor per cell. Keep stable native ownership, revision and
window/node lifetime fences. Ordinary observations, mask and disabled/read-only
updates must not reset the code or caret. Explicit replacement and clear commands
must validate atomically. Keyboard navigation/deletion, directional selection,
paste distribution, native IME and accessibility editing must share the same
accepted text policy. Complete is an ordered input notification, not verification
or a request to contact an authentication service.

Inspection of the pinned `vendor/gpui-base/src/otp_input.rs` confirms that its
append/backspace model lacks the required selection, paste and IME handling and
accepts unfiltered programmatic strings. It remains a reference, not the adapter.
The existing `InputBaseState` has a boolean validator, but calls it for both final
text and marked composition. Installing an ASCII-only validator would reject
legitimate IME intermediates, and that validator cannot normalize insertion text.

The OTP adapter will implement GPUI's `EntityInputHandler` directly, using one
entity and one bounded editing session. This avoids changing the generic editor's
validation/history behavior and lets segmented paint, hit testing, selection and
IME candidate bounds share the same geometry. An invisible ordinary text editor
under a separately painted row would have incorrect glyph/candidate positions.
Do not repair input by feeding asynchronous observations back as replacements.

### Implemented editing model

`rust/native/src/otp_edit.rs` owns an immutable policy, canonical accepted text,
directional selection, optional preedit draft/marked range and bounded undo/redo.
Policy changes require an explicit remount; no retained value is truncated or
reinterpreted. Configuration, focus, leases, revision reservation and publication
belong to the mounted owner and are not implemented by this pure model.

The first marked insertion keeps the accepted value and selection as a checkpoint.
Subsequent preedit updates affect only the draft, including arbitrary single-line
Unicode that would be invalid as a final code. NUL/CR/LF are rejected. The entire
draft is bounded to 4,096 UTF-8 bytes; excessive or malformed preedit leaves the
previous state intact. GPUI mutation ranges use exact UTF-16 scalar boundaries;
surrogate-interior, reversed or out-of-document ranges reject. The model exposes
UTF-8 offsets for render/snapshot consumers. Selection within new preedit is
relative to its insertion, as required by GPUI's platform interface.

Final native insertion or `unmark_text` normalizes the candidate once. A valid
candidate becomes one undo edit with mapped ASCII cell selection. Invalid final
text restores the original accepted value and exact directional selection, ends
composition and reports rejection. Neither case puts intermediate preedit in
history. Escape cancellation and empty marked text restore the checkpoint too.
For a composition rejection, unexpected-character offsets refer to the complete
raw candidate draft; ordinary insertion/paste errors refer to the raw insertion.
Malformed final replacement ranges preserve the active composition unchanged.

Paste has its own normalization path and rejects while composing. Programmatic
replacement, undo/redo and cell navigation/deletion also reject active composition;
native IME navigation must be routed to the platform. Outside composition, cell
movement and forward/backward deletion handle directional selections, endpoints
and selection extension. Programmatic replacement takes canonical text, an exact
selection and Record/Reset history policy; validation precedes any history reset.
Ordinary selection changes and same-value edits do not create undo entries or
discard redo. Explicit Reset clears both stacks even for an unchanged value.

History retains at most 128 edits across undo and redo, each with before/after
values and selections. Its text payload is at most 8,192 bytes (128 × 2 × 32),
separate from the one bounded preedit draft and allocator/structure overhead.
There is no unbounded IME transaction log. A changed accepted value that reaches
full length reports completion eligibility, including a different full code or
redo; unchanged values, selection and preedit do not. The mounted event owner
must distinguish user edits from programmatic observations and preserve ordered
Changed/Complete boundaries. The model does not yet publish bridge events.

### Remaining native integration

Finalize and test:

- Enforce immutable policy at native placement and in the public controller API.
- Connect the accepted/preedit/history model to actual platform IME callbacks,
  keyboard routing and lifecycle cancellation; pure tests are not platform proof.
- Revision reservation and event ordering for Changed and Complete, including
  same-value edits, caret movement, undo/redo, initial state and explicit commands.
- Segmented selection/caret/IME geometry, accessibility value/actions and masking,
  and clipboard behavior for masked fields.
- Bounded queues/history, hidden idle behavior, unmount/window close and old-lease
  command rejection, with native and public examples for each supported alphabet.

These are remaining implementation requirements, not features established by the
pure text helpers/editing model. Actual local macOS acceptance and consolidated macOS/Linux
checks remain necessary before completing OCH-34.
