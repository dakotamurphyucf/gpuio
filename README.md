# GPUIO

Native OCaml applications built with Jane Street Bonsai and Zed's GPUI.
OCaml owns application state; Rust owns native rendering, input and UI resources.
The planned bridge exchanges versioned bin_prot commands and events.

**Status:** milestone 1 foundation work has started. This is a repository scaffold,
not a released UI library. No production widget API is implemented yet.

The baseline is stock OCaml 5.3.0, Bonsai/Jane Street v0.17, Core, Eio 1.3,
Dune 3.24.2 and Rust 1.97.1. macOS and Linux (Wayland and X11) are v1 targets.

## Start here

- [Current implementation status](docs/status.md)
- [Contributor guide](CONTRIBUTING.md)
- [OCaml engineering standards](docs/design/engineering-standards.md)
- [Accepted contracts](docs/design/accepted-contracts.md)
- [Architecture](docs/design/architecture.md) and [expanded v1](docs/design/expanded-v1.md)
- [Research and evidence map](docs/README.md)
- [Linear project](https://linear.app/ochat/project/gpuio-8bd4e30f319d)

OCH-19 through OCH-22 supply dependency locking, isolated bootstrap, native build
integration and CI. Until that workflow lands, do not run an unqualified
`opam install .` in an existing switch. In a compatible environment the scaffold
checks are `dune build`, `dune runtest` and `cargo test --workspace`.

Layout: `lib/` contains OCaml libraries, `rust/` native/protocol crates, `test/`
expect tests, `examples/` application examples, `scripts/` development tooling,
and `docs/` versioned design and evidence. Compiled extension packages will use
these ordinary OCaml/Rust boundaries as the SDK develops.

## License

Apache-2.0. See [LICENSE](LICENSE) and [THIRD_PARTY.md](THIRD_PARTY.md).
