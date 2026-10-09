# Retained inspection button accessibility and clipping

OCH-41, 2026-10-06. Local macOS 14.5 arm64 / Apple M1 Max qualification after
`54173d2`. This extends the experimental rich-inspection adapter's evidence,
without changing production rendering, protocol or OCaml APIs. It does not
complete the catalog or establish VoiceOver or Linux desktop acceptance.

## Actual native behavior

The dedicated `native_chart_inspection` executable now exercises real AppKit
`accessibilityPerformPress` against ordinary button semantics in both Card and
Overlay containers:

1. A current semantic object delivers exactly one callback.
2. Another action is queued, with an assertion that it has not yet delivered.
   Removing the target from the source immediately denies its child input gate;
   restoring the same source ID in that UI turn does not revive the queued action.
3. A fresh preview and semantic object recover. Invoking the retained old object
   still produces no callback; another fresh action succeeds.

The Overlay then contains an absolutely positioned nested button. Shrinking the
native viewport clips that button while its outer wrapper remains partly visible.
The button's input gate closes and its real native focus clears, without a tree
or source update. Enlarging the viewport restores eligibility, but the old AppKit
object remains retired. A newly acquired semantic object activates successfully.

These are positive and negative native action tests, not assertions based only on
internal visibility flags. They do not operate VoiceOver or exercise other widget
roles. Command/menu/popup paths, broader combinations of clipping and multi-window
isolation, rich-row helpers and public-gallery qualification remain required.

## Test organization and validation

The existing radar test's AppKit traversal, role lookup and press helpers now live
in shared `chart_native_ax_test.rs`. Radar-specific callback expectations remain
in their original fixture. The new inspection test owns its own callback counter
and source retirement cases; it does not reuse radar IDs or claim radar coverage
as inspection coverage.

The first run passed all new AX/clipping cases, then failed cleanup because the
outer fixture still awaited its pre-exercise source revision. The test now reads
the current source revision after these publication cases. No production adapter
change was required.

Final `native_chart_inspection` passes the new cases and existing button,
Input/Textarea AppKit composition and aggregate-publication cases. Final metrics
are `(43,12,2,0,0)` and chart source bytes return to zero. The scope is native
AppKit/GPUI dispatch, not physical keyboard, IME candidate panels or presentation
latency.

Commands use the repository-isolated environment and two build jobs:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests --test native_chart_inspection --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -p gpuio-native --features native-canvas-tests,native-image-tests --lib --tests --locked -j2 -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests --test native_chart_input --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all --check
```

All four commands pass. The existing foreground chart-input regression qualifies
the factored radar helpers separately, including its AX, clipping, command,
two-window isolation and rating cases; final metrics are `(112,7,2,0,0)`.
The [raw logs and source hashes](chart-inspection-actions-och41-logs.tar.gz) and
[verified manifest](chart-inspection-actions-och41-manifest.json) retain both the
initial fixture failure and corrected run. Test windows closed and all commands
finished. No OCaml/example source changed; the preceding full Dune
check remains evidence for `54173d2`, not a newly executed check of this checkpoint.
Hosted run 37549499328 also covers that earlier commit and does not include these
test additions. OCH-41 and OCH-17 remain open.
