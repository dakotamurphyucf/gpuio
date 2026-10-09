# Rich palette content qualification — OCH-41

Local qualification, 2026-10-05, macOS 14.5 arm64. Base `bd5c7a4` plus the archived
source patch. This qualifies the rich-content slice; OCH-41 and OCH-17 remain open.

The public `View.with_palette_content` API (Core and Bonsai) supplies measured,
passive command rows and interactive header/footer/empty Views. The
[contract](../design/palette-content.md) defines stable identity, structural
admission, bounds, focus, native command ownership and filtered-content lifetime.
The public OCaml Feedback gallery demonstrates the API using registered SVGs,
command descriptions, a native header editor and interactive help/empty actions.

## Local evidence

- OCaml view API expect tests and the full `@runtest`/`@fmt` check pass. Validation
  covers direct-owner requirements, unknown/duplicate command IDs, passive row
  restrictions, the shared subtree bound and rich/plain reconciliation without
  replacing the palette. No expect output was auto-promoted.
- The final full native feature suite passes **956 tests, with two existing
  skips**. Strict native/protocol feature Clippy and formatting pass. A subsequent
  test-only accessibility-focus assertion also passes its focused test; production
  code is unchanged from the full suite and installed binary.
- Native tests cover variable-height measurement and live height changes,
  query focus on mount, visual Tab/Shift-Tab order, header Enter ownership,
  marked-text cancellation before dismissal, nested modal Escape, hidden-empty
  rejection of a queued AX action and atomic invalid descendant/slot admission.
  Filtered decorative activity stops scheduling frames, resumes when visible and
  retires when removed. A 1,000-row rich palette paints fewer than 30 rows before
  and after moving to row 900; a height mutation retains its item/7-pixel anchor.
  These tests use TestPlatform, not physical OS IME input.
- A fresh independently installed gallery builds and passes its linked catalog
  checks. The final full macOS Feedback walkthrough passes real Tab/Shift-Tab,
  query-first focus, header Enter, retained editor text across footer updates,
  interactive empty-state actions, filtered groups and rich row geometry. Existing
  search/Escape policies, native shortcuts, menus, notifications and teardown also
  pass. AX drives field values and button actions; OS key events exercise the
  keyboard paths. Both final full/filtered screenshots were inspected.
- The rebuilt real-window native palette suite passes native query/composition,
  accessibility activation, 1,000-command wheel/keyboard/reorder navigation,
  bounded history, current command eligibility, nested overlays, document Copy,
  focus restoration and command-before-dismissal ordering. It includes internal
  GPUI input dispatch and does not establish physical OS IME qualification.
  Its process is reaped and original clipboard representations restored.

Final installed gallery SHA-256:
`e486aadc2f923bf0bc5f8d60ebf7e051727b133d6a2e9e5381d34d55585c6e9f`.
Final native palette executable SHA-256:
`426cb8319fa8ff598d87fd930518d0609d49b0240606b9aa5eeb7f354c47e182`.
Both foreground walkthroughs close and reap their applications. No VoiceOver or
OS preference changes were made. This is not Linux GUI or screen-reader evidence.

## Failures retained and corrections

The correctly frame-driven opening-focus regression fails with the old focus
manager: the header receives focus before the private query. The palette now
prefers its query while preserving explicit child focus. A second regression
exposes query → header → footer traversal, caused by registering query focus at
the end of panel paint. Recording it in visual order fixes header → query → footer.
A native accessibility-tree assertion independently checks initial query focus.

The first expanded real-window driver cannot observe query AX focus after opening
through AXPress; its diagnostic reports the application window as the focused
AX element. Raising/activating the gallery before opening makes the complete
walkthrough pass without setting query focus or relaxing the assertion. The final
run uses a diagnostic wrapper around the unchanged driver entry point; the wrapper
only logs actual focus on failure. Original failed runs remain in the archive.

Earlier compile/admission failures caught incorrect visibility, Core's `Command`
namespace collision, missing native child admission and test fixture mistakes.
They are preserved alongside successful checks. The first focus test omitted
next-frame processing; only the corrected counterfactual establishes that repair.

[Exact commands, original failures, source patch, logs and screenshots](palette-content-och41/validation.tar.gz)
are retained with a [checksum manifest](palette-content-och41/manifest.json).
Hosted run 37387307992 tests earlier `f6e34e2`, not this addition. Final-source
hosted checks/review, remaining query/highlight/loading/embedding controls, OS
popup menus, other catalog work and release acceptance remain required.
