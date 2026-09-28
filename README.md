# GPUIO

Native OCaml applications built with Jane Street Bonsai and Zed's GPUI.
OCaml owns application state; Rust owns native rendering, input and UI resources.
The bridge exchanges versioned bin_prot commands and events.

**Status:** milestones 1–5 are merged, including streaming documents,
windows/tabs/splits, native extensions, canvas, richer motion, responsive
containers and an [integrated chat showcase](docs/design/agent-chat-m5-showcase.md).
Milestone 6 adds desktop integration, OS notifications, seven native chart families
and the Signal Studio graphics workbench. Local feature acceptance passes;
consolidated validation and hosted gates are in progress. See
[current status](docs/status.md) for validation limits. This is an experimental
framework with no stable API release yet.

The baseline is stock OCaml 5.3.0, Bonsai/Jane Street v0.17, Core, Eio 1.3,
Dune 3.24.2 and Rust 1.97.1. macOS and Linux (Wayland and X11) are v1 targets.

## Start here

- [Current implementation status](docs/status.md)
- [Run the agent workspace](examples/agent_chat/README.md)
- [Write or consume a native component package](docs/design/extensions.md)
- [Explore the native canvas](examples/canvas/README.md)
- [Explore native chart families](examples/charts/README.md)
- [Run the Signal Studio graphics workbench](examples/signal_studio/README.md)
- [Desktop links and file integration](examples/desktop/README.md)
- [OS notifications and action routing](examples/notification/README.md)
- [Navigation and independent content lifetimes](examples/navigation/README.md)
- [Managed trees, lazy loading and approved moves](examples/tree/README.md)
- [Virtual read-only tables](examples/table/README.md)
- [Documents](docs/design/documents.md) and [windows/tabs/splits](docs/design/windows.md)
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
and `docs/` versioned design and evidence. Compiled extension packages use these
ordinary OCaml/Rust boundaries; the [sample consumer](examples/extension_consumer/README.md)
uses the package's public OCaml API and generated native backend.

## License

Apache-2.0. See [LICENSE](LICENSE) and [THIRD_PARTY.md](THIRD_PARTY.md).
