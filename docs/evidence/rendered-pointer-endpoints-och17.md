# Rendered pointer endpoint capture — OCH-17 / OCH-41

Native window selection now retains a checked text position alongside each
virtual block key. A process-unique preparation revision prevents an endpoint
from addressing a later equal-text document. Positions retain no old document,
layout or native owner. This advances the
[rendered selection plan](../design/rendered-document-selection.md); accessible
TextRuns, selection publication and OS actions remain unimplemented.

Prepared ordinary owners carry logical spans. Painted rich-flow fragments reuse
their canonical owner and local range; the view keeps one frame of already-shaped
layouts for hit testing. Unmapped runs are barriers, and points in gaps do not
borrow adjacent text. The map is built only for selectable bounded preparations.
Nonvirtual participants retain unrestricted Copy traversal instead of acquiring
an accidental block-zero restriction.

The current accessor captures bounded same-document pointer endpoints. It does
not replace native geometric selection/Copy or cover Select All, preserved
selection, multi-click, cross-participant selection or arbitrary custom-object
mapping. These paths must be unified before publishing a complete AX selection.

## Validation

On local macOS 14.5 arm64 / M1 Max, **1,092 native tests pass**, with two existing
ignored. New production-view tests on TestPlatform verify forward/backward
direction, separate ownership of duplicate paragraphs, equal-source replacement,
and retention after the anchor actually leaves the virtualized semantic tree.
Position tests additionally reject foreign revisions, split UTF-8 scalars,
interior CRLF and out-of-bounds offsets. These tests are not physical OS input.

Adding owner provenance to preparation admission exposed a real budget bug:
unused projection reserve could admit an oversized plugin vector. The unchanged
existing rejection test failed before separate profile/projection budget checks
and passes afterward. The original failure is retained. Preparation reservations
still shrink to retained allowances and release on drop; these are admission
units, not measured RSS.

Strict native all-target Clippy, full Dune `@all @runtest @fmt`, Rust formatting,
237-file Base reconstruction and the example inventory pass. The final native
document suite passes actual GPU selection/style/streaming checks, pointer and
multi-document Copy, selection policy and cleanup. The rebuilt gallery passes
the permanent selected-document shutdown check with Rust backtraces enabled.
All test children exit normally; captured clipboard representations are restored.
No VoiceOver or OS settings changed.

An initial compilation failed on an `Arc` coercion and was corrected. An earlier
full validation predates the final guard that avoids retaining unused endpoint
maps in unbounded views; the final native/lint/Dune run revalidates that guard.
The [archive](rendered-pointer-endpoints-och17/reports.tar.gz) and
[verified manifest](rendered-pointer-endpoints-och17/manifest.json) preserve these
failures and runs, exact commands, source hashes/patch and gallery binary identity.

Current-source hosted/Linux validation and the remaining rich AX, VoiceOver,
performance/resource, physical presentation and distribution requirements remain
open. This checkpoint does not complete either milestone ticket.
