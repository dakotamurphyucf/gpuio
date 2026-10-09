# Native target animation

Read the [component walkthrough](main.md) for targets, stable wrapper identity,
Bonsai derivation and native completion events. Source: [main.ml](main.ml);
dependencies/PPX: [dune](dune).

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/animation/main.exe
_build/default/examples/animation/main.exe
_build/default/examples/animation/main.exe --self-test
```

Launch opens a window. The optional test checks native endpoints, replacement,
live reduced motion and theme updates, then closes. No external assets are needed.
