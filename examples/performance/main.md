# Follow the variable-height history measurement

[main.ml](main.ml) is an automated workload, not an interactive gallery page.
`App` and `Scope` alias `Gpuio_eio` runtime APIs; `B = Bonsai.Cont` builds the
reactive graph, `E = Bonsai.Effect` describes UI work, and `V = Gpuio_bonsai.View`
builds view descriptions. `L` is the Bonsai virtual list and `C` the immutable
`List_collection`. `Probe` is qualification-only instrumentation supplied by the
[static backend](backend/backend.md).

`body i` creates exactly 64, 256 or 2048 UTF-8 bytes, cycling by row index. `key i`
converts an integer to a stable GPUIO key. `run` starts with 96 smoke rows or 10,000
full rows, a source Var holding their collection, and command Var `(1L, Begin)`.
These Vars expose reactive values through `B.Expert.Var.value`; they are not
ordinary snapshots. `let%arr` derives a new view from current values when they
change. The `graph` argument attaches list computation and lifecycle edges.

`component` calls `L.component (module Int)` with estimated height 72 pixels,
overscan 200 pixels and at most 32 active rows in a 1168 × 720 viewport. The
`render_row` callback receives reactive row ID/data; its `let%arr` builds padded
text with 14-pixel font and 20-pixel line height. `B.Edge.after_display` records
materialized IDs and the current list output in refs. These observations help the
worker verify traversal; they are not additional reactive models. The outer view
uses `L.Output.view` and normally mounts a one-pixel probe extension.

`perform` bridges the Eio worker to UI effects: a short scoped task's `on_result`
runs the supplied effect and resolves an `Eio.Promise`. Constructing `E.of_thunk`
does not execute its body. `send` increments the command sequence and uses this
bridge to set the command Var. The derived extension instance then sends that
request natively. Do not call this blocking promise adapter from a UI handler.

A concrete loop is: native probe `Data (Begun ...)` → handler returns an effect →
the effect queues the observation → worker consumes it and calls list
`Controller.scroll_to` → its UI effect changes list state → the reactive list
recomputes row views → GPUIO updates the native viewport. The worker waits for both
viewport position and target materialization, then requests an acknowledged frame.
The controller is reached through the observed `L.Output`; a frame callback is
not a physical-presentation acknowledgement.

The history interval starts after initial viewport readiness and the probe's
two-second settle. It traverses forward ranges, walks backward through individual
rows, and asserts all rows materialized with no active-budget exhaustion. At row
zero it appends 40 lines to that row using `C.set`, waits for the visible range to
shrink, and checks the grown row occupies the viewport. `Finish` captures native
counters before its own redraw; bounded bucket retrieval follows outside the
interval. The driver checks ordered bucket pages and that counts match summaries.
A separate settled idle interval lasts two smoke seconds or 60 full seconds and
requires zero draws. `--wall-clock` omits the mounted probe and reports elapsed
traversal time instead; it cannot support histogram or zero-idle claims.

The window owns its list lifetime; one application-scoped worker drives it.
Normal completion closes the window, waits for retirement diagnostics, emits
cleanup and shuts down. `--background` requests no focus, which does not establish
visibility, keyboard validation or foreground acceptance.

From the repository root after [setup](../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release examples/performance/main.exe
python3 scripts/measure_list_history.py --build-profile release --smoke --output scratch/performance-smoke
python3 scripts/measure_list_history.py --build-profile release --check-budgets --output scratch/performance-full-1
```

Fresh output directories are required. The [collector](../../scripts/measure_list_history.py)
checks protocol, traversal, cleanup and optional workload budgets: at least 1000
draw samples, p95 ≤ 16.7 ms, p99 ≤ 33.4 ms, peak RSS ≤ 1 GiB and no dropped input
timestamps. Smoke checks the path, not full budgets. There is no `--self-test` mode.
Review [README](README.md) for repeated qualification and optional presentation
instrumentation; ordinary snapshots do not establish typing, IME or physical FPS.
Commands were reviewed, not executed for this guide.

To adapt it, change fixtures, row counts and collector expectations together.
Preserve stable keys, bounded active rows and explicit readiness checks; define
new measured boundaries before running rather than silently including setup.

The traversal uses immediate `Controller.scroll_to` commands, including whole-row
jumps on every backward step. It does not simulate smooth wheel or trackpad
scrolling; large, rapid visual jumps are expected in this workload. Check ordinary
scroll interactions separately before diagnosing a visual glitch from its motion.

A timed-out readiness or rendered-frame wait writes `GPUIO_LIST_WAIT_FAILED`
with the wait label, window snapshot, list viewport, active/materialized row counts
and runtime diagnostics before failing. Viewport waits retain their 15-second
limit; rendered-frame waits retain 10 seconds and name the target row (or growth
phase). These failure-only observations help distinguish state convergence from
missing frame acknowledgements. A window snapshot is not proof of visibility at
every earlier frame, and the diagnostic does not turn an incomplete run into a
passing measurement.
