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
