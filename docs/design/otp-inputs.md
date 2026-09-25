# Segmented OTP inputs (OCH-34)

Status: bounded text policy and atomic edit helpers are implemented. Native
segmented rendering, editing integration, snapshot/event/command contracts and
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

Before native implementation, specify a narrow editing adapter/hook that can
normalize and reject committed edits atomically, distinguish paste, preserve
preedit composition and map the resulting selection. Do not repair input by
feeding asynchronous observations back as replacement commands. Finalize and test:

- Policy changes versus native placement identity; never silently truncate or
  reinterpret a retained code under a different alphabet/length.
- Canonical accepted value versus temporary composing draft, and invalid IME
  commit/cancellation behavior, including selection/history restoration.
- Revision reservation and event ordering for Changed and Complete, including
  same-value edits, caret movement, undo/redo, initial state and explicit commands.
- Segmented selection/caret/IME geometry, accessibility value/actions and masking,
  and clipboard behavior for masked fields.
- Bounded queues/history, hidden idle behavior, unmount/window close and old-lease
  command rejection, with native and public examples for each supported alphabet.

These are remaining implementation requirements, not features established by the
pure text helpers. Actual local macOS acceptance and consolidated macOS/Linux
checks remain necessary before completing OCH-34.
