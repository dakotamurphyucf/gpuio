# Generated OCaml initializer for the selected native implementation

[backend.ml](backend.ml) is generated infrastructure: one external `initialize :
unit -> unit` bound to gpuio_gpuio_counter_backend_initialize. It is not Bonsai
state or a native-widget authoring API. Read it with the
[Rust registration guide](registration.md), [Dune](dune) and
[application manifest](../native.json).

After [setup](../../../docs/development.md), from the root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/extension_consumer/main.exe -j 2
./scripts/gpuio exec dune exec examples/extension_consumer/main.exe
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example extension_consumer
```

The last command independently stages/builds public libraries; `--run` adds the
native smoke described by [README](../README.md). No commands were newly executed
here, and build success is distinct from graphical/platform acceptance.

The executable explicitly links `gpuio_counter_backend`, which implements the
`gpuio.native` virtual library in native/no_dynlink mode and attaches one generated
foreign archive. Dune invokes `build_native.py` with Cargo.toml/profile, producing
archive/link flags from locked composed Rust inputs. [Gpuio_native](../../../lib/native/gpuio_native.ml)
wraps `Backend.initialize` in a lazy value, forced before transport creation or
catalog access. [main](../main.md) requests catalog before opening a window, so
validated installation happens once before `component` use. Calling initialize
repeatedly is not an idempotent per-window operation.

The bridge returns unit and retains no OCaml callback/model; native package state
is created later by mounted views. Both counter and example document profile are
statically included by native.json, even though main only renders the counter.
Composition maintains one Cargo graph with the public SDK and pinned GPUI revision;
consumer code uses public packages rather than private Rust host modules.

For changes, edit the trusted manifest and generate into a fresh directory as
[README](../README.md) specifies. The generator refuses to overwrite changed
output. Review binding/symbol, registration, Cargo/Dune and lockfile together;
select the resulting virtual implementation explicitly. This module has no
independent test: catalog validation and isolated consumer linking/native smoke
exercise different portions of its boundary.

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
