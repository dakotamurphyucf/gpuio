# Document traversal and window resource ownership — OCH-17

Local macOS arm64, worktree based on `83eb87e865c86717a8bc51b9db6fe1f379d909a9`.
This regression runs the production host, document workers and native profile
renderers on GPUI TestPlatform. It opens no OS windows and measures ownership
and admission reservations, not allocator RSS, GPU memory or desktop latency.

## Workload and assertions

One application and its shared document service survive **36 window cycles**.
This exceeds the service's limit of 32 concurrently registered windows, so
retaining closed-window registrations would fail a later cycle. Each window
installs a static document profile with native code actions and highlighting,
publishes two source replacements and closes its GPUI test window.

The first replacement has 96 paragraph/code pairs in a 180-pixel virtual viewport;
the other cycles use two pairs. Every replacement is followed by a one-pair
document. All documents traverse every block forward and backward, for **808 block
visits**. The test checks actual item bounds after each reveal, rather than merely
changing the requested scroll offset.

Assertions require:

- Parser/profile reservation usage remains constant during each full traversal.
- The same short document has identical reservation usage across all cycles.
- Replaced source snapshots lose their final strong owner after installation.
- Closing the window revokes an intentionally retained event sink. The sink keeps
  its reservation until it is dropped, even though it can no longer emit events.
- Presentation, installed profile and released source snapshots lose their owners.
- After each cycle, the shared service has **zero registered windows, live workers,
  queued/delivering completions and reserved bytes**. The test application also has
  no windows remaining.

The targeted run passes, reporting a peak of **288,696 reservation bytes** and zero
final reservations. These are internal conservative admission units, not a
measurement of total application memory. The test adds only a test-feature-gated
resource snapshot; production lifecycle behavior and public APIs are unchanged.

GPUI reclaims dropped entities at an App update boundary. The fixture explicitly
flushes that boundary after dropping its own strong test handles. Checking before
that boundary initially left a valid callback visible; that was a fixture-ordering
error, not evidence of a production leak.

## Reproduction

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native \
  --features native-image-tests,native-canvas-tests --lib --offline --locked -j 2 \
  full_document_traversal_and_repeated_window_close_release_profile_resources \
  -- --nocapture
```

The feature name enables TestPlatform fixtures here; this library test does not
launch the separate physical native-image test executable. Omitting the features
selects zero tests and provides no evidence for this regression.

The full native library suite passes **877 tests**, with two existing private-bus
skips on macOS. Strict all-target native Clippy, repository formatting and
`git diff --check` pass. No hosted Linux or physical desktop execution is claimed
for this change.

## Remaining acceptance

This finite workload strengthens the retained-document lifecycle evidence. It does
not establish bounded total process memory over arbitrary histories, physical
input/presentation budgets, the separate 10k-record/100k-paged-row workloads,
concurrent streaming while typing, idle redraw behavior or clean-machine operation.
The historical chart timing outlier remains unexplained. Those OCH-17 release
requirements and OCH-41 physical gallery qualification remain open.
