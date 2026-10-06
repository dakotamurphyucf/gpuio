# Coordinate four streams, native typing and external measurement

[main.ml](main.ml) has no separate .mli. This is a diagnostic qualification executable,
not an ordinary chat app. Read `perform`/`component`, `run`/config/output helpers,
work/collector handshake, producers/progress, verification/probe pages and cleanup.
`B = Bonsai.Cont` wires reactive Vars, `E` deferred UI operations, `V` views,
`L` managed list, `Model` pure [stream blocks](stream_model.md), `Scope` Eio ownership,
`Editor` native composer and `Probe` qualification-only instrumentation.

From the root after [setup](../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release examples/performance_streaming/main.exe -j 2
python3 scripts/measure_streaming_typing.py --build-profile release --smoke --output scratch/streaming-typing-smoke
python3 scripts/measure_streaming_typing.py --build-profile release --check-budgets --output scratch/streaming-typing-full-1
```

Use fresh output directories and one owned GUI workload at a time. The collector
requires macOS and an already built executable. The program waits for stdin protocol
messages; launching it alone does not generate typing or pass qualification.
Smoke is 4 seconds/80 updates per stream/40 keys; full is 120 seconds/2400 per stream/
1200 keys. [README](README.md) and [predeclared plan](../../docs/design/performance-qualification.md)
require warmup plus three full optimized runs; smoke cannot pass full budgets.
No command was executed by this source review.

## Reactive history, composer and native probe

`component` observes the model Var and passes `Model.rows` to `L.component` with typed Int
comparison and stable stringified keys. Estimated height `160`, 200-pixel overscan,
`max_active` of `32` and Follow_tail_when_at_end bound mounted rows in a 1168 × 600 viewport.
Logical retained blocks remain O(total data). `render_row` is plain text stream/block
label plus canonical source, not four monolithic Markdown documents. It owns no
producer/async row work, hence ignores row lifetime; new delayed row effects would
need [Managed_rows guards](../../lib/bonsai/managed_rows.mli).

`Editor.create` provides one labelled Multiline Qualification composer with 2–3 rows,
outside transient history rows. `after_display` verifies no list budget exhaustion,
tracks peak active rows and records current editor/list controllers for Eio work.
The final `let%arr` renders history/editor and `Probe.instance` using sequenced commands.
Probe `Data` queues bounded protocol responses; `Failed` raises, while `Mounted`/ordinary
`Command_completed` are not timing completion signals. [Probe interface](../performance_probe/ocaml/gpuio_performance_probe.mli)
requires Begun after its settling delay and Finished before fetching histogram pages.
The selected [Dune backend](dune) enables qualification profiling, not ordinary-backend
overhead equivalence or a hardware-latency guarantee.

## Eio-to-UI bridge and exact handshake

`run` keeps App alive through last-window close, starts one main `Scope.start` task and
opens a focused 1200 × 800 window. `perform` starts an empty scoped task whose UI on_result handles
and maps a supplied effect into an Eio.Promise; the caller awaits it. `sync` wraps a
UI thunk. This adapter belongs in the active Eio task, not view construction or a
UI handler that must process its own queued completion. It has no independent
closed-scope/timeout result; collector/process deadlines bound failures externally.
`wait`/`read_line`/frame use local 30-second timeouts where shown, with 5 ms polling waits.

The GPUIO_STREAM_PERF stdout protocol emits config, awaits observed composer and
Probe.Begun, then ready. Collector replies go. A common monotonic start follows;
four forked Eio producers inside one Switch sleep until absolute index/20 deadlines,
then `sync` `Model.append`/`Var.set` and store elapsed time after UI effect completion.
Late work is recorded/caught up, never silently dropped; these timestamps are not
native frame timestamps. Each second emits counts and last observed composer byte
length; that cached snapshot is not final native proof.

## Independent content and timing verification

After joining all producers, `Model.validate` checks every block, UI-update arrays/
elapsed/peak rows are emitted and collector replies typed. `Editor.read_snapshot`
then obtains exact native text, checked against asdf repeated for expected key count.
The collector independently dispatches real keys and inspects native AX text;
replacement commands/AXValue assignment do not count as typing. It restores owned
input-source selection, and records each dispatch deadline/timestamp.

A frame callback precedes `Probe.Finish`; Finished contains native metric counts/
dropped-input count. The program requests every Buckets page with exact metric/
offset assertions, retaining totals instead of trusting a summary percentile.
These are native submitted-frame/input-dispatch metrics, not GPU completion,
physical presentation or hardware key latency. The collector separately checks
sample counts, exact pacing, CPU/RSS, failures and optional presentation reports.

Measurement interval closes before final source row verification. Each stream's
last key scrolls into view, receives a render callback and emits verify-row; collector
checks canonical AX row content and sends `verified N`. It then closes window, drops
observed controllers/resets model, waits for no windows/pending requests/queued
commands/jobs, emits diagnostics/total bytes and shuts down. External collector
validates the complete ordered record set/resource retirement. No forced GC/cache
purge makes measurements appear smaller.

Trace: go → four absolute-deadline producers publish independent blocks while
collector dispatches 10 Hz keys → native editor owns typing/history → join/source
validation → typed/native snapshot and probe histograms → external last-row checks →
acknowledged teardown. A frame callback alone cannot certify the source/typing/
pacing/budget gates. For adaptation, preserve immutable canonical data, scoped
producer ownership, explicit deadlines and independent evidence channels. Change
workload/budgets before measurement, not after seeing results; keep parser/render
performance of rich documents as a separate qualification claim.
