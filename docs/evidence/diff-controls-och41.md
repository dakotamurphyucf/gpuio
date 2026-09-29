# OCH-41 diff controls foundation

Status: parser and projection foundation validated locally; public controls and
mounted integration remain pending. See the [implementation design](../design/diff-controls.md).

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
