# Prepared rendered semantic ownership — OCH-17

Local macOS arm64 checkpoint, 2026-10-07. OCH-17/OCH-41 and milestone 07 remain
open. This adds logical structural metadata; it does not yet publish rich
TextRuns, accept OS selection actions or qualify VoiceOver.

## Implementation

`RenderedText` prepares a bounded preorder arena alongside the existing native
selection projection. It records paragraphs, heading levels, blockquotes,
lists/items, code, tables/rows/cells, frontmatter terms/definitions and custom
block owners. Row/column/header metadata and structural separators retain their
actual ancestry. Preparation precedes layout, so owners outside the viewport
remain addressable without retaining an AST or native view.

Opaque semantic IDs include the preparation identity. Parent, direct-child,
part-range and selection queries reject foreign preparations, even for equal
text. Direct-child traversal skips subtrees. Links retain exact logical fragments
within native text parts, including the ordered edges of empty atomic objects.
Styled pieces of a source-identified link coalesce; equal destinations alone do
not merge independent links. The renderer's current NodeContext resolves
reference URLs/titles during both initial and resource-refresh preparation.

The arena admits at most 32,768 nodes. Reservation and retained accounting include
node capacity and unique shared URL/title bytes. Repeated reference occurrences
share destination storage. Source, generated-text and Copy limits are unchanged.
This is allocation admission, not measured process-memory acceptance.

## Validation

Seven new native regressions cover nested roles/table coordinates and ancestry;
200 unpainted headings, foreign IDs and weak-owner retirement; frontmatter and
declared native glyph blocks; styled/reference links; two empty objects within
one link; adjacent HTML links with equal targets and exact Unicode ranges; and
500 references sharing one long URL. Existing Copy text remains unchanged.

All **1,132 native library tests pass; 2 existing tests remain ignored**. Actual
macOS native editor/document tests, full Rust workspace tests, strict all-target
Clippy, full Dune `@all @runtest @fmt`, Rust formatting and `git diff --check`
pass through the isolated toolchain with two jobs. The rebuilt gallery passes
native reading order, actual Copy and normal close with child exit zero and typed
clipboard restoration. Documentation inventory remains 429 sources / 266 reviewed
groups / 0 pending; that audit establishes structural coverage only.

The maintained Base patch reconstructs all **240 files** exactly, excluding
Cargo.lock. Patch SHA-256:
`c556fcde3baecedfffaf195e7cf7e356f4d475bc2b0f19de4606906c3243ab73`.

Initial failures are retained: the frontmatter fixture omitted its explicit parser
option, and the initial link implementation passed the legacy paragraph string
map to an API requiring typed LinkMark references. The fixture now enables the
intended parser behavior; preparation uses the actual renderer context. Neither
acceptance assertions nor admission limits were weakened.

The [archive](rendered-semantic-structure-och17/reports.tar.gz) and
[manifest](rendered-semantic-structure-och17/manifest.json) retain exact commands,
results, source/platform/binary hashes and initial failures. Members are verified
against their recorded hashes.

## Remaining integration

The [design](../design/rendered-document-selection.md#prepared-structural-ownership)
still requires attaching these owners to the actual native semantic subtrees,
preserving interactive children and unique link actions. TextRuns must split at
semantic and shaped geometry boundaries without duplicating a hidden flat document.
Selection must publish after native paint finalizes its range, before the frame's
accessibility update. OS actions require current owner/window/visibility/modality,
input policy, preparation and interaction-generation checks.

Native unit structure checks and existing desktop checks do not qualify that
unimplemented publication/action path or physical VoiceOver. Broader catalog,
interaction/reflow/virtualization, performance/resources, physical presentation,
notices, API and distribution gates remain open. Hosted run 37629820030 covers
earlier e4630996, not this checkpoint; no current-source Linux GUI, clean-machine
or physical-presentation acceptance is inferred.
