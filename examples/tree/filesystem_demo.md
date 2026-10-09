# How `Filesystem_demo` connects lazy loading to a native tree

[README](README.md) · [Implementation](filesystem_demo.ml)
· [Interface](filesystem_demo.mli) · [Filesystem loader](filesystem.md)

`run ~self_test` creates the runtime; `component` constructs the Bonsai graph.
`App.run` owns GPUI on the OS main thread and one OCaml Eio UI domain.
A child application scope named `filesystem tree` owns `Loader.create`, which
captures the explicit `Eio.Stdenv.cwd` capability in `Filesystem.load root`.
This loader is created before graph evaluation. Its work is independent of
transient row lifetimes; view derivation performs no filesystem I/O.

## Reactive preferences and native rows

`B` abbreviates `Bonsai.Cont`, `E` means `Bonsai.Effect`, and `graph` owns reactive
computations. `B.state` creates selection-mode state initially `Multiple`, and
notice text with setter effect constructors. `Loader.value` supplies immutable
reactive snapshots. `Tree.component` combines these with fixed 30-pixel rows,
90-pixel overscan, a 32-row budget, and initial expansion of `Filesystem.root_id`.
The bounded parent gives native layout its viewport; default item content uses
node labels. `Loader.controls` supplies generation-checked loading effects.
See [tree widget](../../lib/bonsai/tree.mli) and
[loader](../../lib/eio/tree_loading.mli).

Opening `B.Let_syntax` enables `let%arr`: dependencies read current reactive
values to build views and callbacks. `and` binds dependencies rather than threads.
`on_action` maps `None` to `E.Ignore`, activation to a notice setter, and moves
to a notice saying application approval is required. No move is applied and no
file is opened by activation.

The widget owns live selection/expansion preferences and native keyboard input;
application mode changes reconcile eligible selection. Its output exposes view,
state, projection, viewport, and active count. The summary gets the loaded tree
through `Rows.source` and counts mounted rows separately from loaded nodes.
An output error becomes readable text here, instead of unconditional unwrapping.
`B.Edge.after_display` records successful output for diagnostics using a thunk.

## Expansion and reload traces

Expand Workspace: native input becomes an ordered tree request; the widget
updates expansion and submits loading demand; the scoped Eio loader reads a
page; completion publishes a snapshot; Bonsai derives rows; native layout mounts
the bounded active set. Selection does not pin every selected offscreen row.
Native focus retention can pin a row within the same active budget.

The mode button runs a setter effect. Reload runs `Loader.reset` inside a thunk
with a fresh `Filesystem.initial ()`. Reset cancels obsolete work and retires
old targets/controllers/transient models. A captured filename alone does not
permit an old reveal effect to act on a replacement generation.

## Commands and diagnostic limits

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/tree/main.exe
_build/default/examples/tree/main.exe
_build/default/examples/tree/main.exe --self-test
```

Run from the repository root using [development setup](../../docs/development.md)
and a graphical session. The window is 760 × 620 logical pixels. The self-test
uses [Support.perform](support.md) to schedule controller effects. It waits for
`test`, expands it and `test/virtual_list`, captures the known test file, reveals
it with focus, and observes its native viewport pin. It checks the 32-row budget,
explicit selection, reset selection repair, and a stale reveal after reset.
Each staged wait has a 20-second bound; the final native frame has a 5-second
bound. Success prints `TREE_WIDGET_PASS` and calls `App.shutdown`.

A controller reveal opens loaded ancestors and waits for their displayed
projection; it never discovers an unknown path. These checks use real directory
I/O and native command/layout observations, but no physical keyboard, pointer,
IME, or assistive-device input. They depend on checkout paths and are not the
large-tree resource acceptance workload. Persist deliberate preferences outside
the widget for later mounts; choose application versus window scope explicitly
when adapting loader ownership, and keep all blocking work outside `let%arr`.
