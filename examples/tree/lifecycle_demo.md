# How `Lifecycle_demo` exercises tree ownership and cancellation

[README](README.md) · [Implementation](lifecycle_demo.ml)
· [Interface](lifecycle_demo.mli) · [Effect bridge](support.md)

`run ()` is a scripted graphical diagnostic, not an ordinary interactive explorer.
It uses in-memory unit payloads and controllable Eio producers, with no filesystem
I/O. `initial` constructs a chain of `Tree.max_depth` nodes plus an unloaded
Lazy work branch. Roots have depth 1; the deepest node exercises depth 128.
Validated `Tree.create` admits this forest.

## Scope and reactive graph ownership

`App.run ~exit_on_last_window:false` owns GPUI on the OS thread and one Eio UI
domain. A 700 × 420 window factory creates `Loader` in `App.Window.scope window`
**before** graph construction. The loader survives transient row eviction but
closes with its window. An app-scoped diagnostic task can observe cleanup after
the window itself closes.

`Producer.t` stores behavior plus started/active/finished/peak counters. `Wait`
yields in `Eio.Fiber.await_cancel`; `Fail` returns a deliberate retryable error;
`Complete` returns one loaded leaf. `Exn.protect` decrements active/increments
finished even during cancellation. These counters are test observations, not a
reactive application model. The [loader contract](../../lib/eio/tree_loading.mli)
bounds framework worker concurrency; this scenario checks peak 1 for its workload.

`W.component` reads `Loader.value`, fixed 28-pixel rows, zero overscan, an eight-row
budget, and loading controls. Each custom item graph registers
`B.Edge.lifecycle` activation/deactivation effects, then uses `B.map` to render
its label. Native rows own focus/disclosure/accessibility; the custom content
adds no separate tree reducer. The unused item lifetime is safe here because
rows start no delayed work.

Opening `B.Let_syntax` enables `let%arr` to derive the root from reactive output.
`B.Edge.after_display` stores successful output through an effect. Neither
view derivation nor lifecycle registration performs blocking I/O. See
[tree API](../../lib/bonsai/tree.mli).

## Scripted event/effect/state/view transitions

The task captures current targets via `Output.target` and schedules controller
effects through `Support.perform`. Deep reveal opens loaded ancestors, derives
a new projection, and requests native focus when the row mounts. The script
waits for a native viewport focus pin and verifies the eight-row bound.
It requests content resize to 520 × 320 and waits for observed dimensions;
a command reply/frame alone does not establish compositor size completion.

Expanding Lazy work changes widget expansion, starts loader demand, and produces
one waiting Eio job. Collapsing cancels it; the test checks no leaf was inserted
and the boundary returns Ready. Reopening starts another job. The script selects
the branch, captures a reveal, removes it with `Tree.replace` + `Loader.update`,
and executes the old reveal. It checks cancellation and selection repair rather
than redirecting old work to a reused ID.

Resetting to fresh initial data retires old targets. A new expansion with behavior
Fail publishes `Failed`; two frames must not auto-retry it. The script changes
behavior to Complete and calls explicit `Controller.retry`, then reveals/focuses
the loaded child. Finally another reset starts a waiting producer and force-closes
the window. It checks producer exit, equal mounted/unmounted counts, inactive
window scope, and zero queued/running loader counts before application shutdown.

## Commands, bounds, and adaptation

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/tree/main.exe
_build/default/examples/tree/main.exe --lifecycle-self-test
```

Run from the repository root in the [repository environment](../../docs/development.md)
with a native graphical session. Each labeled poll has a 15-second timeout;
frame waits have a 5-second timeout. Success prints `TREE_LIFECYCLE_PASS` and
shuts down. This mode uses programmatic controller commands and observed native
layout/focus pins, not physical keyboard/drag, IME, assistive-device input, or
full 100,000-row resource acceptance. A pin is native retention evidence, not
proof of physical user input or screen presentation.

When adapting, put jobs in application/window scopes rather than transient item
graphs, let cancellation propagate, and validate targets before delayed work.
Treat failed boundaries as explicit retry state. Use `request_close` for ordinary
user decisions; this diagnostic uses force-close deliberately to verify cleanup.
Large-tree and platform evidence remain separate in the
[evidence ledger](../../docs/evidence/managed-trees-och38.md).
