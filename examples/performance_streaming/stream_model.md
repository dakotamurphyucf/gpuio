# Pure canonical stream blocks and membership-preserving updates

[stream_model.ml](stream_model.ml) and its [interface](stream_model.mli) define
qualification data, including one inline expect test. Read `Row`/`Progress`/t/constants,
`fragment`, `append`, `validate` and test. `C = Gpuio.List_collection` is an immutable
collection; there is no Bonsai graph, native registration, timing loop or Eio task
here. [main](main.md) owns all scheduling/rendering/measurement.

After [setup](../../docs/development.md), from the root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune runtest examples/performance_streaming -j 2
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release examples/performance_streaming/main.exe -j 2
python3 scripts/measure_streaming_typing.py --build-profile release --smoke --output scratch/streaming-typing-smoke
```

The last command runs the macOS native collector, not a pure model test. It needs
a fresh output directory and the already built paired performance executable.
[README](README.md) separates smoke/full optimized qualification and Linux coverage.
No tests/builds/GUI runs were newly executed for this guide.

## Typed model and exact byte fixtures

Abstract `t` stores `(int, Row.t, Int.comparator_witness) List_collection.t` plus
an internal four-element updates array. Public `Row` contains stream/block/text;
`Progress` exposes updates and bytes. `create` begins with empty collection/all zero
counts. `fragment` formats stream/index and Unicode λ/世界, pads ASCII x and appends
newline to a total of 128 bytes. This is deterministic canonical text, not real model
output, rich Markdown or a network payload. `progress` reports each stream's
accepted count times 128 in stream order.

Four streams use IDs 0–3. `append` requires exactly `previous + 1` and sequence ≤ 1,000,000;
invalid stream/repeated/missing/out-of-order input returns Or_error before mutation.
It computes `block = (sequence - 1) / 16` and stable `key = block * 4 + stream`. First `fragment` in a
block appends one row with `C.splice`; remaining fragments use `C.set` to concatenate
text onto that row without changing order/membership. A full block has 16 × 128 = 2,048
bytes. `Array.copy` updates only the new model, preserving previous snapshots.

[Collection contracts](../../lib/core/list_collection.mli) distinguish membership
from integer keys/payload. A point update preserves `Item_ref` and identity while an
independent source/removal-reinsert retires it. Row ordering follows actual first
block arrival, not a sort by key; independent stream interleaving can vary that
order while canonical row data stays correct. Only current `t` is retained by main;
this pure API does not enforce how many historical snapshots a caller keeps.

## Validate every retained block and complete counts

`validate` requires each stream count equal nonnegative `updates_per_stream`, computes
`ceil(updates / 16)` blocks per stream and rejects missing/extra rows. It regenerates
each row's exact expected text, including final partial block, and checks key
formula plus String.equal. This verifies retained source bytes, not native paint,
input timing or how many uploads/render frames occurred. For the 120-second workload,
2400 updates per stream yield 600 full history rows and 1,228,800 bytes; four-second
smoke yields 80 each, 20 rows and 40,960 bytes.

The single `let%expect_test` first appends stream `2`, sequences `1` and `2` and confirms old
membership survives, text length 256, rejection of repeated sequence `2`, missing stream `1` sequence `1`, and
invalid stream `4`. It then interleaves `[3; 1; 0; 2]` for 19 sequences, validates canonical
contents and prints 8 rows plus 19 updates/2432 bytes for each stream. Assertions/
[%expect] establish pure model behavior, not scheduling or native frame coverage.
[Dune](dune) enables inline tests on the unwrapped model library separately from
the executable's qualified native backend.

Trace: producer `3` publishes sequence `17` → new block `1`, key `7` appends → other streams can
interleave without disturbing its identity → sequence `18` point-updates key `7` →
`Virtual_list` recomputes only admitted rows observing changed payload. For real data,
keep source validation/identity explicit and bound retained bytes/record counts;
this finite qualification model deliberately retains every canonical history block
rather than implementing eviction or a production chat persistence format.
