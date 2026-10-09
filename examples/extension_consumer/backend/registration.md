# Install generated `component` and document-profile factories once

[registration.rs](registration.rs) is generated static composition, paired with
[OCaml initializer](backend.md). Read [native.json](../native.json), [Cargo.toml](Cargo.toml)
and [Dune](dune) before adapting it. It is not the counter author's Rust widget,
a dynamic plugin loader or a per-window resource registration.

From the root after [setup](../../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/extension_consumer/main.exe -j 2
./scripts/gpuio exec dune exec examples/extension_consumer/main.exe -- --smoke
```

No build/native run was performed here. [README](../README.md) gives the isolated
consumer workflow and qualified graphical/platform limits. For regeneration use
a new directory, not an edited existing generated output:

```sh
python3 scripts/compose_backend.py examples/extension_consumer/native.json scratch/extension-consumer-backend
```

This writes build inputs and is a reviewed adaptation command, not a launch
prerequisite or an action executed by this guide. Manifest paths resolve relative
to native.json; relocated independent applications must review machine-local paths
and generate/review Cargo.lock using the pinned toolchain.

`ocaml_interop::export` exposes the exact symbol used in backend.ml. Runtime/unit
arguments are unused; components store no live OCaml values. The function calls
`gpuio_native::registrations::install` with `component_0::factory` and
`document_profile_0::factory`. Cargo aliases these to the independently packaged
[counter](../../extension_package/README.md) and
[document profile](../../document_profile_package/README.md). They return public SDK
Arc factories; installation validates descriptors, not a rendered counter/document.

[registrations](../../../rust/native/src/registrations.rs) validates both registries
before committing either to OnceLock. Duplicate/incompatible installation fails;
the generated expect treats it as a startup invariant failure. Lazy OCaml forcing
ensures once-only install before reading catalogs or creating transports. Reading
an empty catalog prematurely can freeze the process catalog, so ordering is a
real contract. The immutable process catalog is independent of instance mount/
window disposal and offers no per-window unregister action.

Trace: main requests `extension_catalog` → lazy initializer enters exported Rust →
both trusted factory sets install atomically → schema validation succeeds → later
`View.extension` mounts a factory-backed native counter → window close disposes
that native instance. The profile is available to document consumers but this
main does not render a profiled document, so its inclusion is not proof of profile
behavior. Counter properties/commands/events and widget state are SDK-owned, not
OCaml pointers retained by the registration bridge.

For another package/profile, update trusted manifest, regenerate fresh, review
schema/SDK compatibility and committed lock inputs, then link one native implementation.
Native factory authoring/lifecycle tests belong to its package; consumer build
coverage and command/frame smoke remain narrower than physical input/pixel validation.
