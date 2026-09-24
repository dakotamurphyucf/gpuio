# Editable choices

`./scripts/gpuio build examples/combobox/main.exe` builds the public Bonsai/Eio
example. Run `_build/default/examples/combobox/main.exe` to type a query and choose
an option. The selected application ID and native query are independent.

`--self-test` opens a local window and exercises the public controller through
actual native command acknowledgements: observation, conditional replacement,
stale revision rejection, undo, unmount and close. It constructs an intent value
for the command test; the separate Rust `native_controls` scenario tests actual
keyboard selection dispatch, filtering, accessibility and macOS composition.
