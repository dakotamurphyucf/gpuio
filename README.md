# GPUIO

Native OCaml applications built with Jane Street Bonsai and Zed's GPUI.
OCaml owns application state; Rust owns native rendering, input and UI resources.
The bridge exchanges versioned bin_prot commands and events.

**Status:** milestones 1 and 2 are merged. Milestone 3 adds managed virtual
lists, paging and long-conversation retention. See the current implementation
status and its validation limits below. This is an experimental framework with
no stable API release yet.

The baseline is stock OCaml 5.3.0, Bonsai/Jane Street v0.17, Core, Eio 1.3,
Dune 3.24.2 and Rust 1.97.1. macOS and Linux (Wayland and X11) are v1 targets.

## Start here

- [Current implementation status](docs/status.md)
- [Contributor guide](CONTRIBUTING.md)
- [OCaml engineering standards](docs/design/engineering-standards.md)
- [Managed lists and paging](docs/design/managed-lists.md) and [runnable example](examples/virtual_list/README.md)
- [Typed UI API and style coverage](docs/design/typed-ui.md)
- [Accepted contracts](docs/design/accepted-contracts.md)
- [Architecture](docs/design/architecture.md) and [expanded v1](docs/design/expanded-v1.md)
- [Research and evidence map](docs/README.md)
- [Linear project](https://linear.app/ochat/project/gpuio-8bd4e30f319d)

Read the [development guide](docs/development.md) for platform prerequisites, then:

```sh
./scripts/gpuio bootstrap
./scripts/gpuio build
./scripts/gpuio test
./scripts/gpuio smoke --self-test
./scripts/gpuio smoke --two-windows
```

The wrapper always selects this checkout's isolated opam root and pinned Rust
toolchain. Do not install dependencies into an unrelated active switch. Use
`scratch/` for ignored local experiments and per-agent/per-ticket notes.

Layout: `lib/` contains OCaml libraries, `rust/` native/protocol crates, `test/`
expect tests, `examples/` application examples, `scripts/` development tooling,
and `docs/` versioned design and evidence. Compiled extension packages will use
these ordinary OCaml/Rust boundaries as the SDK develops.

## License

Apache-2.0. See [LICENSE](LICENSE) and [THIRD_PARTY.md](THIRD_PARTY.md).
