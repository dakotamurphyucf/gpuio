# Statically linked native components

Status: OCH-23 design and implementation in progress. This document is a contract
draft; the ticket's clean-consumer and native acceptance gates are not yet passed.

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

The initial SDK admits at most64 registered component names. Properties are at
most64KiB; individual command/event payloads at most16KiB. These limits supplement
the host's retained-tree, transaction and mailbox limits. Component schemas may
choose smaller limits. Bin_prot codecs require exact consumption, reject trailing
bytes and validate domain values after decoding. Length checks precede allocation.

Rust factories validate properties before transaction acceptance. Mount, update,
command and render failures must produce typed failure observations and invalidate
the affected component's native lease; they must not unwind through the FFI.
Extension authors are trusted compiled-code authors. Panic containment is not
process isolation and cannot recover arbitrary unsafe-code corruption or aborts.

## Build composition

Use one Cargo dependency graph and one native static archive per application.
The proposed Dune integration makes the native archive backend a virtual-library
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
