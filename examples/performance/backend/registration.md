# Install the qualification factories before native startup

[registration.rs](registration.rs) is generated static registration, not the probe
implementation. `#[ocaml_interop::export]` exposes the function called by
[backend.ml](backend.ml), described in [its walkthrough](backend.md). The runtime
and unit arguments are unused; this function stores no OCaml values in components.
There is no Bonsai graph, native window or measurement timer here.

`gpuio_native::registrations::install` receives one component factory and one
document-profile factory. The [Cargo manifest](Cargo.toml) names their dependency
aliases `component_0` and `document_profile_0`: respectively the performance probe
and example document profile. The `expect` fails startup if their registration is
incompatible or duplicated. Installation makes factories available; mounting an
extension or attaching a profile happens later through application view descriptions.

The [application manifest](../native.json) is the source of composition. Review
[compose_backend.py](../../../scripts/compose_backend.py), the generated
[Dune rule](dune) and Cargo manifest together when adding a package. The static
archive uses `ocaml-interop` without starting a second OCaml runtime. Keep native
ownership inside the package; do not retain an OCaml callback in generated glue.

The [probe implementation](../../performance_probe/rust/src/lib.rs) is where
`Begin`, `BeginIdle`, `Finish`, document preparation and bucket requests are decoded.
It captures [native snapshots](../../../rust/native/src/performance.rs), settles
for two seconds before emitting `Begun`, and drops its pending task on unmount.
Snapshot subtraction preserves sparse histogram counts and rejects regressed
counters. Histograms measure native draw/submission boundaries, not GPU completion
or physical presentation. Registration itself establishes none of those results.

From the repository root, after [toolchain setup](../../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release examples/performance/main.exe
python3 scripts/measure_list_history.py --build-profile release --smoke --output scratch/performance-smoke
```

Use a fresh measurement output directory. For regeneration, follow the backend
walkthrough: generate into unused scratch, review the generated files and lockfile,
and correct output-relative paths when relocating them. The generator refuses
changed-content overwrites. Add instrumentation
through a package with a bounded protocol and explicit unmount cleanup, then extend
its collector before claiming a new metric. Commands here were reviewed, not run.
