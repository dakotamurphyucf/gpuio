# Managed table column observations — OCH-41

2026-10-03, macOS arm64, milestone worktree based on `83eb87e`.
`Table.Column_viewport` now exposes stable visible column IDs, pin status and
horizontal full/partial visibility. Core's managed-table constructor has an
optional callback; Bonsai exposes `Output.column_viewport`. The Collections →
Result table gallery displays the snapshot. See the
[contract](../design/table-column-viewport.md). Physical macOS and the broader
catalog/release gates remain open.

The native adapter measures actual pinned/scrolling header panes and native
widths/order/offset, intersecting table/ancestor/window geometry. The prior
buffered render range was unsuitable: it excluded pins, included overscan and
could retain stale single-column ranges. The first production Host regression
also exposed an incorrectly placed root measurement canvas; explicit top/left
positioning inside a relative root corrects its origin.

Appended Event75 preserves existing representations. Independent Rust/OCaml bytes
cover the new event. Decoder/admission checks bound unique IDs, pin order,
schema/query/transaction revisions and native owner/handler identity. Adjacent
mailbox snapshots coalesce only within the same revision/generation; input and
revision/query changes remain barriers. Retained string bytes count toward the
mailbox quota. Native publication follows paint and changes only with the column
snapshot or transaction revision; unchanged redraw/pixel movement is silent.

Core and Bonsai tests cover stale callbacks, unknown/duplicate IDs, incorrect pins,
empty-vs-absent snapshots, source/query/config changes and retained point updates.
The source remains application-owned and all active-row columns count against
the existing row/cell budget. This is layout observation, not OS occlusion.

## Validation checkpoint

The native suite passed **810 library tests**, with two existing private-D-Bus
skips, and **10 table admission tests**:

```
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j 2 -p gpuio-native --features native-image-tests,native-canvas-tests --lib --test tables
```

A final focused Host repeat passes additional checks for exactly one initial
publication, silent unchanged redraws, full-to-partial transitions, silent pixel
movement within the same partial bands, and narrow-window clipping. Empty-data,
single/all-pinned schemas, horizontal scrolling, reset invalidation and retained
native identity also pass. These are real Host layouts on TestPlatform, not
physical GPU/input evidence:

```
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j 2 -p gpuio-native --features native-image-tests,native-canvas-tests --lib empty_tables_measure_pins_partial_columns_scroll_and_schema_replacement
```

**365 protocol tests and 3 table-adapter unit tests** pass, with no skips. The
full OCaml suite/gallery build, strict Rust lint and formatting also pass:

```
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j 2 -p gpuio-protocol -p gpuio-table-adapter --lib --tests
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @runtest examples/gallery/main.exe
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --offline --locked -j 2 -p gpuio-native -p gpuio-table-adapter --all-targets --features gpuio-native/native-image-tests,gpuio-native/native-canvas-tests -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio check-fmt
```

The structural catalog audit and `git diff --check` pass. A fresh independent
installed-gallery build also passes, staging the public packages and compiling
and linking the complete gallery (`run=False`):

```
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --workspace /private/tmp/gpuio-table-columns-gallery-20261003
```

No OS window was opened by these checks. Linux desktop qualification remains
deferred OCH-47; required Linux non-GUI/release checks remain separate. This
capability does not complete OCH-41 or OCH-17.
