# Preview native test synchronization

OCH-17/OCH-41, C2/R2, 2026-10-08. Foundation
[37848090787](https://github.com/dakotamurphyucf/gpuio/actions/runs/37848090787)
at `0c8f94d2` fails three macOS checks. Linux and the separate extracted-app
receiver pass. Each unchanged test also passed once locally, so those passes
alone did not establish a fix. The terminal logs identify narrower issues below.

## Screen-point routing

The first New window query hit PID 2700 rather than gallery PID 22245, before
checking semantic identity. The cropped gallery screenshot does not establish
that no other window covered the point. The original foreign process is unknown.

The test now reuses the existing window-lifecycle fixture: place the gallery
within the display, wait for foreground ownership, stable target coordinates and
an unobstructed point. This preparation records bounded ownership transitions
and times out on persistent obstruction. It neither checks nor retries semantic
identity. The subsequent single query must still resolve to the exact requested
control or a real descendant; returning its window/ancestor still fails.

Local macOS 14.5 ARM64: all 21 points resolve exactly and the app exits normally.
Twelve portable checks cover placement, asynchronous readiness, permanent/late
occlusion, background windows, deadlines and identity. The added identity checks
accept an exact control/descendant and reject window ancestors, unrelated
controls and foreign processes after readiness, with exactly one identity query.

## Popup radar input

All queued popup cancellation cases pass in the hosted log. The later radar
pointer-focus assertion fails. Its old readiness condition checked only retained
label eligibility. A hide/return publication can restore that eligibility before
the asynchronous geometry worker prepares the current publication; the native
pointer handler correctly rejects obsolete geometry.

A new synchronous assertion proves label eligibility is true while pointer
readiness is false immediately after that publication. The test waits for both
conditions within its existing five-second deadline before sending input. A
native-test-only accessor uses the actual production input predicate. Popup
cancellation, pointer focus and lease-retirement assertions are unchanged.
The complete local AppKit suite passes. This establishes an insufficient test
barrier, not a separately reproduced application input-loss bug; the candidate
still needs hosted validation.

## Navigation resize

The third hosted resize sees the current incoming page at 400×280 and an outgoing
probe left at 500×320. The test assumed a two-second wall-clock transition would
survive three asynchronous resizes and GPU readbacks. A local diagnostic adding
2100 ms before the third resize reproduces the exact failure: the outgoing page
has correctly become hidden, so its last painted probe no longer updates.

The geometry test now freezes a per-presenter clock, enabled only with
`native-tests`, before selecting the destination. At an explicit one-second
sample it removes old probes and checks freshly painted incoming/outgoing widths
and half-width offsets at each resize, retaining the GPU pixel assertion. It then
advances to two seconds and verifies the outgoing page no longer paints and the
incoming page settles at zero. Normal builds still read `Instant::elapsed`.
The temporary diagnostic sleep is not retained in the test.

The complete local native-image navigation suite and nine deterministic motion
tests pass. These checks establish geometry/lifecycle behavior, not frame-time or
physical presentation performance. Existing P1/P2 measurements need no rerun for
these harness changes.

## Commands and retained evidence

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native \
  --features native-tests --test native_menu_popup_queue
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native \
  --features native-image-tests --test native_navigation
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native \
  --lib navigation_motion
python3 scripts/test_macos_hit_testing.py --output scratch/point-routing
python3 scripts/test_mac_window_readiness.py
GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all -- --check
```

Local native integration executions used the first two commands with `--no-run`,
then ran the resulting executables with a 90-second Python subprocess timeout.
Build logs, actual executable hashes/platforms, passing logs, the delayed failing
diagnostic and reconstructible source patches are in the
[32-file archive](foundation-37848090787-och17/reports.tar.gz) and
[verified manifest](foundation-37848090787-och17/manifest.json). The gallery binary
is unchanged from `0c8f94d2`; its hash is in the point report. The archive also
retains the original hosted failures, terminal job metadata, raw hosted timing
reports and all three complete consumer reports. C2/R2 remain open until the
integrated candidate passes and the preview is published.
