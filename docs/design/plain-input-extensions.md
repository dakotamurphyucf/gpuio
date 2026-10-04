# Plain-input extension plan

OCH-41/OCH-10 continuation, 2026-10-01. **Implementation in progress, not release
acceptance.** The password policy below is implemented locally; the remaining
stages are design and implementation work. The bound edit-menu helper is also
implemented locally and under validation; see its [contract](editor-menu.md).
Reusable input frames are now implemented locally with focused automated evidence;
see the [frame contract](input-frame.md). Semantic hints and their asynchronous
exposure-status API are also implemented locally; see the [hint contract](input-content-hints.md).
Pattern/number formatting and native regex edit filtering now have public
configuration, paired protocol, retained editing/IME integration and gallery
examples. Regex rules are prepared through Eio before attachment; filters are
distinct from form/submission validity. Physical and release acceptance remain
open. See the [validation contract](input-validation.md) and
[formatting contract](input-formatting.md). The
[pinned input review](../catalog/editor-review.md) establishes why the existing
single-line/text-area adapter is not yet complete catalog coverage. Keep the
stock OCaml/Bonsai/Core/Eio toolchain, Rust editing ownership, current command
leases and application-owned state. Full code-editor/LSP work remains separate.

## Native policy versus presentation

Draft coherent `.mli` types before implementation. Preserve existing constructors
and defaults through additive optional configuration; do not make current picker
queries, numeric inputs or OTP inherit new plain-input policies accidentally.

The domain boundaries should be:

- Password presentation and semantic privacy, distinct from format masks. A
  typed mode must make unsupported multiline/password combinations invalid.
  Define reveal ownership and observations, clipboard behavior and accessibility
  value policy together. Reading the application value still needs plaintext;
  do not advertise encrypted storage, zeroization or a general secret container.
- A validated formatting/edit-policy value. Specify formatted versus raw text,
  UTF-8 offsets and character classes. Put limits on pattern length/compiled work
  and validate atomically before mutating text or history. Retaining existing
  input that no longer satisfies a changed policy must be an explicit decision,
  not an undocumented clear/truncation.
- Semantic content-type hints with exact supported mappings and a documented
  unavailable/no-op platform result. Keep hints separate from masking and do not
  promise autofill merely because a hint was submitted.
- Input appearance with stable leading/trailing slots, clear/reveal/loading parts
  and focus styling. Rust retains the editor; changing adornments must not remount
  it, erase composition or reset undo. Passive decorations do not become extra
  accessibility/focus targets. Interactive controls need explicit Tab/activation
  behavior and accessible names.

Do not copy upstream's synchronous validation closure across the FFI. Immediate
native rejection needs a bounded declarative policy; asynchronous Eio validation
uses exact current request/query identity and displays its result separately.
Document that difference in application examples.

## Commands and events

Keep ordinary text snapshots observational and retain exact revision-conditional
replacement. Add native operations only where composing existing commands cannot
provide the required atomic behavior. Clear/reveal actions must honor current
read-only/disabled/modal state and composition, and cannot act on a remounted
query through a stale controller. First Escape cancels composition before any
optional clear-on-Escape action. Define whether clear records undo, how it reports
selection changes and whether it preserves focus before coding the control.

Extend the paired protocol deliberately with independent OCaml/Rust fixtures.
Review every decoder, config conversion, atomic admission rule and resource charge;
new optional plain-input fields must have explicit defaults for the other editor
consumers. No synchronous OCaml call is permitted from editing, layout or paint.

The [formatting contract](input-formatting.md) records the implemented exact-text/
offset boundary and the remaining native validation and acceptance work.

## Implementation sequence

1. Password/masking and semantic privacy: types, native display/clipboard/AX
   behavior, reveal policy, revision/selection/composition retention and explicit
   observation/logging policy. Add a public gallery example before claiming it.
2. Reusable input frame with prefix/suffix, clear/loading/reveal, appearance and
   current native edit context-menu capabilities. Use ordinary View composition
   where it preserves those semantics; use retained native actions for atomic
   editor behavior. Keep helper identity separate from the editor controller.
3. Formatting and validation: inspect exact upstream edit/IME/mask semantics,
   choose bounded data types, independent fixtures and a documented raw/formatted
   value model. Cover Unicode, paste, replacement, undo and rejected policy updates.
4. Ordinary text-area controls: soft-wrap, continuation layout, editable search/
   replace and required viewport commands. Inventory these separately from
   diagnostics/folding/LSP so shared-engine functionality is not silently deferred.
5. Native and public acceptance: current-state changes, queued requests, two
   windows, removal/close, bounded history/memory, idle behavior, macOS physical
   keyboard/IME/clipboard/AX/VoiceOver and an independently installed consumer.
   Required Linux build/unit/consumer checks remain; deferred Linux desktop
   qualification stays OCH-47.

Each step needs reviewed invariants and tests appropriate to the behavior. The
sequence does not turn placeholder types, a compiled example or source presence
into release acceptance. No bridge tags or exact public signatures are reserved
by this plan alone; finalize them together with the implementing `.mli` files.

## Password policy — implemented contract

`Text_input.Config.create ~mode:Single_line ~privacy:(Password Hidden) ... ()`
creates a password field. `Privacy.Plain` is the backward-compatible default.
`Password_display.Hidden` and `Revealed` are application-controlled presentation
states; change the config on the same controller to reveal without replacing the
draft. Multiline password configurations are rejected. The public gallery's
Text inputs page demonstrates reveal, read-only, disabled and submission with
synthetic text. Its reveal checkbox and integrated reveal action use application-owned state;
the reusable input frame supplies the adornments and native clear action.

Rust keeps the same input entity, focus handle, text revision, directed UTF-8
selection, undo history and marked composition across privacy changes. Native
masking suppresses Copy/Cut while hidden. Revealed passwords permit ordinary
Copy/Cut, subject to existing disabled/read-only/selection rules. Paste and editing
retain existing editor behavior. Both password states expose AccessKit's
`PasswordInput` role and omit the accessibility value, even when glyphs are
revealed. Changing back to Plain restores the text-input role and value.

This is a presentation and accessibility policy, not a secret-storage container.
Application snapshots, command replies, submissions, serialization and derived
sexps contain plaintext. Do not log them as if masking redacted their content.
There is no new guarantee of encryption, zeroization, macOS Secure Event Input,
autofill or password-manager integration. Native platform/VoiceOver verification
is still required separately from TestPlatform accessibility-tree checks.

The paired unpublished epoch-3 protocol appends Op73 `Set_editor_privacy` /
`SetEditorPrivacy`, with tags 0 Plain, 1 Hidden and 2 Revealed. Existing editor
config bytes remain unchanged. Initial Plain needs no operation; transitions emit
only the changed policy, with no draft replacement. Native admission accepts the
operation only on single-line Input nodes; choice-picker query slots must remain
Plain, including descendant-only updates. Failed batches roll back the policy,
tree revision and retained-resource accounting. No GPUI fork change is needed.

See [password validation evidence](../evidence/editor-privacy-och41.md) for the
current test scope and outstanding desktop acceptance.

## Ordinary text-area layout — implemented contract

`Text_input.Config ?layout` now carries `Text_area_layout` for multiline fields:
soft wrapping, flush/matching continuation indentation, whitespace indicators and
bounded cursor margins. The paired native adapter retains draft, composition,
selection, focus and history. See the [contract](textarea-layout.md) and
[local validation](../evidence/textarea-layout-och41.md). Ordinary search/replace now has [typed commands and a reusable bar](textarea-search.md); physical acceptance remains open. The new
[viewport query/scroll API](editor-viewport.md) observes completed native layout
and acknowledges pending scroll requests explicitly.

`Text_input.Config ?clear_on_escape` adds native opt-in clearing for both input
modes. Composition takes precedence, current editability/focus/filter guards apply,
and a successful clear is undoable. See the [contract](editor-escape.md) and
[local validation](../evidence/editor-escape-och41.md).
