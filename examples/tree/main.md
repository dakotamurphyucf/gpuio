# How `main.ml` chooses a tree scenario

[README](README.md) · [Source](main.ml) · [Dune](dune)

This entry point parses flags with `Array.exists (Sys.get_argv ())` and delegates
runtime ownership to one of three modules. It creates no Bonsai graph, native
window, or filesystem capability itself.

The first branch is `--lifecycle-self-test`, which runs
[`Lifecycle_demo.run`](lifecycle_demo.md) and takes precedence over other flags.
Otherwise `--outline` selects [`Outline_demo.run`](outline_demo.md), passing
`--self-test` and `--gesture-self-test` independently. Without either selection
flag, [`Filesystem_demo.run`](filesystem_demo.md) receives `--self-test`.
A gesture flag without `--outline` is therefore not a gesture test. Unknown flags
are ignored rather than rejected by a command-line parser.

From the repository root in the [repository environment](../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/tree/main.exe
_build/default/examples/tree/main.exe
_build/default/examples/tree/main.exe --self-test
_build/default/examples/tree/main.exe --outline
_build/default/examples/tree/main.exe --outline --self-test
_build/default/examples/tree/main.exe --lifecycle-self-test
python3 scripts/test_tree_outline.py
```

The filesystem self-test requires this checkout as the current directory: it
expects `test/virtual_list/tree_widget_test.ml`. All executable modes use a real
native graphical session. The outline gesture harness requires macOS and
Accessibility permission for its launching process; it starts the already-built
executable with `--outline --gesture-self-test`, drives drag/menu/confirmation,
and reaps its child. That flag alone waits for harness actions rather than
performing autonomous gestures. Use one diagnostic mode at a time.

[Dune](dune) separates pure `outline_data` into `Tree_example_model`, its expect
tests into `tree_example_tests`, and the UI modules into `main.exe`. The executable
links Core, GPUIO/Bonsai/Eio and enables `ppx_jane` plus `bonsai.ppx_bonsai`.
Run the pure test group with:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune runtest -j2 examples/tree
```

Read [filesystem loading](filesystem.md), [outline policy](outline_data.md),
[outline tests](outline_test.md), and [effect bridge](support.md) for the modules
behind these launches. Their walkthroughs distinguish source-level assertions,
programmatic native commands, and actual input automation. None of these command
listings is a claim that tests ran or that
[Linux desktop qualification](../../docs/platform-release-policy.md) passed.

When adding another scenario, give it a clear exclusive flag and explicit runtime
entry point. Keep file capabilities and application scopes in that runtime layer,
and keep pure policy modules independently testable.
