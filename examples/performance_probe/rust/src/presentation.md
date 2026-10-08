# Export bounded OS presentation evidence after the measurement cutoff

[presentation.rs](presentation.rs) is compiled only with the probe's optional
`presentation-diagnostics` Cargo feature. It serializes native observations;
it does not render a frame, dispatch input, implement Bonsai state or perform
I/O in a Metal callback. [The probe](lib.md) prints its returned JSON after
admission stops and callback settlement ends.

## A session observes resources without owning the renderer

`Active` stores `Result<Session, StartError>` and start capture cost. `Active::start`
uses `Limits::default`: at most 128 unfinished frames and 4,096 raw records.
The [native wrapper](../../../../rust/native/src/performance/presentation.rs)
requires a real macOS AppKit window and rejects TestPlatform/other operating
systems as unsupported. `Session` holds bounded measurement state, not a window,
entity, renderer or drawable. Callback holders use weak state references; they
cannot prolong the session. Session start requests no frame.

`stop` prevents new admission; `pending` reads the unfinished count without copying
histograms. An unsupported start is retained as an explicit error with zero pending
observations, not converted into a successful zero-sample session. The probe stops,
waits at most two seconds and calls `report` even when callbacks remain pending.
The external validator rejects pending or unsupported evidence.

The actual [Metal renderer hook](../../../../vendor/gpui-apple/src/metal_renderer.rs)
attaches before the existing present path. It retains only an owned bounded frame
record in the callback, reads drawable presentation time and a separate host-clock
callback arrival, then completes measurement accounting. No OCaml callback, export,
view invalidation or file I/O occurs there. The
[core collector](../../../../vendor/gpui/src/presentation.rs) keeps clock bounds,
session/frame identity and explicit loss outcomes. Presentation time is an OS
observation; callback arrival, GPU command completion and hardware/photon latency
are different quantities.

## Preserve counters, absence and uncertainty

`report` returns schema `1`, kind `gpui_presentation_interval`, paired CPU elapsed/
input/draw counts, capture/settlement/encoding costs, bounded activation/visibility
observations and end observation. `snapshot` exports session/window identities,
admission/closed/pending flags, all loss counters, cumulative histograms and raw
trace. Histograms describe submission-to-presentation, input-to-presentation and
scheduled animation presentation intervals. Trace truncation is accounted for
separately from lost measurements; histograms remain cumulative.

`record` keeps drawable ID, input count, active/animation/new-scene metadata,
outcome, host timestamps and lower/upper latency bounds. `None` becomes JSON null;
a real drawable ID or timestamp of zero remains zero. Do not substitute callback
arrival for presentation or subtract unrelated clock epochs. `snapshot_encode_ns`
measures the native snapshot conversion portion before the final outer JSON
construction; it is not a complete serialization/output benchmark.

The unit test `records_preserve_absence_zero_and_clock_bounds` constructs synthetic
records, checks a zero drawable identity and latency bounds, then changes the
outcome to `Zero` and verifies absent input bounds remain null. It tests export
fidelity, not native Metal support or presentation performance.

## A report is evidence to validate, not a pass marker

[scripts/presentation_report.py](../../../../scripts/presentation_report.py) pairs
one report with each independently validated CPU phase. It checks schema, session/
window identity, counts, histogram totals, ordered records, loss counters, stopped
admission, settlement and known visibility. It rejects absent/extra reports,
unsupported collection, duplicate sessions and truncated visibility observations.
One-second samples cannot prove continuous visibility.

The permitted startup exception is narrowly defined: an input-free initial prefix
of zero-time outcomes must recover to positive presentation within 100 ms. Zero
frames remain counted and excluded from presented histograms; later or input-bearing
zero outcomes fail. Full non-idle budgets require at least 1,000 presentations,
submission p95 ≤ 33.4 ms and p99 ≤ 50 ms. Streaming additionally requires at least
1,000 paired input frames, input p95 ≤ 75 ms and p99 ≤ 125 ms. Idle must have zero
attempts. These gates supplement existing content/CPU/resource checks; three full
optimized repetitions, overhead and resource qualification remain separate work.

Trace: native frame admission → Metal callback updates bounded state → Finish
stops admission → bounded settlement → snapshot/JSON → collector pairs CPU phase
and rejects missing/lost/invalid evidence. A render callback alone cannot substitute
for this channel, and this guide claims no new platform or release acceptance.

## Commands and adaptation

From the root after [setup](../../../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --manifest-path examples/performance_probe/rust/Cargo.toml --locked --lib --features presentation-diagnostics
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release examples/performance_presented/streaming/main.exe -j 2
python3 scripts/measure_streaming_typing.py --build-profile release --presentation --smoke --executable _build/default/examples/performance_presented/streaming/main.exe --output scratch/presented-typing-smoke
```

No commands were executed for this source walkthrough. Use a fresh output directory,
a visible focused native window and one owned GUI workload at a time. Linux builds
and pure tests establish no Linux desktop presentation acceptance. Read the
[presented workload README](../../../performance_presented/README.md) and
[declared contract](../../../../docs/design/metal-presentation-qualification.md)
before full qualification. For adaptation, preserve explicit unsupported outcomes,
clock uncertainty and all accounting; declare new limits/budgets before measuring.

## Opt-in foreground input diagnosis

Setting `GPUIO_DIAGNOSE_FOREGROUND_INPUT` (to any value) enables a separate cursor
on GPUI's existing bounded foreground journal at each native Begin. Both generic
Begin and BeginIdle are supported: the list workload uses generic Begin for both
history and idle. `Active.stop` stops presentation admission and drains the journal
once before callback settlement. No extra sampling timer or per-frame callback is
installed. Unmount drops the cursor with the active presentation session.

`diagnostic_foreground_inputs` is null by default. When enabled, the JSON report
contains total input dispatches, those marked as invalidating, counts for at most
32 event-kind names, an overflow-kind count and the journal's lost-entry count.
Non-input records are discarded after draining. The summary includes no key
values, text or pointer coordinates. This journal covers the foreground thread,
including other windows on that thread; it does not identify a human or other
producer, and synthetic input can share the same event kind as OS input. Lost
entries prevent concluding that no unobserved input occurred.

This is diagnostic instrumentation, not release qualification. The collector
rejects every non-null diagnostic summary, even one reporting zero inputs/losses.
Remove the environment variable for accepted measurements. Draining/aggregation
has a cost and is included in diagnostic stop-capture timing; do not combine this
run with the ordinary collector-overhead comparison.

For a bounded diagnostic smoke:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release examples/performance_presented/list/main.exe -j2
GPUIO_DIAGNOSE_FOREGROUND_INPUT=1 ./_build/default/examples/performance_presented/list/main.exe --smoke > scratch/input-diagnostic.log 2>&1
```

Retain the raw log, but parse the `GPUIO_PRESENTATION` JSON lines and inspect just
`diagnostic_foreground_inputs`; a single complete presentation trace can be several
megabytes. A diagnostic-enabled run must never be relabeled a performance pass.
The pure summary tests cover kind grouping, invalidation counts, lost entries and
bounded distinct names. Python collector tests check rejection and preserve
ordinary absent/null-field compatibility. Native evidence must be recorded
separately from those unit tests.
