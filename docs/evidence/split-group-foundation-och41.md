# Flat split-group data and geometry foundation — OCH-41

2026-10-02, macOS arm64, base `83eb87e` plus uncommitted milestone work.
This is a foundation checkpoint, **not a working flat-group view**. The public
Core module explicitly says that the view/renderer is under construction.
The existing `Split_pane` still supplies the working two-pane component.

The [design](../design/split-group.md) records the full remaining contract:
stable panel identity, bounded constraints, native-owned geometry, one-shot
resize requests, per-boundary keyboard/accessibility/captured dragging, passive
handle decorations, lifecycle and a public gallery. The pinned group's indexed
state and unindexed paint callback require adaptation; this implementation adds
a separate bounded native model without changing the existing base fork or
two-pane adapter.

Implemented here:

- `Split_group.Id`, `Panel`, `Resize_request`, `Config`, `Source`, `Snapshot` and
  checked conversion. IDs, labels, ranges, unique panel count, request serials
  and current snapshot order/ranges have explicit validation.
- Matching standalone OCaml/Rust records and bounded decoders. Two hand-assembled
  independent bin_prot fixtures are checked by both language implementations;
  every truncation, trailing bytes, malformed numbers/text/IDs, duplicates and
  over-limit collections have Rust codec coverage. These records are **not yet
  attached to a tree operation, native event or capability**.
- Native pure fitting with complete per-panel min/max bounds, mixed explicit/
  flexible initial preferences, proportional retained preferences, hidden sizes,
  infeasible overflow/unused space, boundary transfers through neighboring panes,
  programmatic resizing and feasible boundary ranges.
- A retained native state model maps preferences by ID across reorder/insertion/
  removal, preserves hidden sizes, applies reset generations and constraint
  updates, stages noncommitting drag previews, restores cancelled gestures and
  emits one final snapshot on a completed change. Requests wait for usable,
  visible targets, cancel when removed/cleared and do not replay after a reset.
  Zero layout preserves preferences. View routing, actual capture and native
  widgets remain to be integrated.

Validation (`GPUIO_JOBS=2 ./scripts/gpuio exec`):

- `dune build -j2 lib/core/gpuio.cma`: pass.
- `dune build -j2 @test/view_api/runtest @fmt`: pass; includes three new expect
  tests for exact public values, validation and current-snapshot membership.
- `cargo test --offline --locked -j2 -p gpuio-protocol --test split_group`:
  **2 passed**.
- `cargo test --offline --locked -j2 -p gpuio-native --test split_group_geometry --test split_group_state`:
  **9 passed**. A deterministic generated test checks 2,000 groups (up to 64
  panes each), all bounds, feasible/infeasible fitting, conservation, hidden
  retention and boundary extrema in both directions.
- `cargo clippy --offline --locked -j2 -p gpuio-native --all-targets --features native-image-tests,native-canvas-tests -- -D warnings`: pass.
- `cargo fmt --all -- --check`, catalog structural audit and `git diff --check`:
  pass. The audit establishes source inventory, not feature completion.

Formatting corrections separated interface documentation comments and added a
blank line between the two new Dune fixture copy rules; the final checks passed. Logs and exact working
notes are under `scratch/agents/root-20260929-m7-resumed/`, `split-group-*`.

Remaining: tree/event protocol integration and atomic admission, Core/Bonsai view
construction/reconciliation/event fencing, native same-frame panel layout,
per-handle focus/keyboard/AX/pointer capture/cancellation, style/decorative content
contracts and resource accounting, production-host tests, gallery and installed
consumer, then actual platform/release acceptance. No OS windows, Linux runtime,
CI, publishing or Linear write evidence is claimed by this checkpoint.
