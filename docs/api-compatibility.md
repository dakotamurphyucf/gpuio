# Public API compatibility and limits

This describes the current experimental implementation, not a stable v1 release
announcement. [Current status](status.md) records open acceptance work. Use the
[starter guide](getting-started.md) for a compiled application and installation
example; older imported architecture/API sketches are design history, not exact
call signatures. Public `.mli` files are the authoritative current interfaces
where present; the [API layer map](api-layers.md) explains generated wrappers,
inferred protocol interfaces and reachable integration helpers.

## Build compatibility

GPUIO has one repository and release train for OCaml, Rust, the native backend and
vendored dependencies. Build all parts from the same checkout and exact lockfiles.
The current baseline is stock OCaml 5.3.0, the Jane Street/Bonsai v0.17 family,
Core, Eio 1.3, Dune 3.24.2 and Rust 1.97.1. OxCaml and an Async application scheduler
are not required. The maintained Bonsai/native patches are build inputs; an
unpatched upstream package is not a tested replacement.

There is no published stable API or binary plugin ABI. Public source signatures,
labels and behavior may change during this experimental phase; recompile consumers
when updating. Do not combine an installed OCaml prefix from one revision with a
native archive from another, even when both report the same protocol epoch.

The [bridge](design/bridge-v1.md) uses encoding family v1 and an exact epoch-3
handshake. Epochs 1–2 and unknown future epochs are rejected. Epoch 3 is still
unpublished and has accumulated paired additions without allocating new capability
bits; the full legacy mask is not evidence that arbitrary epoch-3 builds interoperate.
The epoch is a wire boundary, not the library's release version or an application
persistence format. Rebuild both languages together.

Native component/profile packages use explicit names, versions and schema
fingerprints, checked SDK/source identities and one generated Cargo graph. This
catches incompatible registrations; it does not provide binary compatibility
between independently compiled Rust archives. Upgrade the package codecs, native
factory, manifest and lock together. See [components](design/extensions.md) and
[document profiles](design/document-profiles.md).

## State, units and timing

- Handles and observed snapshots are scoped to generation-checked owners. A
  remounted editor or recycled window slot is a different object. Do not persist
  native IDs or replay a previous process's snapshots/commands.
- Editor selections and source offsets count UTF-8 bytes. Use validated selection
  constructors; character counts, UTF-16 indices and grapheme counts are different.
  UI dimensions and reported geometry use logical pixels unless documented
  otherwise. Bounds are not proof of physical visibility.
- Views/configurations describe desired state. Events, correlated commands,
  source publication, layout and physical presentation are different stages.
  `Window.request_frame` observes a native render callback and can be delayed
  while occluded; it is not an application-data readiness barrier.
- Effect callbacks run on the OCaml UI domain. Native synchronous editing,
  layout, paint and input policy do not call OCaml. Use Eio scopes for asynchronous
  work and inspect each command's typed result, including stale/closed/capacity
  outcomes. Cancellation and window closure retire deliveries according to the
  owning API; they are not interchangeable with successful completion.
- The default shared Bonsai clock wakes at 60 Hz. This is a runtime timer, not a
  request to redraw an idle native window at 60 FPS. `App.run ~tick_hz` changes the
  clock resolution/cost tradeoff; motion and input remain native. See
  [runtime scheduling](design/runtime.md).

Derived `bin_io`, `sexp` or equality support does not imply a stable persistence
schema. Persist application-owned records with explicit versions/migrations;
reconstruct validated GPUIO values and live resources when loading.

## Supported scope and meaningful limits

macOS is the initial functional/release target. Linux compilation, unit/private-bus
and independent-consumer checks remain required, while real Linux desktop
qualification is OCH-47. Windows is outside v1. Read the
[platform policy](platform-release-policy.md) and each OS capability's result type.

Current bounded contracts include 32 simultaneously live windows, ordinary editor
text up to 262144 UTF-8 bytes, and an application-wide limit of 64 pending editor
requests. Those numbers are admission limits, not performance promises. Large
read-only documents use the document source/virtual reader path; a full editable
code editor/LSP is deferred. See [editing](design/native-editor.md),
[documents](design/documents.md) and the [family ledger](catalog/README.md).

Managed lists virtualize active views and native caches, not all application-owned
history or metadata. Use paging for unbounded records. Static native extensions
are trusted compiled code; panic containment is not a security sandbox. Text
projections, arbitrary custom renderers and nested scroll containers have distinct
selection/search/focus obligations. Core library functionality does not imply an
arbitrary plugin satisfies those obligations.

Hot reload, a dynamic plugin ABI, full code-editor/LSP infrastructure, editable
spreadsheet-style grids, general terminal/multimedia engines and comprehensive
docking are outside this release's required scope. Native drawing, animations,
read-only tables/trees/documents, multiple windows and app-declared controls have
implemented APIs, with their exact evidence and gaps in the catalog. Application
authors can compose or extend them without assuming deferred subsystems ship as
ready-made components.

## Release qualification

A successful source/consumer build proves compilation and linking in that tested
environment. TestPlatform checks do not establish real IME, clipboard, VoiceOver,
GPU timing or clean-machine distribution. Local ad-hoc application signatures are
not Developer ID/notarization. These remain explicit milestone-07 gates in
[status](status.md) and [distribution](distribution.md).
