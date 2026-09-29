# OCH-41 diff controls foundation

Status: parser, projection, typed API and live event routing validated locally;
mounted controls remain pending. See the [implementation design](../design/diff-controls.md).

The parser checkpoint `ed2a368` has native two-file gutter/GPU evidence in the
[highlighting ledger](subtree-highlighting-och41.md#file-boundaries-in-native-diff-folding).
The following projection work is separately tested without opening a GUI.

`rust/native/src/document_diff_projection.rs` constructs visible text and row
maps from the bounded original diff. It keeps collapsed headers, removes hidden
body rows, counts preview limits across expanded body rows, and stops before a
subsequent file/hunk header when that limit is exhausted. Eight projection tests
verify:

- Original/display UTF-8 byte and caret mapping, including joins and preview EOF.
- Selection/copy intervals that never silently span hidden source rows.
- Selection direction and exact-byte preservation, or clearing when a projection
  would hide selected text or reveal extra text inside the selection.
- Syntax-run colors/emphasis and native hunk-candidate display-row mapping.
- Exact file/hunk boundaries, zero limits, duplicate labels and collapsed counts.
- Every Unicode-boundary prefix of a streamed multi-file patch under five limits.
- Empty/malformed metadata and visible non-scalar endpoint rejection.
- A6000-body-row file with collapse retaining one header and under1KiB of counted
  projection storage; a20-body-row preview retains22 rows and under4KiB. These
  counts cover the projection object, string and row-vector capacities. They
  exclude canonical snapshots, temporary construction, native editor buffers,
  allocator overhead and process RSS; they are not the application memory budget.

Local macOS14.5 arm64 validation:

```sh
./scripts/gpuio exec cargo test -p gpuio-native --lib document_diff --features native-image-tests --locked -j2
./scripts/gpuio exec cargo clippy -p gpuio-native -p gpuio-protocol --all-targets --features native-image-tests --locked -j2 -- -D warnings
./scripts/gpuio exec cargo fmt --all --check
git diff --check
```

All14 targeted tests pass: eight projection cases plus six parser cases. Strict
Clippy and formatting/whitespace checks pass. These are algorithm/compilation
checks, not native controls, clipboard, IME, screen-reader, Linux desktop or
release acceptance. No public capability has been added. The remaining paired
protocol/API, lifecycle, native controls, source-page/search, resource charging,
gallery/consumer and release gates are enumerated in the design.

## Typed values, codecs and native ownership

`Document.Diff` now provides validated configuration constructors and private
file/line/event values. `Document_diff_wire` and Rust `document_diff` have paired
standalone codecs, with bounded variable-length readers and source/configuration
provenance. No operation/event variant has yet been added to the live transport,
and a document view cannot consume these controls yet.

Four OCaml expect cases and four Rust protocol tests cover three independent
configuration fixtures and five event fixtures, exact field order/UTF-8 bytes,
all truncated prefixes/trailing data, invalid labels/coordinates/payload lengths,
ownership-aware intent validation and exact256KiB encoding limits. The fixtures
include both old/new line numbers and an unnamed empty-payload annotation. New
expectations were inspected against the intended bytes and the independently
constructed Rust values; no automated promotion was used.

Three native state tests exercise the same typed configurations with parsed diffs
and visible projections. Controlled actions leave values unchanged until a config
update. Managed seeds remain initial values across same-generation updates; changed
steps take effect on the next show-more activation. Generation/ownership changes
reset values, decreasing generations reject atomically, and removed-file overrides
are pruned while bounded future seed keys remain available. Duplicate labels share
collapse state. A stale preview count cannot increment a newer managed limit.

Commands for this stage:

```sh
./scripts/gpuio exec dune runtest test/document_diff test/protocol test/text_source
./scripts/gpuio exec cargo test -p gpuio-protocol document_diff --locked
./scripts/gpuio exec cargo test -p gpuio-native --lib document_diff --features native-image-tests --locked -j2
```

All pass locally on macOS14.5 arm64 (17 native diff/parser/projection/state tests;
four Rust protocol cases and four new OCaml expect cases). This remains algorithm
and codec evidence, not mounted native input or release acceptance. Runtime source
registration/handler/configuration-epoch rejection, resource charging, rendering,
Core/Bonsai view integration, gallery and consumer acceptance remain open.

Consolidated local workspace library validation also passes:461 Rust tests, with
two private-bus cases excluded by the library command. Strict native/protocol
all-target Clippy with `native-image-tests --locked -j2 -- -D warnings` passes
after correcting two style diagnostics in the new encoded-size accounting.
The four protocol cases pass again after that correction. Jane Street OCaml
formatting, Rust formatting and whitespace checks pass. No GUI test was run for
this contract/state-only change.

## Live transport and lifecycle routing

The append-only live protocol now carries `SetDocumentDiff` (operation58) and
`DocumentDiffEvent` (event65). Core/Bonsai expose `Document.Config.create ~diff`
and `View.document ~on_diff`; navigation and diff observations share one native
handler. Diff settings retain a monotonic per-mount epoch across replacement,
clearing and restoration; cosmetic and callback-only changes preserve it.

Two new OCaml expect cases verify independent envelope bytes, truncation/trailing
rejection, shared callbacks, current closure dispatch, config/source/handler
replacement, unaccepted transactions and teardown. A runtime expect case exercises
source publication races: begin/chunk/unsent publish cannot authorize an event;
the exact pending publish can. Reset retires older generations immediately, and
accepted generation boundaries reject invented revisions from before the reset.
Release, closure and reused resource IDs reject stale events.

One Rust protocol integration case independently verifies those envelopes and
rejects nonpositive epochs. Three headless native integration cases verify atomic
tree/config admission and retained charging; handler/source/epoch/revision fences;
and a128-event queue of maximum-size line payloads. All three path copies and line
text count toward queue bytes; event129 is rejected, order survives bounded1MiB
encoded drain batches, and window-output retention clears only after the drain.

Local macOS14.5 arm64 commands:

```sh
./scripts/gpuio exec dune runtest test/document_diff test/runtime test/protocol test/text_source test/highlight
./scripts/gpuio exec cargo test -p gpuio-protocol -p gpuio-native --test document_diff --features native-image-tests --locked -j2
./scripts/gpuio exec cargo test --workspace --lib --features native-image-tests --locked -j2
./scripts/gpuio exec cargo clippy -p gpuio-native -p gpuio-protocol --all-targets --features native-image-tests --locked -j2 -- -D warnings
```

All pass: four Rust integration cases and461 workspace library tests (two
private-bus cases excluded by this command), plus the scoped OCaml suites and
strict Clippy. These checks opened no windows. No capability is newly advertised;
per-file native controls, projected editor integration, input/accessibility,
gallery, installed-consumer and release acceptance remain open.

## Mounted visible editor and resource admission

The retained document presenter now consumes live diff settings and projects one
read-only native editor. It keeps canonical snapshots separate from displayed
bytes, projects cached syntax and hunk candidates, and supports the `word_diff`
emphasis switch. Explicit raw-source search and the Diff view return preserve
controlled collapse/preview values. The new native tests drive settings through
real retained-tree transactions; they do not claim per-file/show-more action UI.

The extended `native_document` harness passes on macOS14.5 arm64:

- Collapsed and preview-limited text actually reaches the mounted editor without
  replacing its entity or canonical source.
- A backwards selection in a surviving file retains its exact bytes/direction
  after preceding rows reappear; a selection crossing newly hidden rows clears.
- Navigation maps visible Unicode bytes back to the correct original path/line.
- Searching hidden canonical text opens raw source and selects its exact bytes;
  returning restores the configured diff projection.
- Paging while a newer source is preparing keeps the installed old snapshot.
  A3000-line projection uses bounded native pages and correct second-page source
  navigation. Pending parse keeps installed Flow/Viewport geometry stable.
- Clearing settings disposes projection/controller/page-text reservations.

The extended `native_highlight_document` harness passes actual GPU checks for
per-file collapse moving the surviving match to its projected row, preview-zero
removing washes, selection painting above highlights, native gutter folding on
projected hunk rows, old painter disposal and restoration of the raw full source.
Existing code/Markdown/image/bidi/scroll/state-style/fold regressions also pass.
These runs used background windows, with native input dispatch for gutter checks;
they do not qualify OS keyboard focus, clipboard/IME or screen-reader behavior.

Additional mounted memory is admitted before construction in the same64MiB pool
as parser work. Conservative units include projection/controller/runs/page buffers,
old and replacement state, and seeds retained from earlier managed configs. A
pool test verifies exact shared-budget exhaustion, rejection after closure and
release on drop; another checks that replacing a large managed declaration with
an empty config keeps its original seed charged until generation reset.

Commands:

```sh
./scripts/gpuio exec cargo test -p gpuio-native --test native_document --features native-image-tests --locked -j2
./scripts/gpuio exec cargo test -p gpuio-native --test native_highlight_document --features native-image-tests --locked -j2
./scripts/gpuio exec cargo test --workspace --lib --features native-image-tests --locked -j2
./scripts/gpuio exec cargo test -p gpuio-native --lib document_ --features native-image-tests --locked -j2
./scripts/gpuio exec cargo clippy -p gpuio-native -p gpuio-protocol --all-targets --features native-image-tests --locked -j2 -- -D warnings
```

The first two GUI runs and workspace/Clippy checks pass. The full library run has
462 passing tests and two excluded private-bus cases; the subsequent targeted
run passes38 cases including the managed-seed accounting regression. The native
document harness passes again after that final accounting change. Interactive file headers,
show-more and richer line actions, path-based syntax parity, gallery/consumer,
application budgets and release gates remain open. No new capability is advertised.

## Native Show more and action provenance

The footer now displays the expanded-but-preview-hidden row count and a native
Show more button. Managed activation raises the limit locally; controlled
activation leaves the view unchanged and queues an intent. Both use the existing
asynchronous diff event envelope. Native-managed operation also works without a
registered application callback.

The extended `native_document` test passes measured-position pointer dispatch,
Enter/Space and an actual macOS AXButton press. It verifies exact visible/hidden
counts, managed applied limits versus controlled intent, source revision/generation
in the queued observation, and focus returning to the editor when either managed
expansion or controlled acceptance removes the footer. Captured old-page, old-config
and old-handler actions reject; whole-document collapse, source reset and release
also reject. A generation reset restores the managed initial limit. The test waits
for the asynchronous AppKit action delivery before checking its queued event.

Rendered actions hold snapshot/page/epoch/handler provenance. The shared admission
check additionally requires the current presenter, node visibility/modal eligibility,
source registration, and an open nonoverloaded window. No synchronous OCaml callback
occurs during layout or native activation.

Local macOS14.5 arm64 validation passes:

```sh
./scripts/gpuio exec cargo test -p gpuio-native --test native_document --features native-image-tests --locked -j2
./scripts/gpuio exec cargo test --workspace --lib --features native-image-tests --locked -j2
./scripts/gpuio exec cargo clippy -p gpuio-native -p gpuio-protocol --all-targets --features native-image-tests --locked -j2 -- -D warnings
./scripts/gpuio exec cargo check -p gpuio-native --features native-tests --locked -j2
```

The library run has463 passing tests and two excluded private-bus cases. Formatting
and whitespace checks pass. This is local native input/AX action evidence, not a
VoiceOver walkthrough, physical keyboard/IME, Linux GUI or release acceptance.
Interactive file headers and richer line activation remain the next mounted work;
gallery/consumer, style parity, application budgets and release gates stay open.
