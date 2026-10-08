# Mount a window-local native measurement state machine

[lib.rs](lib.rs) implements the public extension SDK factory for the
[OCaml protocol](../../ocaml/gpuio_performance_probe.md). It is qualification-only
Rust, not a Bonsai component. `sdk::Factory` validates descriptions and mounts a
`Probe`; `sdk::Component` receives commands, renders a 1 × 1 inert element and
retires tasks on unmount. It stores Rust measurement data and event routes,
never live OCaml values. The application owns the window and workload producers.

## Validation and bounded state

`factory` returns an `Arc<dyn sdk::Factory>`. Its descriptor matches schema version
`3`, fingerprint and property/command limits in OCaml. `Command::decode` accepts
exact single-byte phase tags or canonical decimal bucket offsets fitting `u32`,
with metric 0–4. Property validation accepts only `[0]`. `update` checks properties;
it does not reset the measurement on every reactive render.

`Probe` holds shared `Rc<RefCell<State>>`, one optional GPUI task, bounded window
observations and, behind `presentation-diagnostics`, an optional presentation
session. `State` progresses `Idle` → `Settling` → `Running(Snapshot)` →
`Finished(Interval)`, with `Finishing` added for presentation settlement. Illegal
phase requests return `InvalidCommand`; a second `Begin` cannot replace running
work. A fresh begin is allowed after finish. Retaining the task owns its lifetime;
`taking` and dropping it cancels remaining asynchronous work.

## Begin and finish occur at different native boundaries

`Begin` and `BeginIdle` clear observations and spawn a task with a two-second
background timer. It subsequently updates the captured window handle on the
GPUI context, checks the event route, optionally starts presentation collection,
and captures a native CPU snapshot. `Begun` reports snapshot capture cost, not
the settling delay. Emission is asynchronous and does not request a redraw.
Window destruction or an invalid route prevents stale delivery.

Idle mode samples activation and optional OS visibility at absolute one-second
deadlines, capped at 128 observations. Presentation mode also samples during
ordinary phases, capped at 2,048. Sampling reads window state without notifying
or publishing UI changes; the cap stops the loop rather than growing storage.
It establishes sampled observations, not continuous visibility.

`Finish` requires `Running`, cancels the sampling task, and captures the after
snapshot before its command redraw. `Snapshot::since` checks window identity,
clock ordering and counter monotonicity. `distributions` assigns metrics in order:
draw, dirty-to-submission, animation submission interval, input-to-frame, inputs
per frame. `Finished` includes interval duration, capture cost, dropped inputs
and all five counts. These CPU/submission metrics are not GPU completion or
physical presentation.

With presentation enabled, finish first stops admission and captures end visibility,
then enters `Finishing`. A task polls unfinished callbacks every 10 ms for at most
two seconds. After checking the event route, it prints the
[presentation report](presentation.md) to stderr, drops the session, enters
`Finished`, then emits the typed event (and idle observations when requested).
Remaining pending callbacks are reported; settlement timeout never fabricates
success. The collector decides whether the report passes.

The optional presentation implementation also supports an opt-in foreground input
journal summary for diagnosis. It creates an independent cursor at Begin and
drains once at Finish, without additional polling. The input summary makes the
run ineligible for release qualification; see [the diagnostic contract](presentation.md#opt-in-foreground-input-diagnosis).

## Page snapshots and dispose safely

`Buckets` works only in `Finished`. `buckets` emits at most 128 ordered histogram
buckets from the requested offset, includes total bucket count and rejects offsets
past the end or oversized SDK messages. Inclusive upper bounds and counts must
be retained for combining intervals; averaging percentiles loses evidence.
`DocumentPreparation` instead reads cumulative application worker counters and
can be requested independently of phase state.

`unmount` drops the pending task and active session and resets to `Idle`. This
cancels delayed begin/finish/observation work; the event route supplies a second
liveness boundary. Native session callback holders do not own a window/entity.
The probe does not own or cancel application producers merely because its view
unmounts; those belong to the application's Eio/window scopes.

Trace: mount → sequenced Begin → settling → live native snapshot/Begun → workload →
Finish cutoff → optional callback settlement/JSON → Finished → bounded bucket pages →
unmount cancels remaining probe work. The streaming caller performs independent
source, native editor, frame and retirement checks around this trace.

## Verify and adapt the diagnostic boundary

From the root after [setup](../../../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --manifest-path examples/performance_probe/rust/Cargo.toml --locked --lib
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --manifest-path examples/performance_probe/rust/Cargo.toml --locked --lib --features presentation-diagnostics
```

The decoder unit test exercises valid tags, maximum offset and malformed requests.
The optional report unit test is described in [presentation.md](presentation.md).
Neither tests actual desktop timing. See the [package README](../../README.md)
for paired executable/collector commands. No tests/builds/GUI runs were performed
for this guide. Adapt phase protocol and collector assertions together; preserve
bounded observations, delayed readiness and cancellation rather than exposing
measurement data through arbitrary unbounded native callbacks.
