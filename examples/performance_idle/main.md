# Measure settled idle after native focus changes

[main.ml](main.ml) measures two exposed-window phases, rather than requesting
frames in a loop. `App`, `Scope` and `Editor = Gpuio_eio.Text_input` are Eio runtime
APIs. `B = Bonsai.Cont` builds the reactive graph, `E = Bonsai.Effect` describes UI
work, `V = Gpuio_bonsai.View` builds views, and `Probe` provides native instrumentation.

`component` creates a window-owned single-line editor with an instruction seed.
`B.return` provides a constant reactive config; `B.Edge.after_display` stores the
current editor controller for worker queries. A command Var initially contains
`(1L, Document_preparation)`, providing a warm readiness response. Its reactive
value and the editor feed `let%arr`, which derives the editor view, focusable settle
button and probe extension. `graph` connects the editor and lifecycle edge. There
is no text state machine here: native editor state belongs to its controller.

The settle button's click effect is `E.Ignore`; focus movement is the operation
being tested. The [collector](../../scripts/measure_idle.py) first focuses the
editor through accessibility, then focuses the button and confirms editor blur.
For the second phase it activates the existing Finder. It sends exactly
`ready focused` or `ready unfocused` through stdin. The worker waits for the matching
window activation, reads `Editor.read_snapshot`, and requires both editor focus
false and `App.Window.focused_input` equal to `Ok None` before measuring.

Concrete trace: native window activation observation → `Window.on_change` returns
an `E.of_thunk` effect → execution updates `previous` and the transition counter →
worker consumes the collector acknowledgement and calls `send Begin_idle` → UI
bridge sets the sequenced command Var → `let%arr` derives the new extension instance
→ native probe settles and emits `Begun`. Probe `Data` likewise returns an effect
that queues a typed event. Merely constructing either thunk does not execute it.
The refs and event queue record observations; they are not reactive models. During
the measured idle interval the application intentionally does not change its view.

`perform` runs a supplied UI effect from a short scoped task completion and resolves
an Eio promise. `sync` uses that adapter for UI thunks. One application-scoped worker
waits two seconds per smoke phase or 60 per full phase after the probe's separate
two-second settle. `Finish` captures counters before its command redraw and emits
bounded `Idle_observations`; bucket requests follow outside the measured interval.
All five histogram counts and the activation transition delta must be zero.

Native visibility observations occur at absolute one-second deadlines, capped at
128 samples. Missing OS visibility is `None`, not an assumed exposed window.
The collector requires visible samples throughout both phases, no gap over 1.5
seconds, and independent frontmost observations with matching coverage. These
sampled AppKit observations do not prove continuous physical exposure. An inactive
window can still be visible, which is essential to the second phase's meaning.

Normal completion closes the window, drops the observed controller, waits for zero
windows and pending/queued work, emits cleanup and shuts down. A native failure
raises rather than being reported as idle. To adapt this workload, add an owned
resource and settle its animations/caret before `Begin_idle`; extend the collector's
readiness checks too. Do not replace exposure evidence with a background flag.

From the repository root after [setup](../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release examples/performance_idle/main.exe -j 2
python3 scripts/measure_idle.py --build-profile release --smoke --output scratch/idle-smoke
python3 scripts/measure_idle.py --build-profile release --output scratch/idle-full-1
```

The collector requires macOS; full qualification requires an optimized build.
Use fresh output directories and read [README](README.md) for repeated-run evidence.
The executable accepts `--smoke`, not `--self-test` or `--background`. Compilation,
source review and smoke do not establish full idle acceptance or physical FPS.
Commands were reviewed, not executed for this guide.
