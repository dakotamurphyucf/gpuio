# Loaded-list scrolling and timeout diagnostics — OCH-17

Local macOS arm64, 2026-10-07, based on `4bae7e1690ec6f33932ffb5cccd86d6b072f186a`.
This is diagnostic evidence, **not completed full-list performance acceptance**.
The owner observed jarring motion and possible row overlap during the automated
history workload. No rendering fix is claimed by this checkpoint.

The subsequent [per-paint geometry diagnostic](list-paint-geometry-och17.md)
checks row and content bounds on the original bridge workload. Its full attempt
is also incomplete; it does not close the visual-jitter report.

## Ordinary scrolling versus benchmark traversal

The benchmark calls `Controller.scroll_to` repeatedly: forward by visible ranges,
then backward through individual row anchors. It deliberately makes immediate,
large jumps. That motion is not a smooth wheel/trackpad gesture.

A temporary diagnostic kept the identical 10,000-row fixture interactive for
25 seconds, disabling automatic traversal during that interval. The driver sent
80 actual pixel-wheel events, 40 down and 40 up, with 24-pixel deltas. Each event
checked that the pointer target belonged to the owned application. The fixture
closed normally and the collector reaped its child.

All 80 sampled accessibility trees had ordered, nonoverlapping row text bounds.
Across adjacent snapshots, all **524 comparisons** of retained rows had exactly
the expected 24-pixel displacement (0.5-pixel tolerance). Nine owned-window PNGs
were retained; visual inspection of the mid-scroll frame showed separated rows
and clipping at the list edge. There is no per-frame recording here, so these
samples cannot exclude a brief paint glitch or establish trackpad momentum
behavior. The `frontmost` field in this wheel helper used a string accessor for a
Boolean attribute and is null; it is not activity evidence. Input ownership was
checked independently through system-wide AX hit testing.

Earlier diagnostic captures slowed the automatic traversal by 200ms per move;
five usable AX snapshots had separated row bounds. Selected fast-traversal
screenshots also showed distinct ordered rows. These observations narrow the
investigation but do not prove that the owner's observed transient never occurred.
The temporary delay and interactive hold have both been removed.

## Failed full runs and visibility control

The fixed batch planned one warmup and three full runs under unchanged budgets.
The warmup passed. The first full run failed at **row 2968 viewport** after
220.970 seconds; the batch stopped and did not execute its remaining two trials.
A subsequent diagnostic full run failed after 153.094 seconds with an unlabelled
`Eio.Time.Timeout` from the rendered-frame wait. Screenshots were taken during
that run, so it is not unperturbed performance evidence. The owner confirmed
switching away from or covering the window during that latest run. There is no
failure-time visibility observation for the earlier viewport timeout, and it
must not be assigned the same cause without evidence.

The workload now writes `GPUIO_LIST_WAIT_FAILED` before failing either a readiness
or rendered-frame wait. It records the labelled target, window snapshot, viewport,
active/materialized row counts and runtime diagnostics. The 15-second readiness
and 10-second frame limits are unchanged; no failure becomes a passing result.

A controlled test set **AXMinimized=true** on the owned window after three seconds
and verified the resulting Boolean attribute. It reproduced **row 110 rendered
frame** timeout: row 110 was anchored in the viewport, the window snapshot was
inactive, the native command queue was empty and one request was pending. The
collector reaped the expected exit-2 child. This verifies the diagnostic path and
supports visibility-dependent frame delivery; it does not establish the cause
of every earlier stall. An initial control attempt read the AX attribute before
the minimize animation completed and aborted; that failed helper attempt is also
retained, with its child reaped. The corrected helper waits up to two seconds for
the actual minimized state.

## Startup investigation remains open

The previous full-list evidence retains a 102.723ms first-positive presentation
against the unchanged 100ms limit. Its submission timeline contains an approximately
60.9ms gap outside measured GPUI draw work. Three predetermined short diagnostic
trials temporarily instrumented draw entry, drawable acquisition, encoding and
submission. First-positive samples were 31.397, 26.818 and 43.253ms; the earlier
long gap did not recur. This does **not** identify or fix the historical cause.
Renderer instrumentation was restored byte-for-byte before later builds. Neither
these smoke trials nor the ordinary-wheel check satisfy repeated full qualification.

## Validation and retained evidence

Commands, through the repository's isolated environment:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec ocamlformat --inplace examples/performance/main.ml
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release -j2 examples/performance_presented/list/main.exe
python3 scripts/test_measure_list_history.py
python3 scripts/audit_example_docs.py
```

The optimized build passed; all seven collector tests passed. The example audit
reported 429 sources in 266 reviewed groups with no pending groups; that is
structural coverage, not a new whole-document review. Normal-path smoke and the
controlled failure are recorded in the accompanying reports. The failure-only
build SHA-256 is `555f07ee848d0548effe3a2f213dd9cf429c43194bc78882d9f94cf63ada2a98`.
The normal-path smoke uses 96 rows and cannot establish full budgets.

[Reports and diagnostic sources](list-scroll-diagnostics-och17/reports.tar.gz)
include all trials mentioned above, exact temporary source variants, build logs,
collector reports, screenshots and the controlled-minimize helper.
[Manifest](list-scroll-diagnostics-och17/manifest.json) records SHA-256 hashes for
every archive member. Temporary diagnostic scripts are evidence, not production
build dependencies or public API.

OCH-17 remains In Progress. Remaining work includes repeated unperturbed full
workloads with suitable visibility, the unresolved historical startup failure,
collector overhead and the other performance/resource/release requirements.
