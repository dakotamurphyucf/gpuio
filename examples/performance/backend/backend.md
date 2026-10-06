# Initialize the statically composed performance backend

[backend.ml](backend.ml) is generated OCaml glue, not a Bonsai component. Its only
binding, `initialize : unit -> unit`, calls the exported Rust symbol
`gpuio_gpuio_performance_backend_initialize`. It constructs no model, view or effect.
The implementation satisfies the virtual [backend interface](../../../lib/native/backend.mli).

The [generated Dune library](dune) implements `gpuio.native` and links one native
static archive. [Gpuio_native](../../../lib/native/gpuio_native.ml) lazily calls
`Backend.initialize ()` before creating a transport. The contract requires one
main-thread initialization per process; registration failure prevents startup.
This differs from the ordinary backend's empty initializer: these qualification
executables need a registered performance probe and example document profile.

Follow the call across [registration.rs](registration.rs) using its
[walkthrough](registration.md). The manifest [native.json](../native.json) selects
both packages. [compose_backend.py](../../../scripts/compose_backend.py) generates
this binding, registration, Cargo manifest and Dune rules; relative package paths
are resolved against the manifest. To change composition, edit the manifest and
generate into a fresh scratch destination and review the artifacts together:

```sh
python3 scripts/compose_backend.py examples/performance/native.json scratch/performance-backend-review
```

Choose an unused destination. The generator refuses to overwrite existing files
whose generated content differs, so a changed manifest cannot be regenerated over
the checked-in backend directly. It does not generate `Cargo.lock`: generate or
update that lockfile with the repository toolchain and review dependency changes
alongside `backend.ml`, `registration.rs`, `Cargo.toml` and `dune`.

Generated Cargo and Dune paths are relative to the output directory. Before
relocating reviewed files into `examples/performance/backend`, adjust those paths
for their final location, or retain the old backend separately and regenerate into
a fresh final destination. Carry the reviewed lockfile with the generated files;
verify final paths and the complete diff together. Scratch output alone does not
change the application's selected backend. This is an adaptation workflow, not a
measurement. Build and run the linked application from the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release examples/performance/main.exe
python3 scripts/measure_list_history.py --build-profile release --smoke --output scratch/performance-smoke
```

Use a fresh output directory. Initialization proves neither probe mount nor a
successful measured interval. Those require the [driver](../main.md) to receive
native acknowledgements and the collector to validate its output. The commands
were reviewed, not executed for this walkthrough.
