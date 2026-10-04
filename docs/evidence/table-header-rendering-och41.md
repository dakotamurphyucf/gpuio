# Retained managed-table headers — OCH-41

Local macOS arm64 implementation checkpoint, 2026-10-03. This extends the
[checked description foundation](table-header-foundation-och41.md) through the
Core/Bonsai bridge and the production Rust Host. It does not complete OCH-41,
checked header/body-row styling, or physical release qualification.

## Implemented behavior

`View.Expert.managed_table` and `Gpuio_bonsai.Table.component/paged` accept
`?headers`, a list of `Table_header` descriptions. Each contains an ordinary
submitted View, explicit stable content key and exact column/group target.
Bonsai header computations belong to the caller, outside transient body cells.

Op113 attaches a bounded optional target to a generated, inert Container with
one content child. Only direct managed-table ownership is valid. These wrappers
are explicitly separate from `Set_list_rows`: header/body keys may coincide,
headers do not consume the body-cell limit, and ordinary list admission remains
strict. Normal node/editor/asset/byte quotas apply. Target identity uses the exact
canonical group member set and level, never its label or display order.

Mapping changes advance the root schema revision to retire queued native table
gestures; native admission independently enforces this. Text changes inside a
header do not churn body rows or advance placement. Duplicate/missing targets,
invalid wrappers, stale mapping revisions and quota overflow reject atomically.
Removal frees the metadata reservation and normal child resources.

The native delegate renders retained children without calling OCaml. A scoped
`TableDelegate::render_group_header` method supplies group level/leaf range and
preserves the default delegate fallback. Native panes, widths, heights, resize,
sorting and selection remain native. Header buttons own their actions and focus
without pinning a body row. Offscreen header content receives a hidden semantic
ancestor and an input gate; culled leaf slots default to inert. Returning to the
pane restores input on the same retained owner.

The public gallery's Collections → Result table → **Rich table headers** toggle
adds a leaf-header Inspect button and a group spanning category/detail columns.
The button updates the notice independently of selection/sort. Turning the toggle
off removes custom slots and restores plain headers. The example supports the
existing resize/order/theme/padding and body virtualization controls.

## Local evidence

- Core expect tests check colliding header/body keys, stable slot IDs, content-only
  changes, schema fencing on move/removal and no body-row metadata churn.
- Bonsai tests check headers with zero body cells, bounded materialization,
  viewport eviction and fresh data lineage without remounting caller computation.
- Both language suites compare independent Op113 set/clear fixtures; Rust's real
  transaction decoder rejects every truncated prefix and trailing bytes.
- Atomic native admission covers ownership, duplicate/unknown targets, dirty
  ancestors, wrapper style/handler rejection, budget rollback and removal.
- The production Host on GPUI TestPlatform lays out leaf/group controls over a
  100,000-row table, clicks the leaf button, checks independent table actions,
  focus retention, target remapping, clipped input and stale accessibility action
  rejection, and releases all retained tree resources on teardown.

Exact broader-suite and installed-consumer results are recorded below. These are local deterministic tests, not actual AppKit keyboard,
IME, VoiceOver or GPU readback. No OS window was opened for this checkpoint.
Physical macOS and required Linux non-GUI qualification remain separate.

2026-10-03 validation at the current uncommitted worktree based on `83eb87e`:

- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j 2 -p gpuio-native --features native-image-tests --lib --test tables`:
  **812 library tests pass**, two existing private-D-Bus tests ignored;
  **12 table-admission tests pass**. Includes the clipped stale AX action case.
- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j 2 -p gpuio-protocol`:
  **369 pass, no skips**.
- `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @runtest @fmt examples/gallery/main.exe`:
  full OCaml tests, OCaml/Dune formatting and public gallery build pass.

The native Host fixture initially attempted duplicate table config operations,
nonconsecutive node allocation and parent removal before descendants. Correcting
those fixture errors exposed the actual clipping case: a wide header slot could
remain partially visible while its narrower button was entirely behind the pane.
The regression failed before measuring the content root separately from its slot
and passed after that change. No rejected fixture batch was weakened into accepted
production behavior. A test-module feature gate was also corrected so the same
native library builds both with integration tests and as a normal dependency.

- Strict lint passes after the conditional cleanup:
  `GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --offline --locked -j 2 -p gpuio-protocol -p gpuio-table-adapter -p gpuio-native --all-targets --features native-image-tests -- -D warnings`.
- A fresh independent public-gallery build passes:
  `GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --workspace /private/tmp/gpuio-rich-table-headers-gallery-20261003`.
  Result: `INDEPENDENT_EXTENSION_CONSUMER_PASS example=gallery run=False`.
  The path is disposable validation evidence, not a build dependency. This check
  stages installed libraries and uses an outside-checkout consumer without
  modifying any opam switch. It does not run physical gallery acceptance.
- Final `GPUIO_JOBS=2 ./scripts/gpuio check-fmt` and `git diff --check` pass.
  Catalog JSON also parses successfully. No hosted CI, Linux execution, actual
  desktop acceptance, commit/merge or Linear completion is claimed here.

## Composite and held-input follow-up

A further Host regression reproduced two cases absent from the initial test:

1. A wide header row remained visible while its left button was fully clipped.
   Gating only the complete header root left that child eligible. Header rendering
   now carries a native-only context through ordinary View recursion and measures
   each interactive descendant before its semantic subtree. The clipped button
   becomes inert/hidden while its visible sibling remains usable; restoring the
   pane retains the same owner. This introduces no wire or OCaml API change.
2. A held pointer moved over a header button could arm the enclosing column drag
   before the button's click stopped propagation. Interactive header children now
   stop bubbling left-button down without preventing their default behavior.
   Enter down/up still activates once, button drag does not reorder, and dragging
   the bare native header still starts the column drag.

The expanded fixture also places rich content at the second of two repeated-label
header levels and distinguishes pinned/unpinned groups. It reads actual semantic
header bounds and converts physical pixels by the window scale for pointer input.
The direct and composite targeted tests pass with production Host rendering on
TestPlatform. This is still distinct from real AppKit keyboard, IME and VoiceOver
validation. Broader follow-up results are recorded below after they finish.

Follow-up full native regression:
`GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j 2 -p gpuio-native --features native-image-tests --lib`
passes **813 tests**, with the same **two existing private-D-Bus skips**. The
broader input cases change no Core/Bonsai or protocol representation. Required
Linux non-GUI checks and real macOS release qualification remain open.

Final follow-up checks:

- Full native suite rerun with the visible sibling's real semantic Click assertion:
  **813 pass, two existing skips**; clipped left-button AX action is rejected,
  visible right-button AX action emits exactly one Press and no table action.
- Strict native all-target lint passes:
  `GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --offline --locked -j 2 -p gpuio-native --all-targets --features native-image-tests -- -D warnings`.
- Fresh installed-gallery build passes:
  `GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --workspace /private/tmp/gpuio-header-controls-gallery-20261003`.
  Result: `INDEPENDENT_EXTENSION_CONSUMER_PASS example=gallery run=False`.
  This disposable path is evidence only. No switch/default changes or OS windows.
- Final `GPUIO_JOBS=2 ./scripts/gpuio check-fmt` and `git diff --check` pass.
  This follow-up does not claim physical macOS, Linux execution, CI/merge, or
  complete renderer-slot/catalog acceptance; checked row presentation remains open.
