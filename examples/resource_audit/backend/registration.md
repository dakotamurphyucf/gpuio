# How generated `registration.rs` installs both catalogs

[README](../README.md) · [Source](registration.rs) · [OCaml glue](backend.md)
· [Cargo manifest](Cargo.toml)

This generated Rust static-library entry point is extension-author glue.
`#[ocaml_interop::export]` exposes the symbol used by backend.ml; it accepts unit,
returns unit, and stores no OCaml values in components. It creates no window,
Bonsai state, sample, or resource-audit cycle.

`gpuio_native::registrations::install` receives two component factories and one
document-profile factory. Cargo aliases map `component_0` to the performance
probe, `component_1` to the resource audit, and `document_profile_0` to the example
review profile. The [manifest](../native.json), not incidental source ordering,
is the generator input defining this catalog.

The [registration contract](../../../rust/native/src/registrations.rs) validates
both registries before storing either in its process-static OnceLock. Duplicate
or incompatible factories fail; installing after a catalog is frozen fails Closed.
The generated `expect` intentionally propagates initialization failure rather
than falling back to a partial/empty catalog. The selected backend initializes
on the OS main thread before transport creation or catalog queries.

Factories provide trusted native behavior and validated descriptors; they do
not import OCaml callbacks. The host subsequently mounts the audit component
from checked OCaml instance bytes. This registration does not imply that a later
entity check, image decode, plugin action, or Metal probe succeeds.

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release examples/resource_audit/main.exe
```

Use [development setup](../../../docs/development.md). The backend Dune rule
builds this staticlib using its reviewed lockfile and platform flags. There is no
Rust binary for this file. Generator usage and manifest path resolution are in
[backend walkthrough](backend.md); do not hand-edit an alias to bypass a schema
or install another catalog dynamically. Actual run requires the external
collector in [main's walkthrough](../main.md), and its evidence is scoped to
closed-window live GPUI entity handles plus separately requested measurements.
