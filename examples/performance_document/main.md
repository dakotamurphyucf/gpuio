# Follow growing documents through publication and interaction

[main.ml](main.ml) drives a native document workload with a cooperating collector.
`App`, `Scope` and `Document` alias `Gpuio_eio` APIs. `B = Bonsai.Cont` exposes
reactive values, `E = Bonsai.Effect` describes UI operations, `V = Gpuio_bonsai.View`
builds descriptions, and `Probe`/`Profile` select statically registered diagnostic
and document-profile packages. This is not a gallery component or a self-test.

`chunk ~document ~index` constructs exactly 128 KiB, including Unicode Markdown
and a distinguishing header. Smoke appends 2, 2 and 1 chunks; full mode appends
64, 64 and 32, retaining three sources of 8, 8 and 4 MiB. `run` initially creates
Vars for no documents, selected index zero, profile enabled and command `(1L, Begin)`.
`B.Expert.Var.value` reads their current reactive values; the `let%arr` in
`component` recomputes its view when those values change. It uses no state machine.

Before registrations exist, the view shows preparation text. Afterwards,
`Gpuio.Document.Config.create` borrows `Document.handle documents.(selected)`,
selects Markdown and a 700-pixel viewport, and requests Markdown selection text.
`V.with_document_profile` optionally adds the Indigo profile with generation `1L`.
Changing the selected Var borrows a different retained document, not a new source.
The window scope owns all three `Document.create` registrations; a view config
borrows a handle and does not transfer that ownership.

`perform` uses a scoped task completion to execute a UI effect and resolve an Eio
promise. `sync` wraps a thunk through this bridge; an effect value alone does not
run it. One application-scoped worker uses the bridge for Vars and window actions,
while document streaming calls publish through the document runtime.

Concrete trace: `Document.append` publishes a larger retained source → worker
waits for `Document.is_published` and checks errors/source length → native document
updates its view → collector observes the first page and sends `ready N` → worker
records publication-to-acknowledgement time → collector navigates/copies and sends
`continue N` → worker proceeds. Separately, a native probe `Data` event returns an
effect that enqueues the typed event; the worker consumes it, sets the next sequenced
command Var through `sync`, and `let%arr` derives the next native extension request.
The queue/ref observations are not Bonsai state.

The worker creates seeded streaming documents, then resets each to empty before
appending its chunks. For full 8-MiB sources, an extra byte must be rejected without
changing retained source. Numbered checkpoints cover initial state, every growth
navigation, copy, reset and removal of the profile. `navigate` requires both
`ready N` and `continue N`; other checkpoints require continuation, with a longer
copy timeout. Running the executable alone cannot satisfy this stdin protocol.
The [collector](../../scripts/measure_document_growth.py) performs native navigation
and clipboard checks; the application does not itself prove those interactions.

The growth interval follows `Begun`, which acknowledges a settled native snapshot.
`Finish` captures its endpoint before its command redraw; histogram retrieval is
outside the interval. `Document_preparation` snapshots are application-wide
cumulative worker elapsed stage times, including discarded work, not CPU time or
per-append parser latency. Peak fields remain absolute maxima. Append publication
latency and publication-to-AX-ready acknowledgement are separate from physical
paint and GPU completion. Source publication can precede richer preparation.

Normal cleanup clears document Vars, closes the window, waits for zero document
registrations/source bytes and queued work, then shuts down. Adapt source fixtures
and collector canonical expectations together; preserve UTF-8 byte limits,
publication/error checks and exact numbered acknowledgements.

From the repository root after [setup](../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release examples/performance_document/main.exe -j 2
python3 scripts/measure_document_growth.py --build-profile release --smoke --output scratch/document-growth-smoke
python3 scripts/measure_document_growth.py --build-profile release --check-budgets --output scratch/document-growth-full-1
```

Use fresh output directories and the macOS interaction environment described in
[README](README.md). Full optional budgets require at least 1000 draw samples,
p95 ≤ 16.7 ms, p99 ≤ 33.4 ms, peak RSS ≤ 1.5 GiB and no dropped input timestamps.
Smoke and source review establish no new qualification; ordinary histograms do not
prove physical FPS. Commands were reviewed, not run for this walkthrough.
