# Select the generated presentation-enabled native backend

[backend.ml](backend.ml) declares one external `initialize : unit -> unit`, bound
to `gpuio_gpuio_presentation_backend_initialize`. It is generated build glue,
not a Bonsai component or a native-widget implementation. Read the
[Rust registration](registration.md), [Dune](dune), [Cargo manifest](Cargo.toml)
and [application manifest](../native.json) together.

The four [workload Dune files](../README.md) copy existing list, table, document
and streaming OCaml source and explicitly link `gpuio_presentation_backend`.
This library implements the `gpuio.native` virtual library in native/no-dynlink
mode. Dune calls `scripts/build_native.py` using the selected profile and locked
Cargo graph, then attaches the generated foreign archive and C link flags.
It is a static implementation choice, not a runtime toggle in a view.

[Gpuio_native](../../../lib/native/gpuio_native.ml) wraps `Backend.initialize` in a
lazy value forced before transport creation or catalog access. The generated Rust
entry point installs trusted factories once; the OCaml module stores no callback,
model or window handle. Repeated initialization is an invariant failure, not a
per-window registration action. Mounted probe instances own their later native
measurement state; window disposal is independent of the process catalog.

From the root after [setup](../../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release examples/performance_presented/streaming/main.exe -j 2
python3 scripts/measure_streaming_typing.py --build-profile release --presentation --smoke --executable _build/default/examples/performance_presented/streaming/main.exe --output scratch/presented-typing-smoke
```

The second command runs a macOS collector against an already built executable,
with a fresh directory and visible focused window. These commands were reviewed,
not executed. This glue has no independent behavioral test; linking/catalog
validation and real collector runs exercise different boundaries. Smoke does not
satisfy full performance budgets or overhead/resource acceptance.

Trace: executable selects virtual implementation → lazy initialization installs
catalog → app opens window → probe instance mounts → Begin/Finish collect evidence →
window closes and probe task/session retire. For adaptation, update the trusted
manifest, regenerate fresh, review all generated bindings/registration/Cargo/Dune
and lock inputs, then explicitly link one implementation. Keep production and
CPU-only backends' feature choices separate from this diagnostic backend.

The generated [Cargo manifest](Cargo.toml) also carries a `[patch.crates-io]`
entry for GPUIO's pinned `accesskit_consumer` 0.38.0
[text-scope fork](../../../vendor/accesskit-consumer/GPUIO.md). Each composed
backend is a separate Cargo workspace, so patches in dependency manifests do not
carry into its build. The fork keeps nested editors, Documents and Terminals out
of their parent's text range while preserving the semantic tree. The generated
[Dune rule](dune) tracks `vendor/accesskit-consumer` with `source_tree`, so changes
to those sources trigger rebuilding the native archive. Keep both entries when
adapting this backend. This dependency prepares text-scope isolation; rich
accessibility publication, OS selection actions and VoiceOver acceptance still
require their own implementation and validation.
