# Standalone list identity and selection model — OCH-41

2026-10-03, macOS arm64, milestone worktree based on `83eb87e`.
This implements the pure foundation of the
[standalone list contract](../design/selectable-lists.md). It does **not** complete
the persistent list component: native input/ListBox semantics, Bonsai/Eio query
integration, public gallery/consumer and physical/platform acceptance remain.
No OS windows were opened and no native or wire code changed in this chunk.

`List_collection` now supplies opaque source/membership references and a cached,
metadata-only Identity snapshot. Point value updates share Identity and retain
membership; reorder and atomic replacement preserve surviving references.
Separate deletion/reinsertion and independent sources retire them, including when
intermediate snapshots never render. `Expert.item_key` supplies an incarnation
identity for managed-row adapters. It is not a persistence key or native handle.
The value wrapper remains the explicit point-invalidation signal, so payload
comparison is neither required nor introduced.

The identity index adds O(n) OCaml metadata alongside loaded data; it is separate
from bounded native/Bonsai row resources. References and Identity retain no
application payloads. Full release memory/performance qualification must include
this metadata; these checks do not establish an RSS bound.

`List_selection.Catalog` validates loaded/visible/disabled membership and indexes
eligible positions. The model separates logical cursor, committed selection,
range anchor and context target. Single/multiple selection, stop/wrap navigation,
replace/toggle/extended ranges, idempotent AX selection intents, primary/secondary
confirmation and cancellation have explicit contracts. Hidden committed choices
survive filtering; stale references and unrelated sources cannot inherit them.
Cursor repair does not silently commit a new selection. Relative navigation on
an unchanged catalog uses ordered-map lookup and avoids full selection scans.

## Evidence and limits

Expect tests cover source/item identity through streaming, reorder, atomic
replacement, deletion/reuse and foreign sources; independent incarnation view keys;
cursor/selection/context separation; filtering, empty/all-disabled catalogs,
reconciliation, confirmation/cancel, mode changes and hidden/disabled AX setters.
All 256 eligibility masks for eight entries agree with simple forward/reverse
navigation, stop/wrap and range order. A 100k loaded/selected catalog supports
1,000 cursor moves while retaining committed selection and sharing identity across
a streamed payload update. This validates behavior at scale, not timing or RSS.

Commands run through `GPUIO_JOBS=2 ./scripts/gpuio exec`:

- `dune build @runtest examples/gallery/main.exe` passes with the production
  changes and initial exhaustive tests.
- `dune runtest test/virtual_list` passes with the final assertions, including
  reverse wrapping, source-order mode fallback and hidden/disabled AX setters.
- `GPUIO_JOBS=2 ./scripts/gpuio check-fmt` (direct) passes on the final sources.
- `git diff --check` passes.

No snapshots were automatically promoted. Existing duplicate system-library link
warnings remain. Required native/paired-wire input checks and installed-consumer
validation will follow the component implementation. Linux and physical macOS
coverage are unchanged; OCH-41 and the full milestone goal remain open.
