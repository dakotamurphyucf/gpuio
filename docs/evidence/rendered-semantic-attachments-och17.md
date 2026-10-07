# Rendered block semantic attachments — OCH-17

Local macOS arm64 checkpoint, 2026-10-07. OCH-17/OCH-41 and milestone 07 remain
open. Prepared top-level owners now bind to real GPUI accessibility subtrees.
Rich TextRun publication, OS selection actions and physical VoiceOver acceptance
remain unimplemented or unqualified.

## Implementation

The prepared projection records the semantic owner for each original root block
slot, including empty slots for nontext rules and definitions. Filtering those
blocks cannot shift the mapping. Storage is included in projection admission and
retained-byte accounting.

A native scope wraps each mapped block without adding a layout box. Its element
key uses the original block slot, not the preparation identity, so replacing
prepared content does not remount unchanged child controls. The existing Document
node filters recorded candidates against its finalized direct children. Rows
prepainted only for measurement cannot become attachments; ambiguous owners are
excluded. Existing heading/link/control descendants and their actions remain
under their actual native parents.

The frame stores plain IDs and a weak projection reference. Snapshots require the
same window and exact preparation. The accessor is a read-only mapping from the
last completed prepaint, not proof of current visibility after hide/unmount or
authorization to dispatch an OS action. Nested semantic ownership and text-run
placement are still separate work. See the
[design contract](../design/rendered-document-selection.md#real-block-subtree-attachments).

## Validation

Three new tests cover original block slots, real native subtrees and virtualized
attachments. GPUI TestPlatform checks verify heading/link ancestry, a working
code-toolbar action, stable native IDs across equal-text replacement, independent
windows, and 200 logical paragraphs with only realized native attachments.
Virtualized replacement and renderer-resource refresh update semantic owners
without changing native IDs. Disabling/re-enabling accessibility clears/restores
the mappings. These are native framework tests, not physical OS accessibility
acceptance.

All **1,135 native library tests pass; 2 existing tests remain ignored**. Actual
macOS editor/document tests, full Rust workspace tests, strict all-target Clippy,
full Dune `@all @runtest @fmt`, Rust formatting and `git diff --check` pass through
the isolated toolchain with two jobs. Broad production checks completed before a
test-only extension of the virtualized fixture; the final native suite, strict
lint and formatting were rerun afterward. The rebuilt gallery passes native
reading order, actual Copy and normal close with child exit zero and typed
clipboard restoration. Documentation coverage remains 429 sources / 266 reviewed
groups / 0 pending; this is a structural audit.

The maintained Base patch reconstructs all **241 files** exactly, excluding
Cargo.lock. Patch SHA-256:
`3b5779b710b0f5a3b4352bf7fa6e28a30cf3bcc6f960252d0c631f2f635721fc`.

Failed fixture attempts are retained. They used an incorrect ActionRequest field,
exceeded the existing 256-block parser limit, inspected stale bindings after an
implicit redraw, or borrowed the test application twice. The resource-refresh
extension initially added a new renderer name, changing parser configuration and
correctly invalidating the projection. It now registers that name initially and
updates its resource closure in both state and TextView. That failure did not
demonstrate a production resource-refresh defect. No admission limits or acceptance
assertions were relaxed.

The [archive](rendered-semantic-attachments-och17/reports.tar.gz) and
[manifest](rendered-semantic-attachments-och17/manifest.json) preserve commands,
results, source/platform/binary hashes and failed attempts. Archive member hashes
are independently checked during creation.

## Remaining work

Publish complete logical TextRuns, including virtualized content, under the real
semantic hierarchy without duplicate controls or hidden flat text. Reuse cached
shaped geometry, finalize selection after paint, and guard OS actions with current
owner/window/visibility/modality, input policy, preparation and interaction epoch.
Then qualify real OS selection and VoiceOver. This checkpoint does not establish
those paths, broader catalog/performance/resource acceptance, physical presentation,
clean-machine distribution or release readiness. Hosted run 37645873138 covers
parent `fe2e5482`, not these changes; Linux GUI qualification remains deferred.
