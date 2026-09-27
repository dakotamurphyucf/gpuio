# Statically linked native components

Status: OCH-23 is implemented and locally accepted on macOS, including the
separately packaged component, staged clean consumer and native lifecycle/input
checks. This document records the implemented contract. See the
[evidence ledger](../evidence/extensions-och23.md) and
[milestone handoff](../milestone-5.md) for validation and hosted delivery status.

## Ownership and identity

A component package supplies typed OCaml codecs and native Rust behavior. An
application consumes its ordinary OCaml library. Native code owns synchronous
layout, paint, input, focus and accessibility; the bridge only transports owned
bytes and queued semantic events. There are no OCaml callbacks from native paint,
layout or input handlers and no borrowed Rust pointers in public OCaml handles.

A schema has a qualified component name, a positive version and a lowercase
SHA-256 fingerprint of the package's documented property/command/event schema.
Names contain at least two dot-separated lowercase ASCII identifier segments.
Registration rejects duplicate names, incompatible SDK/GPUI versions and malformed
schemas before opening the application. Compatibility requires the exact schema;
there is no implicit decoding fallback or schema migration.

The retained tree supplies generation-checked window/node/handler identity.
Component replacement/reset supplies a separate monotonically increasing instance
generation. Properties are snapshots; commands have explicit positive sequence
identities and are not repeated by native frame rendering. Obsolete sinks and
commands must fail after replacement, unmount or window close. Native background
work must hold a revocable event sink, never an OCaml closure or runtime value.

## Bounds and error handling

The initial SDK admits at most 64 registered component names and 256 retained
instances per window. Properties are at most 64 KiB; individual command/event
payloads at most 16 KiB. These limits supplement
the host's retained-tree, transaction and mailbox limits. Component schemas may
choose smaller limits. Bin_prot codecs require exact consumption, reject trailing
bytes and validate domain values after decoding. Outer byte lengths are checked
before host allocation. Package decoders must additionally validate nested
collection lengths before allocating; a general derived bin_prot reader does not
provide that guarantee by itself. The fixed-byte sample has no nested allocation.

Rust factories validate properties before transaction acceptance. Mount, update,
command and render failures must produce typed failure observations and invalidate
the affected component's native lease; they must not unwind through the FFI.
Extension authors are trusted compiled-code authors. Panic containment is not
process isolation and cannot recover arbitrary unsafe-code corruption or aborts.

## Build composition

Use one Cargo dependency graph and one native static archive per application.
The Dune integration makes the native archive backend a virtual-library
implementation: ordinary applications select the default host; a component
consumer selects a generated backend containing the same host and registered
extension crates. This avoids linking independently compiled copies of GPUI or
native registries. The composition generator supplies the Rust registration entry
point, so consumers do not need to author Rust. Component authors publish an
ordinary Rust library and typed OCaml library with a compatibility manifest.

The public SDK is the only native integration surface used by the sample package.
It must expose native element/context, focus/accessibility and queued event APIs
without private host imports. Exact source pins and schema compatibility are
checked together. Dynamic loading and a stable binary plugin ABI are excluded.

## Acceptance work

Build a separately packaged interactive component and a clean OCaml consumer.
Exercise typed properties, commands and events, keyboard/pointer/accessibility
actions, malformed payloads, schema rejection, panic containment, stale events,
repeated mount/unmount and window close. Native output must remain accessible and
participate in semantic automation. Check the default backend still builds every
existing example. Record actual macOS behavior separately from Linux compilation
and informational compositor runs. Full release coverage remains OCH-17.

## Implemented integration

`Gpuio.Extension.Definition` binds three codecs to a schema; `Instance.create`
encodes a property snapshot and optional sequenced command. `View.extension`
(and the Bonsai view adapter) dispatches typed `Data`, `Mounted`,
`Command_completed` and `Failed` observations. A schema change replaces the node;
a higher generation disposes and remounts native state. Property/configuration
changes rotate the handler and revoke old event sinks. Hidden/disabled sinks
reject input; commands still follow the package's explicit command policy.

One transaction may set an instance's extension configuration once. Commands run
at application time, before rendering, so coalesced frames cannot skip commands.
The host remembers the highest sequence even when subsequent snapshots omit the
command. A lower sequence or the same sequence with different bytes rejects the
whole transaction. A failed component remains failed until its generation changes.

`Gpuio_eio.App.extension_catalog ()` initializes the chosen backend and returns
its exact schemas. Call on the OS main thread before `App.run`; registration is
then immutable. Missing or incompatible factories and malformed package payloads
reject transactions before publishing tree changes. Package hook failures become
instance `Failed` events. Background producers hold weak event sinks; delivery is
bounded by the ordinary input mailbox and reports overload to the producer.

Rust authors implement `Factory` and `Component` from `gpuio-extension-sdk`, using
its re-exported pinned `gpui`. The host contains hook panics. Authors must guard
installed input callbacks with `EventSink.guard`; direct arbitrary GPUI callbacks,
custom element layout/paint code and unsafe code remain trusted author code.
Guarded callback panics revoke the lease; the next host lifecycle/render boundary
observes the failure. Unmount cancels package-owned work; the host also contains
ordinary unwinding failures from unmount/drop. This is source compatibility at the
pinned SDK revision, not an ABI promise.

`examples/extension_package` supplies an OCaml library and an independent Rust
crate. `examples/extension_consumer/native.json` lists the selected packages;
`scripts/compose_backend.py` generates one Cargo archive and a Dune implementation
of `gpuio.native`. The application adds that implementation to its libraries.
Keep the generated files and reviewed Cargo.lock in source control. The default
backend remains available for applications that use only built-in components.

The SDK supplies one primary `Context.focus` handle per instance. The component
binds that handle to exactly one accessible element; the host wrapper records it
for traversal but does not register a second accessibility focus target. Nested
controls may own additional handles under the package's documented focus policy.
Pointer callbacks use `EventSink.guard_pointer`; keyboard/accessibility actions
use `guard`. Inherited pointer disabling does not disable keyboard or accessible
activation. Both guards reject hidden, disabled, obsolete and closed instances.
