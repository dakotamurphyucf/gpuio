# Install the probe factory with presentation diagnostics enabled

[registration.rs](registration.rs) is generated static registration, paired with
[backend.ml](backend.md). Its exported symbol matches the OCaml external exactly.
The unused runtime/unit arguments are not stored; factories and components retain
no live OCaml values. This module authors neither the probe nor a Metal renderer.

The function passes `component_0::factory()` and `document_profile_0::factory()` to
[registrations::install](../../../rust/native/src/registrations.rs). Cargo aliases
resolve to the [probe package](../../performance_probe/README.md) and
[document profile package](../../document_profile_package/README.md).
The manifest selects `presentation-diagnostics` on the probe dependency; that
feature propagates to the native diagnostic collector and enables JSON support.
It does not enable unrelated default features or load a runtime plugin.

`install` validates both registries before atomically committing their immutable
catalogs to `OnceLock`. Duplicate/incompatible installation fails; generated
`expect` makes it a startup invariant failure. Initialization must precede any
catalog query, since a premature query can freeze an empty catalog. The OCaml lazy
initializer supplies that once-only ordering. Factory registration installs
availability; it is not a measurement session or proof of document-profile behavior.

The presentation executable Dune rules copy exact CPU workload sources while
selecting this backend. Native collection remains [optional and platform-qualified](../../performance_probe/rust/src/presentation.md):
unsupported platforms emit explicit unsupported evidence, which a presentation
collector rejects. A successful Linux build is not a native presentation result.

From the root after [setup](../../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release examples/performance_presented/streaming/main.exe -j 2
python3 scripts/measure_streaming_typing.py --build-profile release --presentation --smoke --executable _build/default/examples/performance_presented/streaming/main.exe --output scratch/presented-typing-smoke
python3 scripts/compose_backend.py examples/performance_presented/native.json scratch/presentation-backend
```

The final command generates reviewed build inputs into a fresh directory, not
measurement evidence or a launch prerequisite. The generator refuses changed
existing output; manifest paths are resolved relative to the manifest. The
[owning README](../README.md) documents the committed backend workflow. No command
was executed here. Full qualification needs independent optimized repetitions,
content/CPU/presentation/resource checks and overhead evidence beyond smoke.

Trace: application transport startup forces initializer → exported Rust installs
trusted descriptor sets → reconciler mounts a probe from the catalog → native
Begin/Finish observes that window → unmount drops probe work/session. The process
catalog outlives individual windows and offers no per-window unregister action.
For changes, edit manifest features/factories, regenerate and review the locked
Cargo graph and matching schema contracts; implement component behavior in its
package rather than editing generated registration.
