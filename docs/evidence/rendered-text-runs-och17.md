# Native text runs and prepared position mappings — OCH-17

Local macOS arm64 work after `b02b6079`, 2026-10-07. This is partial native
accessibility integration, not full Document text, OS selection or release
acceptance. OCH-17/OCH-41 and milestone 07 remain open.

## Implementation

Existing Inline labels and link proxies publish TextRun children using the same
cached shaped clusters as pointer selection. Runs split at visual rows,
direction changes, discontinuities and missing geometry. Scalar character lengths
treat CRLF as one character; overlapping ambiguous clusters keep their text
without an invented rectangle. Rich-flow hard breaks retain their exact source
item and link membership without extra glyphs or focus outlines.

Run IDs incorporate prepared fragment endpoints. Equal-text replacement retires
old IDs while keeping native control IDs and actions stable. The Document filters
candidate bindings against its actual completed descendants using a scoped,
read-only GPUI traversal. Bidirectional position conversion checks the window,
exact preparation, published run and character bounds. It uses prepared coordinate
indexes and ordered source intervals rather than searching repeated text.

The [design contract](../design/rendered-document-selection.md#native-text-run-publication-integration-in-progress)
states the limits: these are last-prepaint mappings, not current visibility or
action authorization. Offscreen content without a realized run is still absent
from this publication path.

## Evidence and scope

- Current mapping code: **1,137 native library tests pass; 2 existing tests remain
  ignored**. Strict all-target Rust lint, formatting and diff checks pass.
- Tests cover Unicode/combining characters/RTL, styled links with hard and soft
  breaks, source-position round trips, invalid indices, equal-text replacement,
  cross-window rejection, virtualized replacement/resource refresh, native control
  stability and exclusion of unrelated sibling subtrees.
- Full Rust workspace and Dune `@all @runtest @fmt` passed for the preceding leaf
  publication stage. Source hashes and the exact changed-source snapshot for that
  stage are retained. Those broader checks have **not** been repeated after the
  prepared binding addition; final integration validation remains required.
- The actual macOS editor/document command timed out after 300 seconds with
  HIServices/LaunchServices connection errors in the restricted sandbox. The
  driver killed and reaped its owned process group and unwound clipboard
  preservation. This provides **no current-change GUI acceptance**.
- The documentation inventory remains 429 sources / 266 reviewed groups / zero
  pending. That audit proves structural coverage only.

Both vendor trees reconstruct exactly from cached, hash-verified pinned archives,
excluding Cargo.lock:

| Tree | Files | Patch SHA-256 |
| --- | ---: | --- |
| GPUI | 157 | `37301e87dff2eaa4ff0ccdfccc23c17a27d8f1521feb6c75d613f120386014b8` |
| Base | 242 | `279bdd638e160133d69092feb2cc0322954b309bb40c06e70bf695f2a57dd81a` |

Initial failures remain in the logs. A hard newline was absent from rich-flow
TextRuns and is now explicitly published. Six pointer fixtures initially counted
both labels and their new run children; their queries now target the intended
owners, with the original behavior assertions retained. An outdated scratch patch
helper omitted the existing `presentation-diagnostics` Cargo feature; reconstruction
caught it, and regeneration now preserves the complete prior patch before appending
the reviewed source delta. Fresh downloading failed under network restrictions;
the cached archives have the same pinned hashes.

The [archive](rendered-text-runs-och17/reports.tar.gz) and
[manifest](rendered-text-runs-och17/manifest.json) retain commands, results, failures,
source hashes and reconstruction evidence. Archive members are hash-verified.

## Remaining release work

Complete structural separators, atomic alternatives, offscreen logical text and
visual-line adjacency under the real semantic hierarchy. Publish selection after
paint and implement guarded OS selection actions, then validate physical platform
behavior and VoiceOver. Catalog, performance/resources, notices, presentation and
distribution gates remain open. No current-source hosted, Linux GUI, clean-machine
or physical-presentation acceptance follows from this checkpoint.

The current session cannot commit/push through its read-only `.git` or publish the
Linear update because the connector requires approval and the policy is `never`.
The pending update is retained locally; this document does not imply publication.
