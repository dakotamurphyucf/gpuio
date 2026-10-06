# How `Gpuio_resource_audit` describes one audit cycle

[README](../README.md) · [Implementation](gpuio_resource_audit.ml)
· [Interface](gpuio_resource_audit.mli) · [Native audit](../rust/src/lib.md)

This qualification-only library constructs a checked native extension instance.
It is not an OCaml memory collector or an interactive component. `X` abbreviates
`Gpuio.Extension`; schema/codec construction is pure and does not register Rust.
The dedicated composed backend must supply the matching factory.

`Properties.t` contains warmups, measurements, cycle, and Metal requirement.
Validation accepts warmups 1–3, measurements 1–30, and cycle 1 through their sum.
Encoding creates exactly four unsigned bytes, with Boolean 0/1. Decoding rejects
wrong lengths/trailing bytes and invalid flags/ranges before instance admission.
`let%map.Or_error` sequences checked results here, not reactive Bonsai state or
asynchronous effect work.

`schema` pairs `qualification.resource_audit`, version 2, and fingerprint of
[schema.txt](../schema.txt) with Rust. The properties codec has maximum four bytes.
The dummy command/event codec rejects every value: this component has no Data
payloads or commands. Ordinary host Mounted/Failed signals still exist; declared
one-byte maxima do not create a valid command route.

`instance` always uses generation 1 and accessible label “Native entity
qualification”. The shared lifecycle workload constructs a fresh instance for
each separate sequential window, rather than updating cycle properties in one
retained mount. Rust rejects changed properties after mounting. The workload's
reactive `let%arr` derives a `V.extension` beside actual resources; failure becomes
an effect raising a typed error. This library itself owns no Bonsai graph.

Mounting registers native window identity; after native close, an app-owned audit
subscription checks settled entity state and writes qualification stdout.
No event is delivered through a retired component. The external Python collector
combines that record with acknowledged OCaml retirement and sends continuation
on stdin. This library supplies configuration, not collection or a pass verdict.
See [main](../main.md) and [extension contract](../../../lib/core/extension.mli).

From the repository root using [development setup](../../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune runtest -j2 examples/resource_audit/ocaml
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release examples/resource_audit/main.exe
```

The inline expect test rejects invalid ranges/flags/lengths and checks valid
round-trip bytes. It requires no GUI and proves no native entity or memory result.
The executable must be run with the collector, not directly as a completed smoke.
For another audit configuration, keep immutable per-mount properties and paired
fingerprint validation; do not reuse this instrumentation as an application UI
or claim registration charges measure physical/GPU memory.
