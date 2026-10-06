# Editable choices

Read the [component walkthrough](main.md) for reactive selection state, native
query ownership, a selection trace and the optional command diagnostic.
Source: [main.ml](main.ml); dependencies and PPX: [dune](dune).

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/combobox/main.exe
_build/default/examples/combobox/main.exe
```

Type a query and choose an option. The selected application ID and native query
are independent. There are no external assets or backend requests.

```sh
_build/default/examples/combobox/main.exe --self-test
```

This opens a local window and checks observation, conditional replacement, stale
revision rejection, undo, unmount and close through native acknowledgements.
It constructs an intent for the command test; the separate Rust `native_controls`
scenario checks actual keyboard selection, filtering, accessibility callbacks
and macOS composition. Physical IME and screen-reader acceptance remain separate.
