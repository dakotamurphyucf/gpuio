# Markdown parser options — OCH-41 local evidence

Checkpoint: 2026-10-04, macOS arm64, working tree based on `83eb87e865c86717a8bc51b9db6fe1f379d909a9`.
See [the contract](../design/document-markdown-options.md).

## Implemented behavior

`Document.Markdown_options` exposes Disabled/Code_block frontmatter detection and
MDX syntax through the public configuration and gallery. Defaults preserve GFM;
nondefault options require Markdown. Op119 updates options without republishing
source. Default reset and mode changes admit atomically, stale node IDs reject,
and recycled nodes start with defaults. Independent paired bytes are
`7700010101` (both on) and `7700010000` (defaults).

Worker request equality includes options. Superseded jobs cannot install; the old
picture retains matching renderer flags while new preparation is pending. New
preparation replaces the interpretation atomically, clears incompatible selection
and retains the source allocation/native entity. A fresh identity rejects link
focus/click closures from previous interpretations even if source revision is
unchanged or settings cycle back. Renderer-only image refresh uses installed
parser options, avoiding an accidental main-thread reparse.

The tests exposed dropped MDX JSX content in the pinned conversion. The fork now
preserves nested inline children/marks/links and lowers flow children as blocks.
MDX expressions remain text/code, never evaluated. Malformed MDX falls back to
source and releases worker reservations. Original source remains available for
copying. This does not implement the separately required restricted frontmatter
mapping/description-list renderer or static document plugin SDK.

## Validation

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
  --workspace /private/tmp/gpuio-document-markdown-options-gallery-20261004
GPUIO_JOBS=2 ./scripts/gpuio check-fmt
python3 scripts/audit_component_catalog.py
git diff --check
```

Protocol **378 passed**; native **840 passed**, with two existing macOS private-D-Bus
skips. Focused tests: one Core expect test, one paired codec test, one atomic
admission test and four native parser/worker/presenter tests. Strict all-target
Clippy passes. Pending-install retention, real prepared fragments, selection reset,
old/current/A-B-A navigation delivery, native owner retention and unmount are
covered on TestPlatform. Touch/physical keyboard/VoiceOver/GPU are not claimed.

Full OCaml build/expect tests/formatting and a fresh independently installed
gallery pass (`run=False`), as do official formatting, catalog audit and diff checks.
Installed consumer workspace:
`/private/tmp/gpuio-document-markdown-options-gallery-20261004`.

GPUI Base reconstructs exactly (**233 files**, excluding generated root Cargo.lock).
Combined patch SHA-256:
`704e0bc675f685b3c6fc3eec9a6f79b36ae7868c0d02a994ba8c1d9f49d8266f`.
No source pins or switches changed. The GPUI patch is unchanged.

No AppKit window was opened. Physical macOS, current Linux/hosted, notices,
distribution and other catalog/release requirements remain open. This does not
complete OCH-41 or OCH-17.
