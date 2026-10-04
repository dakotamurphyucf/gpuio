# Structural tables — OCH-41

2026-10-03, macOS arm64, milestone worktree based on `83eb87e`.
`Gpuio.Table_view` (also `Gpuio_bonsai.Table_view`) now composes ordinary Views
from checked keyed Cell/Row/Section descriptions. Header/body/footer rows share
equal grid tracks; column spans drive both actual layout and zero-based logical
indices. Captions have their own semantics and do not replace the explicit table
name. The gallery adds Structural table with grouped headers, row headers, merged
cells, footer, caption, reversal and working native buttons. See the
[contract](../design/structural-tables.md).

## Implementation evidence

- Accessibility role tags 19..25 append Table, Row_group, Table_row, Table_cell,
  Column_header, Row_header and Caption. Old role encodings and operations remain
  unchanged. Typed totals/indices/spans are bounded; semantics are admitted only
  on ordinary Container nodes. Independent paired metadata fixtures cover all
  seven roles. Native decoding rejects malformed/trailing/truncated metadata.
- Public composition validates sibling keys, exact row coverage and bounded
  rows/cells/sections before allocating generated wrappers. Row indices include
  header/footer sections; column indices account for preceding spans. Tests compare
  the actual submitted grid placement to semantic indices/spans. Keyed row
  reversal updates semantics without recreating embedded native controls.
- The production Host test lays out a three-column table, measures a two-column
  cell at 200 logical pixels and its neighbor at the correct position, then
  inspects actual AccessKit counts/indices/spans. A button inside a cell retains
  its native Click action and emits the normal bridge event. Metadata clearing
  preserves that button's identity; full removal retires the table.
- Atomic native admission rejects invalid spans and attempts to replace a
  control's role with structural-table metadata; rejection preserves the previous
  tree, revision and retained-byte accounting. Window close releases retained data.
- A standalone AppKit test uses a retained NSView adapter without an OS window,
  activation or external AX observer. Actual NSAccessibility queries return table
  counts, column/row header lists and merged-cell ranges. In-place updates change
  those ranges; removed nodes and dropped adapters invalidate retained references.
  This is native macOS bridge evidence, not a VoiceOver walkthrough.

## Validation checkpoint

Native validation passes **811 library tests**, with two existing private-D-Bus
skips, **4 accessibility admission tests**, and the standalone AppKit fixture:

```
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j 2 -p gpuio-native --features native-image-tests,native-canvas-tests --lib --test accessibility --test accessibility_structural_table
```

Additional final local checks pass:

- Full OCaml expect suite and gallery executable:
  `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @runtest examples/gallery/main.exe`.
- Full Rust protocol suite, **366 tests/no skips**:
  `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j 2 -p gpuio-protocol`.
- Strict native/table-adapter all-target lint:
  `GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --offline --locked -j 2 -p gpuio-native -p gpuio-table-adapter --all-targets --features gpuio-native/native-image-tests,gpuio-native/native-canvas-tests -- -D warnings`.
- `GPUIO_JOBS=2 ./scripts/gpuio check-fmt`, `git diff --check`, and
  `python3 scripts/audit_component_catalog.py` pass. The catalog audit verifies
  inventory/provenance, not behavioral completeness.
- Fresh independently staged public-library/gallery build:
  `GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --workspace /private/tmp/gpuio-structural-table-gallery-20261003`.
  Result: `INDEPENDENT_EXTENSION_CONSUMER_PASS example=gallery run=False`.
  No packages were installed into another switch.

The physical walkthrough `scripts/test_gallery.py --section structural-tables`
now checks table/header relationships, merged ranges, native actions and retained
control identity after reversal. Its Python syntax was checked; the walkthrough
has **not** run. No OS window was opened. Physical gallery/VoiceOver/GPU and
broader catalog/release gates remain open. Linux desktop qualification is deferred
OCH-47; required non-GUI Linux release checks remain separate. OCH-41 and OCH-17
remain in progress.

## External macOS range-query repair — 2026-10-04

The physical gallery reached the Memory row header but external AX returned no
`AXRowIndexRange`. Column-header ranges and table/header counts already worked.
The adapter implemented the row-header getter but its
`isAccessibilitySelectorAllowed:` gate only admitted Row, Cell and ColumnHeader.
The earlier fixture invoked getters directly and therefore missed external
dispatch's role check.

The headless AppKit fixture now checks both range selectors on RowHeader, their
zero-based values, and selector retirement after removal. It fails before the
repair and passes afterward. `table-row-header-ranges.patch` extends just those
two getter gates; desired-selection opt-in remains unchanged. There is no
dependency version change. The pinned adapter reconstructs exactly: 13 upstream
files, ten ordered patches without offset/fuzz, and both preserved license hashes.

The external structural-table walkthrough then passes on macOS 14.5 arm64:
6 rows, 3 columns, 5 column headers, 3 row headers, every zero-based row/column
range including merged cells, native Memory/Streaming actions, reversal with
retained keyed button identity, page retirement and clean application shutdown.
The harness now explicitly searches within AXTable for its caption. The virtual
table walkthrough separately reveals its viewport before requiring virtual cells
to mount; neither adjustment weakens row or range assertions.

Commands:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked --offline -j2 \
  -p gpuio-native --test accessibility_structural_table
python3 scripts/verify_accesskit_macos.py --archive /path/to/accesskit_macos-0.26.3.crate
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/gallery/main.exe @fmt
python3 scripts/test_gallery.py --section structural-tables \
  --images scratch/agents/root-20261004-resumed/structural-tables-images-002
```

Logs in that local session directory: `row-header-native-before-001.log` (expected
regression failure), `row-header-native-after-001.log`,
`row-header-reconstruction-001.log`, `row-header-gallery-build-001.log`,
`gallery-structural-tables-001.log` (external failure) and
`gallery-structural-tables-002.log` (pass). The latter executable also includes
the recursive renderer stack reduction. This is scoped AppKit/external AX
acceptance, not VoiceOver speech, Linux desktop or performance qualification.

The complete Collections walkthrough also passes (`gallery-collections-004.log`),
including the managed message list, outline tree, virtual result table, structural
table and searchable list. Final native validation reports 920 passed and two
existing macOS private-bus skips; strict combined-feature all-target lint and
Rust formatting pass (`row-header-*-001.log`).
