# Presentation workload integration — OCH-17

Local implementation after `1dd2031`, macOS 14.5 arm64. Four qualification
executables now compose the optional presentation probe while copying the exact
existing OCaml list/table/document/streaming sources. Their development builds and
Dune formatting pass. The maintained GPUI adaptation adds a lightweight pending
count query, avoiding repeated trace/histogram clones while callbacks settle.
All 157 tracked core files reconstruct exactly; patch SHA-256 is
`42acfccfdd227b8727fba735026754ce512d35c0d2ddb39f338907542b13c04c`.

**Presentation performance acceptance is not complete.** The three native smoke
attempts below are preserved as failed runs. Full optimized runs and overhead/
resource checks remain open. The [contract and budgets](../design/metal-presentation-qualification.md#workload-transport-and-predeclared-budgets)
were written before these attempts, and their zero-skipped-frame requirement has
not been relaxed. No VoiceOver operation or configuration occurred.

## Implementation and portable checks

The original CPU v3 protocol and feature-off behavior remain. The opt-in probe
starts a window-scoped native session beside Begin, stops admission before the
Finish CPU cutoff, waits at most two seconds for pending callbacks, exports one
schema-1 JSON report and acknowledges Finished. Export is outside the measured
CPU interval and outside Metal callbacks. Unmount cancels the task/session.

The shared report reader verifies exact CPU-phase elapsed/count correspondence,
unique sessions in the same window, bounded traces and observations, callback
settlement, complete outcome/histogram accounting, ordered host clocks and latency
bounds, sampled activation/visibility, native input pairing and idle silence.
Losses, skipped frames and unknown support remain failures. A truncated raw trace
is permitted only when fully accounted for by cumulative histograms.
The three existing runners (list/table share one) gain explicit `--presentation`;
unrequested native reports cannot silently become CPU-only acceptance evidence.
Document and typing pollers retain the 64 KB ordinary protocol limit but allow
bounded 4 MB presentation records across multiple reads.

Nine presentation parser tests pass, including loss, identity, clock, visibility,
truncation and split-read sensitivity. Existing portable checks pass: list7,
table3, document5, streaming4, composer8. The composer supports explicit package
features and deterministic unions when component/profile factories share a crate;
existing generated manifests and the new backend are checked. Enabled-probe Rust tests2, CPU-only probe test1 and strict enabled-probe Clippy
also pass. Workflow lint, Python syntax, Rust/Dune formatting and diff checks pass. CI now builds all four executables and runs
the probe contract/lint on both platforms; that hosted result is still pending.

## Actual list smoke: failed on skipped startup frames

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build \
  examples/performance_presented/list/main.exe \
  examples/performance_presented/streaming/main.exe
python3 scripts/measure_list_history.py --build-profile dev --presentation --smoke \
  --executable _build/default/examples/performance_presented/list/main.exe \
  --output scratch/agents/root-20261004-resumed/presentation-list-smoke-001
```

[Original report](presentation-workloads-och17/presentation-list-smoke-001/report.json)
and [raw log](presentation-workloads-och17/presentation-list-smoke-001/application.log):
307 admitted frames, 304 presented, three zero-time callbacks for sequences0–2.
No missing callback, saturation, invalid clock, duplicate or histogram overflow;
zero pending after settlement. All visibility/activation samples are true.
The 96-row traversal, growth anchor and cleanup checks pass. The separate idle
interval has zero CPU draws, native attempts or presented frames. Child exits0
and is reaped, but the Python qualification correctly exits1.

Apple documents zero `presentedTime` for unpresented/dropped frames;
[the timestamp contract](https://developer.apple.com/documentation/metal/mtldrawable/presentedtime)
does not allow counting them as presentations. All three are at the start of
this run. The cause remains unverified; do not infer a sustained GPU bottleneck,
compositor cause, or measurement bug from that pattern alone.

## Actual typing: paired samples, failed interval acceptance

The first typing attempt used the existing direct-to-PID key route. It retained
all40 expected characters and40 native paired samples, but sampled activation
became false. Its [failed report](presentation-workloads-och17/presentation-typing-smoke-001/report.json)
remains recorded; this is not foreground acceptance.

Presentation-mode typing now raises the owned window after input-source setup,
focuses its composer and uses the OS keyboard route with a foreground ownership
check before every event. Other CPU-only runs preserve their existing route.

```sh
python3 scripts/measure_streaming_typing.py --build-profile dev --presentation --smoke \
  --executable _build/default/examples/performance_presented/streaming/main.exe \
  --output scratch/agents/root-20261004-resumed/presentation-typing-smoke-002
```

The [second report](presentation-workloads-och17/presentation-typing-smoke-002/report.json)
retains all40 OS-delivered characters, four concurrent streams and cleanup. Every
sampled activation/visibility and the end observation is true. The original input
source is restored. All40 input-bearing frames have valid paired clock bounds and
appear in both CPU and presentation histograms. Their
[diagnostic histogram](presentation-workloads-och17/typing-pairing-diagnostic.json)
has p95 24.085 ms and p99 27.427 ms. This is a small unoptimized sample, not budget
acceptance. Of118 admitted frames,117 present and one startup frame has zero time;
that frame carries no input. There are no other losses or unfinished callbacks.
The child exits0 and is reaped; the qualification exits1 on the zero-time frame.

## Provenance and remaining work

Raw logs/reports retain binary hashes, base revision, dirty state, machine/display/
power metadata and failure tracebacks. The build logs are retained alongside them.
The integration source snapshot hashes describe the checkpoint after these runs:
later harness edits only add split-read handling and clarify the skipped-frame
error text. They do not retroactively turn any run green. No compiler ran during
these native measurements. All three processes are terminal and reaped.

Next: diagnose the reproducible first-frame skips without discarding them, then
qualify all four actual workloads, overhead and retirement on optimized builds.
Keep startup versus steady-state outcomes explicit. The one-Hz window observations
are sampled evidence, not proof of continuous visibility. Required hosted checks,
remaining catalog/API/native behavior, notices and release distribution remain
open. VoiceOver stays on the owner's hold.
