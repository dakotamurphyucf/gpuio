# Plain-input formatting and edit validation — work in progress

OCH-41, 2026-10-01. This is the next implementation contract after semantic
content hints. **Core format values, paired protocol, retained native policy and
the public gallery example are implemented locally and under validation.** Native
regex edit filtering is also implemented locally; its separate
[contract](input-validation.md) defines preparation, filtering and submission
semantics. Physical and remaining input-family acceptance are still open.

## Preserve exact editing contracts

Native Rust remains the owner of text, composition, selection and undo. Formatting
is a text transformation, separate from password privacy and semantic hints.
Snapshots and existing replacement commands continue to describe formatted,
visible text using UTF-8 byte offsets. Never return raw-value offsets as if they
were positions in the displayed draft.

Ordinary `Replace` must keep its exact-text contract. A noncanonical replacement
under an active formatting policy should return a typed error before changing
text, selection, revision or history. It must not silently format the request and
then apply offsets measured in the original string. `Input_format.format_raw`
prepares application-supplied raw values explicitly, before submitting an ordinary
replacement. Start/end/preserved/explicit selections apply to that formatted
string using the existing command semantics; revision and composition guards
still run before format validation.

`bridge_replace_all` assumes that the underlying replacement leaves its validated
string/offsets intact. Native `apply` checks exact formatted-text acceptance first,
including for accessibility SetValue. This prevents a raw replacement from being
silently rewritten before its selection is applied. A dedicated Rust-only Base
policy hook separates canonical draft acceptance from interactive candidate
formatting and returns the caret in the transformed text.

## Bounded format vocabulary

The Core interface is now specified in `lib/core/input_format.mli` and implemented
with focused expect coverage and native configuration integration:

- `Input_format.Pattern.t`: checked UTF-8 pattern source. `9` is an ASCII digit,
  `A` an ASCII letter, `#` ASCII alphanumeric, `*` one Unicode scalar; other
  scalars are literals. This follows the pinned vocabulary. Scalar slot counts
  are distinct from the editor's grapheme-aware cursor/deletion behavior.
- `Input_format.Number.t`: optional single-scalar grouping separator and optional
  fraction limit. Define accepted signs, incomplete decimals, full-width input
  normalization and grouping independently of floating-point parsing. Reject
  separators that collide with digits/signs/decimal syntax.
- `Input_format.t`: pattern or number value with explicit `format_raw`,
  `raw_of_formatted` and `accepts_formatted` helpers. Optional
  `Text_input.Config.create ?format` configures ordinary single-line inputs.
  It is rejected for multiline inputs, comboboxes and picker queries; the
  existing numeric and OTP owners retain their own policies.

Bound pattern bytes and scalar slots, fraction count, validation compilation and
formatted output. Pattern limits are 1,024 bytes / 256 scalars, with the
existing editor text-byte ceiling applying after expansion as well as before it.
These are checked construction/operation limits, not measured resource
acceptance. Input that cannot be represented must produce an explicit error;
never silently truncate arbitrary application text to make a test pass.


## Pure conversion contract

`format_raw` consumes only slot values. For pattern `-*`, raw `-` produces `--`;
the raw character cannot be mistaken for the literal prefix. Empty raw input stays
empty, and trailing pattern literals are not invented after the final supplied
slot. `raw_of_formatted` accepts exact formatted prefixes, including optional
trailing literals already typed. Thus `(12)` can extract `12` even though formatting
raw `12` under `(99)` initially yields `(12`. Neither helper discards mismatched or
leftover slot input.

Number conversion groups only the integer part, preserves leading and fractional
zeros and never passes through binary floating point. Its raw grammar permits a
leading sign, optional decimal point and incomplete drafts. Supported full-width
numeric characters normalize before that check. Fraction limits apply independently
of grouping: excessive fractional digits are rejected, not truncated; limit zero
forbids the decimal point. The optional limit is bounded by the editor byte budget
(0..262144). Formatted extraction rejects incorrect grouping and normalization
rather than silently stripping arbitrary separators. It preserves trailing zeros,
unlike Base's existing numeric `unmask` helper.

`Invalid_text`, `Does_not_fit` and `Limit_exceeded` distinguish malformed/single-line
text, values outside the format and byte-budget exhaustion. Output expansion is
checked before allocation. UTF-8 processing uses byte cursors instead of allocating
a character list for every large draft. Pure conversion does not edit an input,
configure native policy. Independently specified fixtures pair the OCaml and
Rust conversion rules and operation encoding; no floating-point conversion occurs.

## Retained policy changes and validation

A config change retains the native owner and preserves text, directed selection,
revision, composition and undo. A newly incompatible draft remains visible. Its
recovery behavior must be explicit: reject invalid edits from a valid draft;
allow correction of a retained invalid draft without silently stripping its
unrepresentable content. Formatting and validation should resume consistently
when a candidate becomes representable. Do not clear a draft on config change.

`Text_input.Config ?edit_filter` now composes bounded regex filtering after the
format transformation, wholly within Rust. Exact replacements must satisfy both
policies. Preparation, matching modes, empty values, retained incompatible drafts
and submission semantics are defined in the [validation contract](input-validation.md).
Asynchronous Eio business validation remains separate application state, guarded
by current editor identity/revision. No synchronous OCaml predicate is invoked.

IME needs a separate policy for marked intermediate text versus committed text.
A marked range must not be mistaken for a finalized formatted value. Commit should
format atomically and undo as one change. Rejected composition, cancellation,
policy changes during composition and post-commit caret placement need native
regressions before acceptance. Simulated input-handler calls do not establish
physical candidate-panel behavior.

## Native investigation and fixes so far

Two failures were reproduced through the actual pinned `MaskPattern`:

1. `Pattern("*").is_valid("界")` rejected a single Unicode scalar because the
   consumed scalar count was compared with UTF-8 byte length.
2. `Pattern("9A").is_valid("x")` skipped the required digit slot and accepted
   text that formatting would discard. Only omitted literal separators should
   be skippable.

The GPUI Base adaptation now compares scalar counts and rejects mismatched
nonliteral slots. Native integration tests cover Unicode/literals and formatting,
plus valid/invalid ASCII slot cases. A TestPlatform edit test covers formatted
Unicode insertion, UTF-8 caret offsets and undo/redo. Composition and rejected-edit
lifetime checks also pass. The composition test exposed history replay formatting
its recorded intermediate value again, which invalidated subsequent byte ranges:
committing `界12` as `界–12` and undoing produced `––12`. A separate replay flag now
bypasses normalization/validation/masking only during Undo/Redo, preserving normal
`set_value` formatting. Seven focused regressions and the full native library
suite pass. No OS input or new public API is claimed.

The changes belong in `third_party/patches/gpui-base.patch` with its hash updated
in `third_party/sources.json`; archive reconstruction remains a required check.
See [plain-input scope](plain-input-extensions.md) and the
[source review](../catalog/editor-review.md) for the complete remaining surface.

## Native policy integration and resource boundary

`Set_editor_format` is operation 76 in the paired unpublished protocol epoch 3.
It carries an optional validated pattern/number value; legacy editor-config bytes
remain unchanged. Rust validates before atomic tree admission, rejects policies
on incompatible owners (including descendant-only picker-query updates), and
charges configuration plus a conservative compiled-policy allocation allowance.
Clear/removal releases that charge. Both runtimes must be rebuilt together.

A single Rust `BridgeInputFormat` owner is installed on the retained editor.
Pattern candidates use the pinned mask vocabulary with a bounded compiled token
array; oversized incompatible drafts are rejected before allocating mask character
vectors. Numeric candidates use the paired conversion helper, enforce fraction
limits even without grouping, and map the caret through grouping of the whole
value. Formatting only a prefix is insufficient when an integer group changes
width. Application placeholders are preserved; policy removal clears the owned
hook without installing an always-true validator or leaving a stale mask behind.

Marked IME text is provisional and need not fit the format. The native editor
keeps the bounded pre-composition draft and directed selection. Commit or unmark
formats against the current policy; a rejected commit from a compatible baseline
restores that baseline and removes the provisional undo transaction. Cancellation
also restores it. Successful commit is one undoable edit. Policy changes retain
the marked text; the baseline also exists if formatting is first enabled during
an already-active plain-input composition. If the baseline itself does not fit
a newly selected format, incompatible text remains available for recovery.
Read-only/disabled transitions do not strand an unmark operation.

The gallery's Editors page demonstrates reference masks, grouped decimals, free
text, retained incompatible drafts, observed raw extraction and explicit sample
replacement guarded by the observed lease/revision. It does not rewrite native
observations from Bonsai on each keystroke. Native automation/physical IME,
clipboard, external accessibility, visual and resource acceptance remain open.
Regex filtering has its own [integration evidence](../evidence/input-validation-och41.md).
The remaining plain-input catalog and physical/resource acceptance still belong
to OCH-41 and OCH-17.
