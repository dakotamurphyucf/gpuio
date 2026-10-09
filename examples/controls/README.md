# Native controls

Start with the [component walkthrough](main.md) for the preference model, shared
choice IDs, native combobox ownership and a complete selection trace. The
implementation is [main.ml](main.ml); [dune](dune) declares its dependencies.

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/controls/main.exe
_build/default/examples/controls/main.exe
```

The second command opens a native window. There is no self-test flag for this
example. If this is your first Bonsai application, read the smaller
[counter](../getting_started/README.md) first.
