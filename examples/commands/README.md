# Scoped commands

Read the [source walkthrough](main.md) for the model/functions, Bonsai syntax,
native ownership, interaction trace and optional diagnostics.
Source: [main.ml](main.ml); dependencies/PPX: [dune](dune).

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/commands/main.exe
_build/default/examples/commands/main.exe
```

Launch opens a native window. No external service or assets are required.

```sh
_build/default/examples/commands/main.exe --self-test
```

This opens a window and validates programmatic bridge/render/lifetime behavior.
Read the walkthrough for what this test does and does not check.
