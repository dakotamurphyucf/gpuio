# Idle-to-active presentation isolation — OCH-17

Local macOS 14.5 arm64 / Apple M1 Max, source after `2e8883e`.
No VoiceOver, display preference change, renderer patch or concurrent compiler
was used during these native tests. All owned applications are terminal and
reaped. [Provenance](idle-presentation-och17/provenance.json) records native probe
and standalone binary hashes and changed source hashes. Workload reports retain
their existing binary and environment metadata.

## Controlled isolation

The native-only GPUI probe now accepts a bounded optional idle delay before its
90-frame sequences. It changes the test setup only; the default remains zero.
Both cases use the same binary, the same two360×280 windows, and the same collector:

| Setup | Window1 | Window2 | Original strict result |
| --- | --- | --- | --- |
| Two seconds idle before frames | 88 presented,2 initial zero-time outcomes | 87 presented,3 initial zero-time outcomes | Fail |
| No idle delay | 90 presented,0 zero-time outcomes | 90 presented,0 zero-time outcomes | Pass |

Both finish all90 frames/window, have zero missing callbacks, saturation, invalid
clocks, duplicates, pending frames or histogram overflow, then retire both windows.
[Idle report](idle-presentation-och17/native-idle-presentation-001/report.json),
[no-idle control](idle-presentation-och17/native-immediate-presentation-001/report.json).
The first failed outcome is retained unchanged. This reproduces the transition
without OCaml, Eio, the protocol bridge or the application workload.

A standalone AppKit/Metal clear-pass program then renders one frame, waits for
successful presentation/GPU completion, idles for two seconds, and resumes its
120-frame sequence. Its [source](idle-presentation-och17/metal-idle-isolation.swift)
uses neither GPUI nor our presentation collector. It reproduces two zero-time
frames immediately after idle. Both GPU command buffers completed successfully;
their GPU execution durations are about25 and24 µs. Subsequent frames present.
The [report](idle-presentation-och17/standalone-metal-idle-001/report.json) preserves
the original failed strict gate. This demonstrates that the behavior also exists
below GPUI; it does not identify its internal OS/display cause.

Apple documents zero time for an unpresented/dropped drawable, so it must not be
counted as a displayed frame. [Apple timestamp contract](https://developer.apple.com/documentation/metal/mtldrawable/presentedtime).
Successful GPU completion alone is insufficient for presentation acceptance.

## Criteria correction and fresh evidence

The [dated criteria correction](../design/metal-presentation-qualification.md#idle-to-active-correction--2026-10-05)
was written before new workload runs. It allows only a fully recorded, input-free
initial zero-time prefix whose first actual presentation is within100 ms of the
first submission, reusing the existing responsiveness ceiling as a hard transition
bound. All later skips, skipped input-bearing frames, unaccounted tail outcomes,
missing callbacks, overflow and unsettled frames still fail. Presented-only
percentiles and sample floors are unchanged; zero-time frames never enter latency
histograms. The original strict failed artifacts are not reclassified.

Twelve portable tests pass, including the new failure cases: recovery over100 ms,
input-bearing skips, skips after presentation, false latency on skipped frames,
all-zero/no-recovery and skipped outcomes hidden beyond a truncated raw trace.
The native probe builds and its strict Clippy/format checks pass.

Fresh development-build smokes:

| Workload | Native outcomes | Startup recovery upper bound | Scope |
| --- | --- | --- | --- |
| List002 | 229 attempted;226 presented;3 initial zeros | 86.187 ms | Complete96-row traversal, growth/anchor, cleanup; idle0 CPU/native frames |
| Typing003 | 114 attempted;113 presented;1 initial zero | 68.254 ms | All40 OS-delivered keys and paired input frames; four concurrent streams; exact final text/source and cleanup |

[List report](idle-presentation-och17/presentation-list-smoke-002/report.json),
[typing report](idle-presentation-och17/presentation-typing-smoke-003/report.json).
Both pass the revised smoke gate with all observations active/visible, no other
loss/invalid outcome, zero pending callbacks and child exit0. The typing run
restores its original input source. These are not full optimized acceptance runs
or a promise of perfectly gap-free frames under every OS condition.

## Reproduction

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo build -p gpuio-native --locked \
  --features presentation-diagnostics --test native_metal_presentation --message-format=json -j2
# Select the executable from the preceding compiler-artifact record.
python3 scripts/test_metal_presentation.py --binary PATH_TO_BUILT_PROBE \
  --idle-before-frames-ms 2000 --output scratch/native-idle
python3 scripts/test_metal_presentation.py --binary PATH_TO_BUILT_PROBE \
  --output scratch/native-immediate
xcrun swiftc docs/evidence/idle-presentation-och17/metal-idle-isolation.swift \
  -o scratch/metal-idle-isolation
GPUIO_METAL_IDLE_AFTER_FIRST_MS=2000 scratch/metal-idle-isolation > scratch/metal-idle.json
```

The idle native and standalone probes intentionally retain their strict result
and exit1 on skipped frames. The controlling Python process used a27-second
native timeout and an18-second standalone timeout. The workload smoke commands
are those in [the integration evidence](presentation-workloads-och17.md), with
fresh output directories List002 and Typing003. No production code was modified
for these results. Optimized repetitions, overhead/resources, remaining native/API
coverage, current hosted gates and final release distribution remain open.
