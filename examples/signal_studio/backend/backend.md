# Generated OCaml bridge to the composed native backend

[backend.ml](backend.ml) is a generated one-line external binding, paired with
[registration walkthrough](registration.md). It is packaging/runtime infrastructure
for maintainers, not a Bonsai component or mutable domain model. Its function
initialize has OCaml type unit → unit and links symbol
gpuio_gpuio_signal_backend_initialize from the statically composed Rust archive.
It has no receiver, per-window state or cleanup action of its own.

From the root after [setup](../../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/signal_studio/main.exe -j 2
./scripts/gpuio exec dune exec examples/signal_studio/main.exe
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example signal_studio
```

The consumer command stages public libraries and builds an isolated composed
consumer; add --run for the supported native walkthrough described in
[README](../README.md#local-checks). Build-only Linux coverage is separate from
GUI acceptance. No build/test was newly run for this guide.

[backend/dune](dune) implements the gpuio.native virtual library, uses native-only/
no_dynlink mode and links generated foreign archive/link flags.
The [executable stanza](../dune) explicitly selects gpuio_signal_backend alongside
the separately packaged counter OCaml library. [Gpuio_native](../../../lib/native/gpuio_native.ml)
wraps Backend.initialize in a lazy value: catalog access or transport creation
forces it once. [Application](../application.md) requests extension_catalog before
run_desktop and asserts Counter.schema membership, so registration precedes
native component use. Do not directly repeat initialize; Rust registration is
process-wide and rejects another install.

The manifest [native.json](../native.json) lists this backend library, GPUIO root
and one trusted static counter factory. [compose_backend.py](../../../scripts/compose_backend.py)
generates the binding and related files from paths relative to that manifest:

```sh
python3 scripts/compose_backend.py examples/signal_studio/native.json examples/signal_studio/backend
```

That command rewrites generated build inputs; it is an adaptation workflow, not
a launch prerequisite or a command executed by this review. Review generated
Cargo.toml/registration/Dune and the committed Cargo.lock together. For another
component, update the trusted manifest/package and regenerate instead of hand
adding an external symbol. Property/event schema validation and native widget
lifetimes belong to the SDK/component and application views, not this bridge.
