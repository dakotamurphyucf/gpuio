# Typed components

Read the [main source walkthrough](main.md) for API calls, runtime ownership,
interaction traces, commands, and diagnostic limits.
Read [ordinary components](components.md) and the [compiled Bonsai sample](bonsai_component.md)
for the shared counter and reactive syntax.

Run `./scripts/gpuio exec dune exec examples/view_api/main.exe` from the repository
root. Click the counter and theme buttons, use Tab/Shift-Tab and Enter/Space, or
select/copy the text in the cards. `--self-test` performs 20 acknowledged revisions
and closes automatically; native interaction assertions are a separate Rust test.

`components.ml` contains reusable, ordinary OCaml components. `bonsai_component.ml`
is a compiled Bonsai.Cont example sharing those components and using
`Gpuio_bonsai.View` for direct effects. The executable currently supplies an
explicit low-level Eio bridge runner. Ordinary applications can use the current
public `Gpuio_eio.App.run` runner; this sample deliberately shows the bridge layer.
See [the API contract](../../docs/design/typed-ui.md) for keys, resets, inheritance,
state precedence, limits and the GPUIX style mapping.
