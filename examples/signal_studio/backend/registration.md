# Generated static Rust factory installation

[registration.rs](registration.rs) is generated composition code, not a handwritten
counter implementation or a runtime plugin loader. Read it with
[backend.ml's bridge](backend.md), [Cargo.toml](Cargo.toml), [Dune](dune) and
[application manifest](../native.json). After [setup](../../../docs/development.md),
from the root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/signal_studio/main.exe -j 2
./scripts/gpuio exec dune exec examples/signal_studio/main.exe
```

Build verifies linking, not native counter interaction or platform acceptance;
[README](../README.md#local-checks) lists isolated consumer/native checks. This
review does not build, execute, regenerate or modify native sources.

The ocaml_interop::export attribute exposes the exact symbol used by backend.ml.
Its runtime and unit arguments satisfy the interop signature and are unused:
no OCaml value/closure is retained in a native component. The exported function
calls gpuio_native::extensions::install with one component_0::factory and expects
success, treating duplicate/incompatible composition as an invariant failure.
Cargo aliases component_0 to the separately packaged gpuio-example-counter.
Its [factory](../../extension_package/rust/src/lib.rs) returns an Arc<dyn SDK Factory>;
that descriptor validates bounded properties/commands and constructs native
counter state later at mount. Registration itself creates no counter view.

[extensions::install](../../../rust/native/src/extensions.rs) validates an immutable
SDK registry and installs it with an empty document-profile registry through
[registrations](../../../rust/native/src/registrations.rs). OnceLock freezes the
process catalog; a rejected validation does not install a partial catalog, but
a second successful installation is disallowed. Install must precede transport
creation/catalog lookup; lazy OCaml initialization owns the once-only ordering.
There are no document profile factories in this application's manifest.

Cargo builds a staticlib rooted at registration.rs, depends on the locked interop
revision/no-caml-startup and repository native/SDK/GPUI packages. The generated
Dune rule runs build_native.py and accounts for manifest/lock/registration/source
trees in its dependencies. The archive is linked through the gpuio.native
implementation selected by the application; this is static trusted composition,
not loading arbitrary Rust/OCaml code from a notification or document.

Trace: App.extension_catalog → lazy Backend.initialize → exported Rust install →
validated process catalog → OCaml checks Counter.schema → later Ui.extension
instance mounts the registered factory. Application/window disposal retires mounted
instances; the immutable catalog is process-wide and has no per-window unregister
API. Counter native state and callback payloads own Rust lifetimes; they never
store live OCaml values. The separately packaged counter's diagnostic drop logs
are opt-in acceptance instrumentation, not this registration function's contract.

To add another package, change native.json and regenerate using the command in
[backend guide](backend.md); review schema/package compatibility and lock inputs.
Do not call install for every view or manually edit this generated list alone.
The public SDK/consumer gate verifies composition separately from actual runtime
input and native lifecycle evidence.
