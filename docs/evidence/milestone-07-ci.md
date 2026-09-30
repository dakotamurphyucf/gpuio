# Milestone 07 hosted validation follow-up

[Run 36781947346](https://github.com/dakotamurphyucf/gpuio/actions/runs/36781947346)
tested `afb4bba48892b25527db158d533f92eb1fdcaad6`. Linux completed successfully;
macOS failed seven steps. This run predates the Settings composition/reveal
repairs and subsequent catalog work. It does not validate those changes.

## Display-scale assertions — locally repaired

Two failures reproduce exactly on local macOS 14.5 arm64 when the native test
fixture overrides GPUI's scale to 1. This exercises real native layout and GPU
readback; it does not change or qualify the physical desktop's display mode.

- Passive link animation expected 112.5 logical pixels at 200ms, but native
  layout returned 112. The pinned layout snaps to device pixels, with half ties
  toward zero. The test now compares the analytic tween width after device-pixel
  snapping, retaining its 0.1 logical-pixel tolerance. At 2×, 112.5 remains exact.
- The dashed border test required a pure-black sample in every gap. On the
  rounded panel's one-device-pixel left edge, successive three-pixel samples
  had valleys of 61..119, shoulders of 192..133 and peaks of 255. The one-pixel
  gaps crossed sample centers and remained visibly antialiased. The check now
  accepts a midpoint-or-darker RGB sample only for one-device-pixel strokes;
  thicker gaps still require near-black. Every edge still needs multiple bright
  and gap samples; solid and absent edges retain their original strict checks.

Both failing-before cases and corrected suites ran locally. The 72-case border
matrix and complete composed-link suite pass with `--scale-one` and the normal
Retina scale. No production renderer change was needed for these two failures.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native --features native-image-tests --test native_border_style --test native_link -- --scale-one
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native --features native-image-tests --test native_border_style --test native_link
```

Each invocation used a 180-second process-group watchdog and closed/reaped
normally. Local diagnostics are in the ignored per-agent scratch directory;
the original hosted logs remain attached to the linked run.

## Remaining failures — not resolved

| Step | Observed failure and next investigation |
| --- | --- |
| Public date picker | `Option.value_exn None` during the self-test. Inspect readiness of observed draft state after opening/remounting; a fixed number of rendered frames may not establish observation delivery. Cause remains unverified. |
| Retained table host | First targeted OS Down key left selection at row 1 rather than row 2. Check native activation/readiness and event delivery; preserve actual OS keyboard coverage. |
| Table full-history retention | The 900-second watchdog expired during continuing second-pass progress. Retained row/cell counts stayed at 128/512 in the available log. Investigate paint cost and an appropriate bounded execution budget without reducing the required 100,000 rows × two traversals. Incomplete traversal is not resource acceptance. |
| Public navigation | An exact editor-snapshot comparison failed after routing forward. Inspect focus/revision versus text/selection changes and verify hidden-editor behavior; do not assume a test-only defect. |
| Streaming documents | The last diff header was absent from the interaction map after a large scroll and one draw. Investigate scroll clamping/layout readiness and actual header visibility. |

These five failures, current-head required CI, other catalog families and the
broader OCH-17 requirements remain open. No milestone or release acceptance is
claimed. Full Linux desktop qualification remains OCH-47; a passing Linux job
does not establish graphical acceptance.
