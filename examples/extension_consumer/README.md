# Consuming a native component from OCaml

`main.ml` is a Bonsai application using `Gpuio_example_counter`. `native.json`
selects the Rust package and factory. The generated `backend/` links that package
and GPUIO through a single Cargo graph and supplies the `gpuio.native` Dune
implementation. The consumer does not author Rust or access private host modules.

Run from the repository root:

```sh
./scripts/gpuio build examples/extension_consumer/main.exe
_build/default/examples/extension_consumer/main.exe
```

`--smoke` activates its window and closes after command acknowledgment
and a correlated paint observation. Interactive mode supports native pointer,
keyboard and accessibility activation and an OCaml button issuing a new command.
The application checks the linked schema catalog before starting the runtime.

For a new application, create a manifest like `native.json`, with paths relative
to that file. Each factory is a public Rust function returning `Arc<dyn Factory>`.
Generate into a new directory:

```sh
python3 /path/to/gpuio/scripts/compose_backend.py native.json backend
cargo generate-lockfile --manifest-path backend/Cargo.toml
```

Review and commit the generated files and lockfile. Add the generated library
name to the executable's Dune `libraries`. The generator refuses to overwrite
changed files; regenerate into a fresh directory when changing the manifest and
review the replacement. Use the same pinned toolchain and native system packages
as GPUIO. Copy its `rust-toolchain.toml` into the application root so Cargo invoked
by Dune outside the GPUIO checkout does not fall back to a global Rust default.
Native packages outside the Dune workspace use absolute paths; Cargo
checks them incrementally on each build. Regenerate those machine-local paths
when relocating an application checkout.

The application must also have the pinned GPUIO and Jane Street native packages
available to Dune. A reproducible independent-consumer check stages installed
public libraries under an isolated prefix, including the vendored Bonsai family,
and copies the component and Rust toolchain pin into a separate Dune project:

```sh
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --run
```

This leaves its temporary workspace path in the result for inspection. Omit
`--run` to validate the build on a machine without a GUI. The check never installs
into an opam switch. Its successful GUI run is distinct from Linux GUI release
acceptance. See [the SDK contract](../../docs/design/extensions.md) for lifecycle,
limits, schema compatibility and the trusted native-code boundary.

The smoke application activates its window by default because it must observe
an actual rendered frame before closing. An occluded background window may not
receive that frame on macOS. Pass `--background` directly to the example only
where background frame delivery is available; this does not waive the paint
acknowledgement or extend the independent runner's timeout.
