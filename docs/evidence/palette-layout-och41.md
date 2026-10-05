# Grouped palette qualification — OCH-41

Local qualification, 2026-10-05, macOS 14.5 arm64. Base `e88f524` plus the archived
source patch. This closes the grouped/separator presentation slice, not the
whole palette catalog or milestone 07.

The public `Group.Id`, `Group`, `Entry` and `Config.create_entries` API is
implemented with optional operation 126 and atomic native admission. The
[design](../design/palette-layout.md) records exact bounds, retained ownership,
filtering, keyboard and measured-list semantics. The public Feedback page now
uses two groups separated by a divider.

## Results and scope

- Independent Rust and OCaml bin_prot fixtures agree on the layout transaction.
  Protocol tests cover malformed/truncated payloads, duplicate groups, invalid
  indices, aggregate index allocation limits, exact flat-command coverage and
  the shared keyword/layout metadata budget. OCaml expect tests cover validation,
  command order, layout-only reconciliation and reset without remounting.
- The full default Rust suite passes 1,361 tests, with two existing skips, before
  the final admission-time projection refinement. After that refinement, all
  605 default native unit tests pass again (two existing skips). The focused
  feature filter passes eight tests: seven palette/projection tests and one
  existing color-palette test. They cover filtering/group removal, short measured
  divider geometry, disabled navigation, retained query, atomic rejection,
  composition/Escape, retired scope behavior and logical scroll anchors. The
  reset regression checks the current projection **before paint**.
- Final full OCaml `@runtest`, formatting and strict native/protocol feature
  Clippy pass. The independently installed public gallery builds successfully.
  No OCaml expect output was auto-promoted.
- The real-window native palette suite passes 1,000-command wheel/keyboard/reorder
  reveal, bounded query/history, pointer and outside policies, hidden/nested
  overlay cleanup, marked/committed text, accessibility activation, current command
  eligibility/generation, native document Copy, focus restoration and
  command-before-dismissal ordering. Its inputs include internal GPUI dispatch;
  this is not a claim of physically typed OS IME validation. The child is reaped
  and the user's original clipboard representations are restored.
- A fresh independently installed gallery passes the complete macOS Feedback
  walkthrough, including actual OS keyboard shortcuts/Escape and AX-driven query,
  search/visibility policies, both passive headings, disappearing empty groups,
  native command/menu actions, retained editing and teardown. Screenshots of the
  full and filtered palette were inspected; headings, divider, highlighted command
  and disabled Copy row render correctly. This is not VoiceOver or Linux GUI
  evidence. No OS preferences were changed.

The installed gallery executable SHA-256 is
`d4fd542f9d46941c080edbfcbe54b51fe7b6ff3230d6464a3465dd3c21373b3a`.
The native palette executable SHA-256 is
`41ae95ac21d8f60ffd42a3991d1f4c1f1241241bfe9fbe6f4792f1e0cb8edd93`.

Initial native compilation caught an ambiguous closure index and test assertions
against GPUI's non-`PartialEq` `ListOffset`. Explicit index typing and field-wise
assertions corrected those compile errors. The first OCaml build had already
copied that earlier native source and failed on the same error; the subsequent
full builds pass. Original failure logs remain in the archive. Formatting
promotion affected formatter output only. No acceptance bound was relaxed.

The measured-list review also added explicit retained row/key memory reservations
and admission-time projection refresh: an occluded window must not wait for paint
to update its retained group data. Row element IDs separate command and group
namespaces. Public rich rows, interactive header/footer/empty content, query and
highlight controllers, loading, persistent embedding and remaining catalog/release
requirements stay open.

[Commands, source patch, original failures, final logs and screenshots](palette-layout-och41/validation.tar.gz)
are retained with a [checksum manifest](palette-layout-och41/manifest.json).
Hosted run 37387307992 tests the earlier `f6e34e2` checkpoint and does not qualify
this addition; current-source required CI and review remain necessary.
