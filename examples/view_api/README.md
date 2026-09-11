# Typed components

Run `./scripts/gpuio exec dune exec examples/view_api/main.exe` from the repository
root. Click the counter and theme buttons, use Tab/Shift-Tab and Enter/Space, or
select/copy the text in the cards. `--self-test` performs 20 acknowledged revisions
and closes automatically; native interaction assertions are a separate Rust test.

`components.ml` contains reusable, ordinary OCaml components. `bonsai_component.ml`
is a compiled Bonsai.Cont example sharing those components and using
`Gpuio_bonsai.View` for direct effects. The executable currently supplies an
explicit Eio bridge runner; the public Bonsai application runner is OCH-9 work.
See [the API contract](../../docs/design/typed-ui.md) for keys, resets, inheritance,
state precedence, limits and the GPUIX style mapping.
