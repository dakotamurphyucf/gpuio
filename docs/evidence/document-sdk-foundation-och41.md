# Static document SDK foundation — OCH-41

Local working-tree checkpoint, 2026-10-04, macOS arm64, based on
`83eb87e865c86717a8bc51b9db6fe1f379d909a9`. This implements authoring/preparation
boundaries, not public application attachment or ticket completion.

The new `gpuio-document-sdk` crate defines validated profile descriptors, immutable
registry bindings, bounded property admission, cooperative cancellation, checked
highlighters, explicit block/inline plugin presentation, native action-renderer
slots and revocable event contexts. It reuses the component SDK's event leases
without adding Base dependencies to that lightweight SDK. Rust and OCaml public
shape/ownership are documented in the [profile contract](../design/document-profiles.md).
The OCaml interface is a formatted draft, not an exported application API.

Checked adapters call the pinned Base Markdown/HTML preparation paths. They reject
wrong plugin names, excessive generated public strings and hook failures instead
of silently treating an error as a declined match. Text/NonText/Opaque declarations
remain distinct in the real prepared displayed-text projection. HTML requires a
host-supplied image replacement callback and does not invoke Markdown AST parsers.
Code highlights are checked for UTF-8 boundaries, ordered nonoverlap, style values
and aggregate run limits before being returned with the same prepared document
and profile. A thread test transfers the actual prepared result across a worker
boundary; the SDK itself does not schedule jobs or install them in the host pool.

## Validation

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j 2 -p gpuio-document-sdk
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --offline --locked -j 2 \
  -p gpuio-document-sdk --all-targets -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo check --offline --locked -j 2 --workspace
GPUIO_JOBS=2 ./scripts/gpuio check-fmt
python3 scripts/audit_component_catalog.py
git diff --check
```

**15 SDK tests passed** (six contracts, nine real preparation tests), as did strict
all-target SDK Clippy, workspace compilation, official formatting, the structural
catalog audit and diff checks. Tests cover schema/dependency rejection before
factory callbacks, owned property bytes, factory/highlighter panics, cancellation,
invalid Unicode ranges and whole-result rejection, revocable event lifetimes,
actual block/inline source/projection behavior, opaque/non-text search accounting,
HTML images and highlighting, generated-string and parser-chain limits.

One aggregate-run test initially hit Base's 16 KiB line limit before reaching its
intended check; splitting its large code into bounded lines now exercises aggregate
highlight rejection. Native alert fallbacks compiled, but their visual/AX behavior
and custom renderer callbacks have not been tested in a host window.

Root Cargo.lock adds only the local SDK package. A structural comparison against
the saved pre-change lockfile shows every existing package record unchanged. No
third-party pin or switch changes. Existing workspace test/lint commands include
the new member; no hosted run is claimed. No desktop windows opened.

## Remaining integration

Required work still includes protocol/catalog registration, typed Core/Bonsai/Eio
attachment and event decoding, backend generation, exact worker identity and
installation fencing, parser/render resource reservations (including opaque native
allocations), failure observations, source/property/visibility/modal revocation,
renderer/selection/copy/accessibility/focus behavior, offscreen virtual controls,
application defaults, a public gallery and an independent installed consumer.
These SDK tests do not establish any of those host-level claims. Full OCH-41 and
OCH-17 macOS/resource/distribution/hosted/review/publication acceptance remain open.
