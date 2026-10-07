# Hosted popup focus and navigation resize investigation — OCH-17

While [run 37579057201](https://github.com/dakotamurphyucf/gpuio/actions/runs/37579057201)
at `73b4e713` is still running, Linux foundation has passed and macOS has reported
two required native-test failures. This checkpoint retains the relevant live-log
excerpts; it is not a terminal run report or a repair claim.

- `native_menu_popup_queue` passes its initial queued-lease cases, then fails
  `menu_popup_chart_test.rs`'s chart-focus assertion after an injected click at
  `(200, 80)`, before opening the original-data browser. The cause is not yet
  established.
- `native_navigation` with `native-image-tests` passes its earlier semantics,
  disclosure, disabled/inert, stack and nested-lifecycle cases. In the resize
  workload, the child width remains 500 pixels when 400 is expected, despite a
  preceding viewport wait and two-frame barrier. The existing log cannot tell
  whether the viewport subsequently changed or child layout remained stale.

On local macOS 14.5 arm64 / M1 Max at `b514c301`, the unchanged popup test passes.
Navigation passes with both `native-tests` and the actual `native-image-tests`
feature used by CI, including its resize pixel checks. These local passes do not
explain or qualify the hosted failures. The local revision includes later
rendered-selection work; it is not a clean rerun of the older hosted revision.

Diagnostic-only edits retain the assertions and timeouts. The popup failure now
reports active/hovered state, pointer and focused handle, viewport, and chart/menu
bounds. The resize failure reports its current viewport and incoming/outgoing page
bounds. Strict all-target native Clippy with all native test features passes after
these edits. There is no retry, skipped assertion or runtime behavior change.

[Retained reports](hosted-input-geometry-och17/reports.tar.gz) and the
[verified manifest](hosted-input-geometry-och17/manifest.json) contain the hosted
excerpts, three local native logs, exact commands, diagnostic patch and lint log.
The local test driver restores captured clipboard representations after child
exit. All local test children exited; no VoiceOver or OS settings changed.
The hosted run is preserved until terminal rather than cancelled by a new push.
