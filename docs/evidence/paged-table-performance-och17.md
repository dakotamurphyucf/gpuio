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
validation log. The later explicit settling wait passed the subsequent optimized rebuild and
smoke warm-up.

Session artifacts are under `scratch/agents/root-20261004-resumed/`:
`table-performance-smoke-00{1,2,3,4,5,6}/`, `table-compact-{native,protocol}-001.log`,
`table-compact-host-native-001.{log,json}`, `table-column-native-00{1,2,3,4}.log`
and `table-final-validation-001.log`. The parent source checkpoint is `1921ae3`;
these repairs and the workload are subsequent changes. Raw reports retain exact
executable hashes; checkout state is not embedded build identity.

## First full run and diagnostics follow-up

The settled optimized source `5b29573` was preserved as executable SHA-256
`111e9a17620137ab243382e3fd004b9be4db7a98b3ff9411f7347922bfc154e6`.
Its fresh smoke warm-up passes. Full001 then fails with a native frame
acknowledgement timeout after 727.693 seconds, whole-process CPU 827.876 seconds
and peak RSS 548,536,320 bytes. It does not produce complete traversal histograms
or accepted coverage, so these resource observations are not a performance pass.
No simultaneous local compilation or second owned GUI ran. Independent source
editing and portable Python process tests continued; desktop activity was not
controlled. All owned processes were reaped.

The full driver originally emitted frame-failure geometry only in smoke mode,
leaving this failure's row/stage and activation unknown. It now emits bounded
failure diagnostics in either mode and progress once per 10,000 visited rows in
each direction. The timeout, workload and acceptance thresholds are unchanged.
A read-only power-log inspection found no display-sleep event near the failure;
that does not identify the cause or establish uninterrupted visible rendering.
The owner has chosen ordinary visible benchmark windows while away for subsequent
long runs. These require fresh reports, not reassignment of this failed attempt.

Artifacts: `table-performance-warmup-001/`, `table-performance-full-001/`,
`table-qualification-001.log`; the subsequent diagnostic build passes in
`lifecycle-build-002.log` alongside the separate lifecycle workload. That build's
first attempt failed on an OCaml 5.3 reserved-word identifier in the new lifecycle
example; it was renamed before the passing build. No compiler pin changed.

## First passing full run — source b75ffa2

After a fresh smoke warm-up, full002 passes with the normal visible window and
an owned, temporary display/system-idle assertion, released when the batch exits.
The executable SHA-256 is
`84e85e810bdaf1ba4eb493e48a41adc1e7a3d6b4d9065b74d9b49b0fa0c438e3`.
No simultaneous local compilation or second owned GUI ran; source editing and
lightweight portable tests continued. The owner selected this desktop arrangement.

Both directions visit all 100,000 rows, every row materializes, all 64 column
bands are observed, selection survives eviction, and the middle-row/final-column
reveal succeeds. Peak active rows/cells are 26/1,664, with at most 512 current
application payload rows, 1,564 page loads and 1,560 evictions. All declared
resource and queue counters return to zero.

The 28,993 native draws have p95 13.844479 ms and p99 14.114815 ms. Peak process
RSS is 606,683,136 bytes (578.578125 MiB); whole-process wall time is
1,551.870371 seconds and CPU time 1,722.277686 seconds. The settled 60-second idle
interval records zero native work; asynchronous activation snapshots remain true
with zero observed changes. These are native work histograms, not physical
presentation timing or a GPU/IOSurface allocation census.

The complete report, including histograms, machine/display metadata and empty
`budget_failures`, is [the full002 report](paged-table-run-002-och17.json). Command:

```sh
python3 scripts/measure_table_history.py \
  --executable scratch/agents/root-20261004-resumed/performance_table-b75ffa2-release.exe \
  --build-profile release --timeout 3600 \
  --output scratch/agents/root-20261004-resumed/table-performance-full-002 \
  --check-budgets
```

This is one successful full run; repeated-run acceptance remains open. The prior
failed attempt is retained above and has not been relabeled.


## Second passing full run — preserved source b75ffa2

Following its own successful smoke warm-up, full003 passes using the same
preserved executable and SHA-256 as full002. The [unaltered full003 report](paged-table-run-003-och17.json)
records observation checkout `2fdf53b97ffabbb9a861af1ee4a81d3d8cf791a8`
and its clean state at launch; those fields do **not** identify the executable's
source. The actual executable source remains
`b75ffa29a2219513b2904a131e6d88f176cc7286`. Subsequent document-workload source
editing, formatting, an offline manifest-only lock update and short portable
collector tests continued while the preserved binary ran. No local compilation
or second owned GUI ran. The normal visible window and temporary idle assertion
were retained for this repeat batch.

Coverage and retention match full002: 100,000 rows each direction and materialized,
64 columns, peak 26 active rows/1,664 cells, 512 current payload rows,
1,564 page loads and 1,560 evictions. All declared cleanup counters return to zero.
The 29,049 draws have p95 **13.819903 ms**, p99 **14.032895 ms**. Peak RSS is
**604,061,696 bytes** (576.078125 MiB). Whole-process wall time is 1,534.740790 s;
CPU is 1,673.827151 s user plus 30.064030 s system. The 60.003985 s idle interval
has zero native work, with observed activation true at both boundaries and zero
observed changes. All declared budgets pass. Physical presentation and GPU memory
remain separate requirements.

Command matches full002 above with output `table-performance-full-003`; the
preceding warm-up uses `--smoke` and omits `--check-budgets`. Two independent full
passes are now complete; the third repeat remains pending. Full001 remains a
retained failure with unproven cause.
