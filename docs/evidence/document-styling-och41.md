# Internal document styling — OCH-41 local evidence

Checkpoint: 2026-10-04, macOS arm64, working tree based on `83eb87e`.
See [the contract](../design/document-styling.md) and the public
[`Document.Style` signature](../../lib/core/document_style.mli).

## Behavior and bounds

Core/Bonsai configuration, theme-resolved reconciliation and Op118 now expose
six palette parts, paragraph/heading metrics, inline-code highlighting and four
passive code/table/header/cell refinements for Markdown/HTML. Code/Diff rejects
these settings. Defaults restore when removed. Ownership, visibility, clipping,
fixed-height and independent interaction/scroll overrides reject; this is a
checked data API, not arbitrary native callbacks.

Theme changes update the separately cached resolved style without republishing
source. Native updates retain prepared text and logical selection while resetting
virtual row measurements and invalidating the enclosing managed-list row. The
GPUI Base setter previously left cached virtual heights behind; it now remeasures
while preserving selection. The gallery demonstrates the public styling toggle.

Core expect tests cover invalid values/scopes, duplicate palette parts, theme
resolution, mode restrictions, source retention, silence for unchanged values
and reset. Independent Rust/OCaml default/clear fixtures use
`76000101000000000000000000000000000000` and `76000100`. Bounded Rust decoding
covers rich values, truncation, invalid options and declaration limits. Atomic
native admission tests cover wrong kind, stale IDs, invalid tokens/values,
mode changes with clear, and recycling.

The production presenter TestPlatform regression checks actual painted colors
for inline code and all four parts; changed/restored Flow height and virtualized
scroll extent; retained selection, source/native identity and exact displayed-text
Arc identity; and unmount release. It uses the real document worker and renderer.
It is not a physical desktop/GPU/clipboard test.

## Validation

Executed serially through the isolated repository toolchain:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j 2 -p gpuio-protocol
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j 2 \
  -p gpuio-native --features native-image-tests,native-canvas-tests --lib
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --offline --locked -j 2 \
  -p gpuio-native -p gpuio-protocol \
  --features gpuio-native/native-image-tests,gpuio-native/native-canvas-tests \
  --all-targets -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @all @runtest @fmt
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery \
  --workspace /private/tmp/gpuio-document-styling-gallery-20261004
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j 2 \
  -p gpuio-native --features native-image-tests,native-canvas-tests \
  --test document_style
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j 2 \
  -p gpuio-native --features native-image-tests,native-canvas-tests \
  --lib document_view::style_test
GPUIO_JOBS=2 ./scripts/gpuio check-fmt
python3 scripts/audit_component_catalog.py
git diff --check
```

Protocol **376 passed**; native **833 passed**, with two existing macOS
private-D-Bus skips. Admission **1 passed**; Core **2 expect tests passed**.
Strict all-target Clippy, full OCaml build/tests/format and a fresh independent
installed-gallery build pass (`run=False`). The follow-up stronger exact
prepared-text identity assertion also passes, followed by strict lint/format.
Catalog audit and diff checks pass.

GPUI Base reconstructs exactly (**233 files**, excluding only generated root
Cargo.lock); SHA-256 of its combined patch:
`46f454d1bbb98e69a630fb27aa9688ca63946683cfeacf432bbfe2c940b3bf4b`.
The GPUI patch is unchanged. No pins or switches changed.

No AppKit window was opened. Physical macOS, Linux/hosted, catalog and release
acceptance remain open; this slice does not complete OCH-41 or OCH-17.
