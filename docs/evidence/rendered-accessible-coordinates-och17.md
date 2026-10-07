# Prepared rendered accessibility coordinates — OCH-17

Local macOS arm64 checkpoint, 2026-10-07. Milestone 07 and OCH-17/OCH-41 remain
open. This implements logical coordinate conversion; rich accessibility-tree
publication, OS selection actions and physical VoiceOver are still outstanding.

## Implementation

`RenderedText` now prepares an accessibility coordinate index alongside its native
selection owners. Opaque part IDs include the exact preparation identity, so an
old part cannot address an equal-text replacement. The index is independent of
painted geometry and retains no strong native view or AST owner.

Conversions distinguish UTF-8 bytes, scalar/CRLF character indices and accessible
UTF-16 units. Invalid scalar boundaries, surrogate interiors, mid-CRLF offsets,
foreign part IDs and partial atomic-object endpoints are rejected. Shared edges
prefer the following part without losing the ordered edges of adjacent empty
objects. An empty document retains a zero-character caret part.

An empty atomic alternative contributes one U+FFFC to accessible text only. Its
two character edges map reversibly to the existing native object's ordered slots.
Native backward selection and Plain/Source Copy use the existing request path;
no replacement characters enter clipboard text, source or public protocol offsets.
A nonempty atomic alternative remains readable but admits selection only at its
whole-object edges.

ASCII parts allocate no character table. Other parts store one encoded width byte
per character plus eight-byte byte/UTF-16 checkpoints every 64 characters. Lookups
binary-search parts/checkpoints and inspect a bounded block of widths; they do not
scan or reshape the whole document per query. Iterating character lengths reads
the compact widths directly. Preparation reservation and retained accounting
include these allocations, with unchanged text-admission limits.

## Validation

Seven added native tests cover actual prepared headings, links, tables and code;
combining marks, joined emoji and bidirectional logical text; stale equal-text
identities; ordinary and atomic CRLF; empty-document and adjacent empty-object
edges; native backward selection with Plain/Source Copy; checkpoint boundaries;
and a 1 MiB generated alternative. The large mixed-text index retains less than
2 MiB more than its equivalent ASCII projection and stays within its preparation
reservation. This is an allocation invariant, not a process-memory benchmark.

All **1,125 native library tests pass; 2 existing tests remain ignored**. Actual
macOS native_editor/native_document, the full Rust workspace and strict all-target
Clippy pass through the isolated toolchain with two jobs. Exact Base reconstruction
matches 239 files, excluding Cargo.lock; patch SHA-256 is
`fe184874858fa3f85f11bfc7cc4bb6967513dcb88ba51a0da0f7e215dc1b5061`.

Initial fixture failures are retained: two missing Context arguments and a borrowed
string mismatch failed compilation; the first large-data fixture incorrectly used
1 MiB of displayed glyphs, exceeding the existing 64 KiB displayed-text allowance.
It was corrected to the intended admitted atomic alternative; limits were not
raised. The full-offset prototype passed 1,122 native tests before the compact
index replaced it. All original selection assertions remain enabled.

Full Dune `@all @runtest @fmt` passes, including a fresh run after the OCaml
shutdown repair below. The rebuilt gallery passes native reading order, actual
Copy and normal close with child exit zero and typed clipboard restoration.
Rust formatting, `git diff --check` and the documentation inventory
(429 sources / 266 reviewed groups / 0 pending, structural coverage only) pass.

The [archive](rendered-accessible-coordinates-och17/reports.tar.gz) and
[manifest](rendered-accessible-coordinates-och17/manifest.json) retain exact
commands, source/platform/binary hashes, regression failures and final results.
Archive contents are verified member by member.

## Native-close race found during validation

The first rebuilt gallery passed reading order and Copy but exited with an
uncaught OCaml `Closed` during normal close. Rust closes command intake and queues
`Stopped` after earlier output. That closure can occur between the OCaml worker's
event drain and its next submission; treating the synchronous response as fatal
was incorrect.

The runtime now records native transport closure, drops unsent commands, stops
further submissions and wakes its loop to continue draining. It waits for the real
`Stopped` event and normal cleanup rather than fabricating acknowledgements or
completing requests prematurely. Queued commands, motion updates, frame commits
and initial handshake use the same submission handling. Bridge disposal still
waits for the native runner to return. Busy preserves commands; other errors
propagate. The bridge interface and runtime design document this contract.

A deterministic Eio expect regression injects the terminal submission result at
the internal transport boundary with real runtime/window drivers. It reproduces
the exception before the fix, then checks queued/motion/frame paths, no premature
completion, exactly-once terminal cancellation and suppression of late commands.
A second test checks Busy, Malformed and successful dequeue. These are runtime
ordering tests, not simulated GUI acceptance. The actual rebuilt-gallery close
also passes afterward. Original failure logs remain in the archive; no failed
assertion was removed or promoted into an expectation.

## Remaining acceptance

The [design](../design/rendered-document-selection.md#prepared-accessible-coordinates)
requires logical parts to be integrated into the actual semantic hierarchy,
including unpainted blocks and independently selected embedded editors. Character
geometry must come from current shaping. Native selection is finalized during
paint, after ordinary accessibility-node preparation; publication must use that
finalized selection before the frame's tree update is sent. OS actions still need
native owner/window/visibility/modality/input/generation guards.

This checkpoint does not qualify rich TextRun publication, OS-set selection or
VoiceOver. Full catalog, broader interaction/reflow/virtualization, performance and
resources, physical presentation, notices, public API and distribution gates remain
open. Hosted run 37629820030 covers preceding e4630996, not this change. No new Linux
GUI, clean-machine or physical-presentation acceptance is inferred from these tests.
