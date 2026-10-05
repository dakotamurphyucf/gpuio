# Paged-table qualification and admission repairs — OCH-17

Status: workload implementation and scoped native regression evidence. Full
100,000-row performance qualification, repeated runs and physical presentation
remain open. The reference viewport and declared performance budgets were not
reduced after the initial failures.

## Workload

[The public OCaml workload](../../examples/performance_table/README.md) uses
100,000 logical row identities, 64 columns and an application-owned four-page
cache (128 rows/page, at most 512 payload rows). Identity metadata remains
O(logical rows). Point updates load/evict payloads without resetting lineage;
this is not remote-I/O throughput or the boundary-append Table_paging adapter.
The 1168×720 table has 32-pixel rows, 64-pixel overscan and limits of 32 active
rows / 2,048 active cells. It traverses every row range both ways, keeps selection
across eviction, checks every column, reveals a middle cell, checks settled idle
and verifies window-owned cleanup. Smoke uses 1,030 rows and two seconds of idle.

The strict Python report reader shares native histogram parsing and process
collection with the loaded-list workload. Seven existing list-report tests and
three table-report tests pass. A Core expect test checks cache bounds, disjoint
jumps, partial final pages, source identity and invalid targets. Optimized build,
cache tests and formatting passed before native smoke testing. Reports preserve
failed runs; wall-clock mode cannot claim frame budgets or zero redraws.

## Compact plain-text cells

The initial smoke failed before measurement: a normal-height 64-column viewport
exceeded the bridge's 4,096-operation atomic limit. Every plain cell previously
allocated a styled container, metadata, a text child and a child splice.

`Bonsai.Table.Cell.text` now uses a compact Text leaf carrying its column and copy
text. Paired create/update operations change one cell, with equal display/copy
text. Rich/custom cells retain their ordinary View subtree. No operation-count,
message-byte, node, generation or retained-memory guard was increased. Both text
payloads remain charged. Native final validation still checks row/schema
ownership, stable column identity, absent child/handler/rich spans and matching
text; invalid updates publish nothing. Rich/plain changes replace that owner.
See [the retained contract](../design/data-tables.md).

Local checks pass:

- Core: 64 columns × 28 active rows can mount, replace a disjoint viewport and
  stream every cell within the unchanged atomic limit; rich/plain replacement
  and independent paired operation bytes also pass.
- Native table integration: 14 tests, including compact-cell ownership,
  duplicate/orphan/invalid operations, retained-byte charging and rollback.
- Protocol: independent OCaml/Rust bytes, bounded decoding, truncation and invalid
  text checks pass in the ten-test table suite.
- Actual macOS native host: compact text renders, GPUI-dispatched Copy reaches
  the real clipboard, native NSAccessibility cell values match, updates retain
  selection and cleanup releases table resources. The existing targeted AppKit
  keyboard/editor and table AX sequences also pass. Clipboard restoration and
  owned-child reaping use the existing harness. This is not VoiceOver acceptance.

## Horizontal visibility report

Smoke002 traversed all 1,030 rows both ways and retained selection, then timed out
waiting for column 17 to report fully visible. A production Host/TestPlatform
regression reproduced this without any desktop activation or occlusion.

At the failure, the horizontal scroll handle reported pane origin x=146, width
1,021 and offset −1,155. The header observer reported origin x=−1,009: its child
canvas had already moved with the scroll. Column measurement applied that offset
again and clipped the incorrectly shifted pane against the window. The correction
recovers the unscrolled pane origin before measuring bands. It changes reporting
geometry, not the scroll target, tolerance or budgets.

All 64 native column commands now pass, including published observations, along
with existing empty/pinned/partial-column cases. The first corrected test reached
column 42 and hit the bounded event queue because the test did not drain it;
adding a client-like observation drain resolves that fixture error. Production
backpressure remains unchanged. Temporary geometry prints were removed.

## Desktop runs and remaining evidence

Smoke003 timed out on a frame acknowledgement. Smoke004/005 record active=true at
history start and active=false before frame timeouts at rows 84/147. Those traces
do not establish the cause of the activation change or prove OS occlusion. All
failed reports and child cleanup are retained.

Smoke006 passes on macOS 14.5 / M1 Max after the header correction: all 1,030
rows forward/backward/materialized, all 64 columns, selection/reveal, 512 peak
loaded payload rows, 26 peak active rows / 1,664 cells, and zero retained owned
resources after close. Process peak RSS is 343,506,944 bytes; 422 native draws
have p95 14.189 ms / p99 14.557 ms. The two-second idle interval has zero draws,
active snapshots true/true and zero observed activation changes. This is a short
functional check, below the required 1,000 performance samples. Its executable
SHA-256 is `f027aff63028478ed8c3309927e6a0322cc3b0e383df61488eee853dd526e53a`.
An explicit two-second settling period was added afterward before measured idle,
matching the declared qualification procedure. Full measurements still need
warm-up and three independent optimized runs. No table performance acceptance
is claimed.

The full native unit suite passes 921 tests with two existing macOS private-bus
skips, and strict combined-feature all-target Clippy passes. The full protocol
suite passes 393 tests; full `dune runtest -j2`, optimized table build and `@fmt`
also pass. Exact commands and terminal outcomes are retained in the session
validation log. The later explicit settling wait requires the subsequent rebuild.

Session artifacts are under `scratch/agents/root-20261004-resumed/`:
`table-performance-smoke-00{1,2,3,4,5,6}/`, `table-compact-{native,protocol}-001.log`,
`table-compact-host-native-001.{log,json}`, `table-column-native-00{1,2,3,4}.log`
and `table-final-validation-001.log`. The parent source checkpoint is `1921ae3`;
these repairs and the workload are subsequent changes. Raw reports retain exact
executable hashes; checkout state is not embedded build identity.
