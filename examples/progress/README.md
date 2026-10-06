# Native progress

Read the [component walkthrough](main.md) for the four-stage model, Bonsai syntax,
native presentation and the separate render-acknowledgement diagnostic.
Source: [main.ml](main.ml); dependencies and PPX: [dune](dune).

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/progress/main.exe
_build/default/examples/progress/main.exe
```

Next stage cycles empty → indeterminate → halfway → complete. No download occurs.
The launch opens a native window; no external assets are required.

```sh
_build/default/examples/progress/main.exe --self-test
```

The optional diagnostic opens a window, checks native render acknowledgements
for each state and removal, and closes automatically. It does not measure
physical presentation or prove platform desktop acceptance.
