# Managed table behavior — OCH-41

2026-10-03, macOS arm64, milestone worktree based on `83eb87e`.
Public Core/Bonsai tables now expose row-header visibility, Stop/Wrap keyboard
boundaries and per-column-header selection eligibility. The Collections → Result
table gallery exercises all three. See the [contract](../design/table-behavior.md).
The extracted header navigation was also adapted to honor eligibility consistently.
Physical keyboard/VoiceOver, broader catalog and OCH-17 release acceptance remain
open. This change does not update GPUI or toolchain pins. The scoped table adaptation
is documented in `rust/table/UPSTREAM.md`.

## Behavior evidence

- Independent OCaml/Rust fixtures freeze appended Op111 and its explicit clear;
  existing Config/Column fixtures remain unchanged. Rust rejects truncated/trailing
  data, invalid boundary tags, duplicate/oversized IDs and excessive counts.
- Core expect tests check defaults, whitelist/schema validation, no-op updates,
  accepted-versus-prepared callback policy, node retention, schema revision advances
  and reset. Current header restrictions reject column select/context/copy while
  permitting cells and sorting.
- A Bonsai driver regression repairs invalid selection, ignores an old callback,
  checks an existing controller against current policy and preserves mounted cell
  computations. No new scheduler or ownership boundary is introduced.
- Native admission tests cover both operation orders, revision fencing, rejected
  commands, unknown/duplicate IDs, non-table attachment, budget rejection and atomic
  rollback. Reset restores the original retained-byte charge and eligibility.
- A production Host test on GPUI TestPlatform retains the same native entity,
  clears invalid selection, exposes accessible Select only on eligible headers,
  retains independent Sort and dispatches queued accessibility selection actions.
  A measured cell-space assertion confirms hiding row headers releases their layout
  width. Real native key routing stops at row one or wraps to logical row 100,000 even
  when that row is absent from the mounted cell cache.
- A follow-up regression reproduced right-arrow selection of a forbidden header
  in the extracted widget. The adapter now searches eligible headers for left/
  right and Home/End, bounds work by column count and handles no eligible headers.
  Cells continue to navigate independently.

## Commands completed

All commands use the repository environment with two build jobs.

- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j 2 -p gpuio-protocol`: **363 passed**, no failures/skips.
- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j 2 -p gpuio-native --features native-image-tests,native-canvas-tests --lib --test tables`: **807 library tests passed**, two existing private-D-Bus skips; **8 table admission tests passed**.
- `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @runtest examples/gallery/main.exe`: passed.
- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --offline --locked -j 2 -p gpuio-native -p gpuio-table-adapter --all-targets --features gpuio-native/native-image-tests,gpuio-native/native-canvas-tests -- -D warnings`: passed after the header-navigation fix.

The full native suite passed again after the header-navigation fix, including
row-header geometry, endpoint/arrow eligibility and an empty eligible set. Structural catalog audit and `git diff --check`
also pass.

`GPUIO_JOBS=2 ./scripts/gpuio check-fmt` passed after final formatting.

`GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --workspace /private/tmp/gpuio-table-behavior-final-20261003`
passed fresh package staging and independent gallery compilation/linking after the
native header-navigation fix (`run=False`). This relinks the final native adapter
into the public gallery without installing into any unrelated switch.

No OS windows were opened. TestPlatform accessibility checks do not establish
physical AppKit/VoiceOver or desktop acceptance. Required Linux non-GUI checks
remain separate; Linux desktop qualification is deferred to OCH-47.

## Pending physical gallery checks

In Collections → Result table, select a detail cell and toggle Row headers; its
selection should remain while the row-header gutter disappears. With both row
and cell selection enabled, click the selected cell again to select its row.
Select the CATEGORY header, enable Select only entry header and verify selection
clears; cells under CATEGORY remain selectable and ENTRY sorting still works.
Select the first row/cell, focus the table and compare Up with wrapping enabled
and disabled. Repeat after restoring defaults and switching gallery panels.
Check actual keyboard, pointer, VoiceOver and resource cleanup separately from
these in-process regression tests. These physical steps have not run for this
extension and are not counted as acceptance evidence.
