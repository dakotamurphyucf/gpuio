# Custom table header descriptions — OCH-41

2026-10-03, macOS arm64, milestone worktree based on `83eb87e`.
The [renderer-slot contract](../design/table-renderer-slots.md) separates rich
header content from header/body row presentation and native geometry. This
checkpoint implements checked descriptions and their paired wire representation.
It does **not** expose rendered native custom headers yet.

`Table_header` has opaque targets and keyed generic content descriptions. A target
names a stable column, or a zero-based header level plus exact group membership.
Public construction canonicalizes member order; raw metadata must already be
canonical. Labels never identify groups. The supplied content key is independent
of the target, so its future native placement can retain identity across label
and display-order changes. Callback-bearing content is not serialized or compared.

`validate_all` accepts at most 320 unique keyed targets in the current checked
schema: 64 leaf columns plus four levels of up to 64 groups. It rejects missing
columns/groups, duplicate keys/targets and partial membership. Normal View/native
resource quotas will additionally apply at integration. This count is separate
from the managed body-cell budget.

Rust's bounded standalone decoder has matching validation and retained-byte
accounting. Independent column/Unicode-group fixtures are checked by both
languages. Tests cover truncation at every byte, trailing data, malformed tags,
invalid UTF-8/IDs/levels, empty/duplicate/unsorted groups, maximum 64-member payloads,
oversized counts/messages, rename/reorder stability and ambiguous repeated labels.
Core additionally checks the full 320-target schema and collection conflicts.

## Validation checkpoint

- Core library build and view-API expect tests pass:
  `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 lib/core/gpuio.cma` and
  `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @test/view_api/runtest`.
- Full protocol suite passes **368 tests/no skips**:
  `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j 2 -p gpuio-protocol`.
- Strict protocol lint passes:
  `GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --offline --locked -j 2 -p gpuio-protocol --all-targets -- -D warnings`.

Full OCaml tests, gallery and formatting also pass after the final bounds check:
`GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @runtest @fmt examples/gallery/main.exe`.
The initial formatting gate required a blank line between Dune copy stanzas;
that was corrected. Group construction now checks its size before mapping/sorting
members. No OS window opened. These
tests establish a description/codec foundation only. Native ownership/admission,
appended operations, reconciliation, header renderer/focus/gesture integration,
checked row presentation, Bonsai APIs and gallery examples remain to implement.
Physical macOS and the full catalog/release gates remain open.
