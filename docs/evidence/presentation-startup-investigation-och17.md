# Presentation startup investigation — OCH-17

2026-10-05, local macOS 14.5 arm64 / M1 Max. Source baseline `3935560`
contains the same application inputs as the preceding `e2939be` measurements.
VoiceOver was not operated or configured. No renderer behavior or vendor patch
was changed in this investigation. One owned GUI ran at a time, with no concurrent
local compilation during measurements.

## Table and document results

The first optimized table smoke completes all 1,030 rows in both directions,
all 64 columns, selection/reveal and cleanup. It records 420 presentations from
423 attempts, with three initial input-free zeros and no other losses or pending
callbacks. The first positive presentation arrives **152.099 ms** after initial
submission, failing the declared 100 ms startup bound. Its two-second settled
idle has zero CPU draws and native attempts. This failure is retained and does
not justify raising the threshold or repeatedly rerunning for a passing sample.

The document smoke passes, followed by one full optimized run: 20 MiB aggregate
content, complete traversal/copy checks, cleanup and clipboard restoration.
The full run has 4,880 presentations from 4,883 attempts, three initial input-free
zeros, first presentation at 44.447 ms, submission-to-presentation p95/p99
19.218/26.018 ms and peak RSS 252,854,272 bytes. Its 3,053 native input samples
pair with CPU samples. Raw trace truncation is explicit (4,096 retained, 787
truncated); cumulative histograms cover all outcomes. All budget checks pass,
but this alone does not complete the three-repetition requirement.

The smoke preceding the planned second document repetition subsequently fails
at 165.648 ms startup (194 presentations, three initial zeros, no other loss).
The application completes and exits zero, but the driver rejects the startup;
the chained second full run does not start. The report's dirty checkout consists
only of documentation and workflow changes, retained in the archive; application
sources are unchanged. [Second smoke failure](presentation-startup-investigation-och17/document-second-smoke-failure.tar.gz).
The startup issue therefore affects multiple workloads and remains unresolved.

```sh
python3 scripts/measure_table_history.py --build-profile release --presentation \
  --smoke --executable _build/default/examples/performance_presented/table/main.exe \
  --output scratch/agents/root-20261004-resumed/presentation-table-smoke-001
python3 scripts/measure_document_growth.py --build-profile release --presentation \
  --smoke --executable _build/default/examples/performance_presented/document/main.exe \
  --output scratch/agents/root-20261004-resumed/presentation-document-smoke-001
python3 scripts/measure_document_growth.py --build-profile release --presentation \
  --check-budgets --executable _build/default/examples/performance_presented/document/main.exe \
  --output scratch/agents/root-20261004-resumed/presentation-document-full-001
```

Original reports, executable hashes and logs: [table failure](presentation-startup-investigation-och17/table-smoke-failure.tar.gz),
[first document run and smoke](presentation-startup-investigation-och17/document-first-run.tar.gz).

## Locating the startup gap

A scratch variant of the standalone Metal probe records drawable acquisition,
encoding and submission clocks, then compares ordinary presentation with
`presentsWithTransaction` and `waitUntilScheduled`. Both runs render once, idle
for two seconds and finish a total of 120 frames. Both retain two zero-time frames
after idle; recovery takes 47.915/48.659 ms. Acquisition after idle takes about
0.821 ms, later acquisitions under 0.017 ms. These small probes do not reproduce
the table's large gap. Both exit 1 against their unchanged strict zero-frame gate;
they are diagnostic results, not passing qualification or evidence for changing
the renderer's presentation mode.

The next scratch overlay uses GPUI's existing `FrameTimingCollector`, then adds
its existing task trace. These opt-in traces alter collection cost and **are not
performance acceptance runs**. The overlay touches only the qualification probe;
the original tracked source is restored byte-for-byte after each build.

- The first traced table has a gap from 36.688 to 173.756 ms between platform
  submission and the next draw. That next draw takes 13.227 ms, and its platform
  submission takes 0.157 ms. The overall startup still fails at 194.007 ms.
- The task-traced table has a similar gap from 25.038 to 144.033 ms. Its longest
  recorded first-second foreground task poll is 4.520 ms. All 892 recorded thread
  task entries remain retained, so this observation is not an overwritten trace.
- The traced list smoke does not reproduce its earlier full-run failure. It
  recovers at 55.982 ms. This diagnostic result does not replace the original
  failed full list measurement.

These traces narrow the delay to the period between frames, outside the measured
GPUI draw and platform-submission calls. They do not establish whether the
remaining gap comes from OCaml preparation, scheduling or another untraced source.
A separate three-second process sample observes Bonsai/Incremental graph work,
allocation and collection on the OCaml side. Aggregate sampling cannot attribute
the exact gap to one function. Two earlier sampling-launch mistakes are retained:
one reused an output directory before launching an app; the other failed to
identify the child and terminated/reaped it. The corrected direct-child sample
and application both exit zero. None is acceptance evidence.

The [standalone archive](presentation-startup-investigation-och17/standalone-timing.tar.gz)
contains source, compilation output, invocation settings and both raw reports.
The [GPUI trace archive](presentation-startup-investigation-och17/gpui-frame-traces.tar.gz)
contains exact overlays, build logs and native traces; the
[sampling archive](presentation-startup-investigation-och17/ocaml-sampling.tar.gz)
retains the process sample and failed preflights. Diagnostic binaries are built
with the repository's isolated `dune build --profile release` for the relevant
`examples/performance_presented` executables, copied to scratch, and invoked with
`GPUIO_SCRATCH_FRAME_TRACE=1` and the ordinary smoke driver. Normal binaries are
then rebuilt after restoration. Both restored list/table hashes match their
preserved pre-experiment binaries exactly.

## Hosted failure is distinct

[CI run 37312985910](https://github.com/dakotamurphyucf/gpuio/actions/runs/37312985910)
passes the required Linux foundation job. The actual PR merge is
`aaac182179af61ae43a8567c4a4b9e97b38c1d8e`, with tree
`fb58367212a33aa9fb4c21cf2f0804267e0e03db`, identical to branch `2e8883e`.
The macOS foundation fails the new GPUI presentation hook, and the dependent
fresh-package receiver is skipped. Do not transfer the earlier successful
fresh-package qualification to this revision.

The runner identifies its GPU as **Apple Paravirtual device**, macOS 15.7.9.
Both native windows complete all 90 callbacks, remain observed visible, settle
with zero pending work and close, but **all 180 timestamps are zero**. This is
neither local startup-prefix behavior nor evidence of successful presentation.
The standalone calibration was skipped because it followed the failed hook.
Its ability to report positive presentation timestamps on this runner is unknown.
[Raw hosted evidence](presentation-startup-investigation-och17/hosted-metal-failure.tar.gz)
includes the report, logs, device counter calibration and source provenance.

The workflow now runs these two independent probes after the other native and
consumer checks, with `!cancelled()` on each so one failure cannot suppress the
other's evidence. It also records display metadata. **Both checks remain required;
no failure is waived or reclassified as success.** The next hosted run must supply
the missing standalone evidence before deciding how to handle this environment.

[Artifact manifest](presentation-startup-investigation-och17/manifest.json).
Open work: identify/fix the local startup delay, complete repeated presentation
workloads and collector overhead/resources, and resolve the hosted observation
failure. OCH-17 remains In Progress.

Follow-up: [managed lifetime allocation](managed-lifetime-performance-och17.md)
identified a substantial OCaml initialization cost, but its optimization was
subsequently withdrawn after constant-configuration branch reactivation reused
retired lifetime tokens. The [correctness repair](gallery-lifetime-repairs-och41.md)
restores reset-scoped allocation and enables inline tests in the release profile.
The optimization's passing timings do not qualify that corrected implementation.
Current optimized workloads must be rebuilt and measured; the original startup
failures and the declared 100 ms bound remain unchanged.
