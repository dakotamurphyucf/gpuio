# How generated `backend.ml` selects static initialization

[README](../README.md) · [Source](backend.ml) · [Rust registration](registration.md)
· [Backend Dune](dune)

This generated one-line module binds `initialize : unit -> unit` to
`gpuio_gpuio_resource_audit_backend_initialize`. It is backend-author glue, not a
consumer UI component or measurement collector. There is no Bonsai graph,
`let%arr`, effect, model, or native resource handle in this module.

The Dune library `gpuio_resource_audit_backend` implements the virtual
`gpuio.native` library, links its static archive, and disables dynamic linking.
The public native wrapper's lazy initialization calls the selected
`Backend.initialize` before creating its native transport. That chooses the
matching Rust catalog once; application code normally calls App, not this
external function directly. See [wrapper](../../../lib/native/gpuio_native.ml).

[Native manifest](../native.json) includes the performance probe, resource audit,
and document profile. `scripts/compose_backend.py` produces this module, Rust
registration, Cargo manifest, and Dune glue from trusted static package paths.
The generated backend Cargo.lock and linked SDK/GPUI closure must be reviewed
alongside the manifest. Regeneration is an author action, not startup behavior;
this walkthrough does not run it or change generated code.

To regenerate intentionally from the repository root, the generator's syntax is
`python3 scripts/compose_backend.py examples/resource_audit/native.json <output-dir>`.
Paths inside JSON resolve relative to the manifest. Review output before selecting
it, and retain the audit-only leak-detection dependency closure.
To build the existing backend through its owning executable:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release examples/resource_audit/main.exe
```

Use [development setup](../../../docs/development.md). Dune calls pinned
`build_native.py` with the backend manifest, links reported native flags, and
uses the chosen virtual-library implementation. There is no `backend.exe`.
Compilation checks linkage; native initialization validates catalog/schema
compatibility at runtime. Neither establishes native entity release, Metal
allocation or GUI acceptance. Run only through the collector described in
[main](../main.md) for actual sequential qualification evidence.
