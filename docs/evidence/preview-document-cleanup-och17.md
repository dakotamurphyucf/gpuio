# Preview integration follow-up

OCH-17/OCH-41, C2/R2. Foundation
[37862430983](https://github.com/dakotamurphyucf/gpuio/actions/runs/37862430983)
is terminal at `83a9f398`: macOS fails only the document-profile cleanup check;
Linux fails only collector Clippy. The fresh macOS extracted-app job passes.
The previous point-routing, popup queued-start and navigation-resize repairs
pass their corresponding hosted steps on this run. The tested merge
`8dfbc7e8` and branch head share tree `2b767da591672fb49438c99d1ec09448d9e83c70`.

## Linux test-helper condition

The required performance-collector step passes its Rust tests but fails Clippy:
`State::pointer_ready` is unused on Linux. Its only caller is the macOS native
popup test. The helper now uses the same
`cfg(all(target_os = "macos", feature = "native-tests"))` condition as its caller.
This changes neither production behavior nor the macOS readiness predicate.

The collector feature combination passes local macOS Clippy after the change:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j2 -p gpuio-native \
  --features presentation-diagnostics,native-image-tests,native-canvas-tests \
  --lib --tests -- -D warnings
```

This is not Linux compilation evidence; the next Linux CI run must establish
that result. Later Linux consumer checks were skipped after the failure.
Informational Wayland smoke also fails a native-controls text assertion (empty
instead of `De`); that remains an OCH-47 finding rather than Linux GUI acceptance.

## Explicit document-resource snapshots

The hosted public document-profile test completes its interaction sequence,
then times out waiting for `Documents: 0`. Immediately before the timeout it
refreshes the runtime snapshot once, sees images/charts/canvases at zero, and
stops refreshing. The final accessibility dump still displays two documents
and zero registered source bytes.

The runtime page deliberately displays an explicit snapshot, not live counters.
The original cleanup helper only waited for images/charts/canvases; subsequent
passive accessibility polling could not update the document snapshot after
release acknowledgments arrived. This is an insufficient test barrier, not
evidence that a document leak has been repaired.

The helper now optionally includes documents and registered source bytes in its
existing 35-second refresh loop. The document-profile test enables that option.
Both counts must reach zero; persistent nonzero or missing values still fail.
The test does not clear registrations or weaken its final zero assertions.
Other callers retain their existing resource scope.

Four portable tests cover staggered document retirement, separately delayed
source bytes, persistent/missing values and the original caller scope. Three
existing focus-acknowledgment tests also pass. A deterministic comparison shows
the old helper accepts two documents after one refresh, while the new helper
waits three refreshes for zero.

```sh
python3 scripts/test_document_profile_cleanup.py
python3 scripts/test_document_profile_focus_wait.py
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
python3 scripts/test_macos_document_profile.py --output scratch/document-profile
```

The rebuilt local gallery passes all 11 native document-profile checks on
macOS 14.5 ARM64, including the exact event sequence and final zero document and
source-byte counts. The test child is reaped and the clipboard is restored.
The gallery executable SHA256 is
`dee1efb802dc387127d84d88802d79ee1de1591404bc956eea257da475d8c028`.
Its source is `83a9f398` plus the platform-condition correction above; the other
changes are Python test/workflow changes. The native invocation was wrapped in
clipboard preservation with a 180-second subprocess timeout.

Python compilation, Actionlint and diff checks pass. Targeted Ruff passes for
the document-profile script and new test. The shared gallery script retains
244 pre-existing diagnostics, with zero additional diagnostics; it is not
claimed to be Ruff-clean.

## Remaining acceptance

The [42-file archive](foundation-37862430983-och17/reports.tar.gz) and
[verified manifest](foundation-37862430983-och17/manifest.json) retain terminal
metadata, original failures, raw timing reports, three complete receiver reports
and local fix evidence, including the reconstructible source changes. Both raw
Metal probes classify as unavailable; local revalidation agrees. This is not
physical presentation qualification. The receiver reports restore the clipboard
and distinguish packaging revision from binary provenance. They do not establish
signed/notarized distribution acceptance.

Linux consumers and the macOS independent Signal Studio consumer were skipped
after earlier failures; the receiver pass does not replace those required checks.
Submit these locally validated fixes together. Required CI must pass on the
repaired candidate before C2/R2 close. These harness changes
do not change the application paths measured by the retained P1/P2 evidence and
do not require another physical performance batch.
