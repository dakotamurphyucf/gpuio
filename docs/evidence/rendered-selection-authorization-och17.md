# Accessible selection request preparation — OCH-17

Local macOS arm64 work after `c00dc16b`, 2026-10-07. This checkpoint adds
validation before a rendered-document selection request can be applied. It does
**not** connect OS selection actions or complete focus/reveal acceptance.

## Implementation

GPUI retains the final-tree Document/run scope index already needed for selection
publication. Only Documents claimed during paint can authorize endpoints; a claim
with no current selection still admits a valid new range. Hidden/disabled ancestry,
nested independent text scopes, absent runs and invalid scalar indices are
rejected. Calls during draw are rejected, and the adapter's live activation flag
makes deactivation effective before the next frame. The index holds IDs/counts
only, expires each frame and uses one traversal for all Documents.

Base's `prepare_accessible_selection` additionally checks the exact window and
prepared projection, current selectability, the existing host focus predicate and
a strong interaction epoch captured during paint. The strong stamp makes clearing
or changing selection policy invalidate old authorization even when no earlier
request holds that epoch. Preparation returns the existing checked native request
without changing selection, focus or Copy. It grants no lasting authorization;
a future dispatcher must validate immediately before mutation.

The [design contract](../design/rendered-document-selection.md#preparing-an-accessible-selection-request)
explains this boundary and the remaining work. OCaml APIs and wire types are
unchanged. No OCaml callback is added to native painting or input.

## Validation

Two new native regressions cover forward/backward/caret endpoints, explicit
clearing, a frame without a paint claim, nested Document/input/Terminal scopes,
hidden and disabled ancestry, both invalid endpoints, live deactivation,
equal-text documents, cross-window rejection, interaction retirement, the live
host guard, equal-text replacement and unmounting. Existing reading/Copy tests
remain in place.

The first lifecycle test incorrectly separated mutation and assertion into two
VisualTestContext updates, which permitted an automatic repaint. Keeping them
in the same Window update now tests the intended interval before the next paint.
The corrected six-test selection group passes. Strict Clippy found two redundant
references in test assertions; they were removed without changing expectations.
An initial filter matched zero tests and was excluded from acceptance evidence.

The full native suite passes **1,159 tests**, with **2 ignored** isolated Linux
D-Bus fixtures. Strict workspace Clippy and Rust formatting pass. The actual
macOS editor/document test executables pass, covering existing native input,
clipboard, UTF-16 text-client, focus, streaming selection and teardown behavior.
The runner restores and verifies captured clipboard representations. Those tests
do not establish OS rendered-document selection mutation or VoiceOver acceptance.
Full Rust workspace validation also passes. The scoped desktop suite took
116.813 seconds and workspace validation took 180.481 seconds.

Both maintained patches reconstruct exactly from their pinned source archives,
excluding Cargo.lock: **158 GPUI files** and **243 Base files**. Patch hashes:

- GPUI: `6ba31aec27d1116677122d27f3b674c23cacda8c3aa52ad85ca69e2d7927726f`
- Base: `769978bb4bfd44c2e1ae949c27fd09fea0df9b88145fb01f5c84d6d6cb6aa4be`

The native suite preceded only redundant test-reference removal and formatting
of the GPUI helper; final-source Clippy and workspace checks follow those edits.
Dune is not repeated in this cohort: no OCaml, protocol or build configuration
changed, and the parent checkpoint records the previous full Dune result.
Local checks establish no current-source Linux acceptance.

## Remaining acceptance

Connect `SetTextSelection` with deliberate shared-window selection/Copy ownership,
focus authorization and logical/physical endpoint reveal. In particular, test
that clearing an old geometry selection cannot deliver a queued event that erases
a newly installed caret. Complete actual macOS partial selection followed by
Copy, virtualized and atomic-owner cases, stale/reset/remount paths and teardown.
Cached AX replay, visual-line adjacency, actual VoiceOver, catalog, performance,
resources, distribution and release gates remain open. OCH-17/OCH-41 and the full
milestone goal remain in progress.

The [report archive](rendered-selection-authorization-och17/reports.tar.gz) and
[verified manifest](rendered-selection-authorization-och17/manifest.json) preserve
commands, logs (including the corrected failures), source hashes and patch
reconstruction results.
