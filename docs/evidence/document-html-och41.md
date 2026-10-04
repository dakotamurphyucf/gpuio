# HTML reader — OCH-41 local evidence

Checkpoint: 2026-10-04, macOS arm64. This implements the public reader-format
slice of the document catalog, not physical desktop or complete-family acceptance.
See [the public contract](../design/document-html.md).

## Implemented behavior

- `Document.Mode.Html` appends wire tag 3; existing mode bytes are preserved.
  Core/Bonsai use the existing registered source, document view and navigation
  callback. The gallery has an HTML tab with append/reset and search controls.
- The bounded worker parses the raw HTML DOM, validates depth/node/input limits,
  then normalizes prose whitespace and converts it to native reader blocks.
  It does not run the upstream recursive minifier before structural admission.
  The pinned `markup5ever_rcdom` 0.3.0 destroys nodes iteratively, including
  template contents. Exceeded limits produce the existing source fallback.
- Every image in paragraphs, lists, quotes and tables is replaced with the
  explicit asset plugin before layout. External/file URLs cannot reach the
  built-in HTML image loader. Missing resources retain alternative text.
  Invalid/nonfinite/negative/oversized dimensions are ignored.
- Entity decoding, inline line breaks, preserved preformatted whitespace,
  headings/lists/tables and ignored script/style/head/template content have
  focused checks. Wrapper nodes are flattened into separate virtual blocks.
- Deterministic per-preparation anchor identities keep repeated destinations
  distinct and group formatted fragments of one link. TestPlatform keyboard
  traversal includes an image link. Replacing HTML clears selection and old
  keyboard targets because original source spans are unavailable.
- Native selected-content copy remains plain text even with Markdown copy
  requested. Original source is retained for explicit Copy source. A production
  presenter test checks format-only retention, the window selection provider and
  unmount release. Worker tests reject superseded mode results and release work
  reservations after fallback and teardown.
- HTML Flow accepts preview budgets/observations. The full production preview
  fixture covers retained preparation, expansion, collapse, source view,
  observation fences, source reset, narrow/wide resize and unmount for HTML too.

The first regressions found dropped `<br>` breaks and trimmed preformatted code;
the bounded raw-DOM path fixes both. Full OCaml compilation found two chat-demo
matches requiring the new constructor; both now explicitly handle HTML.

## Validation

Executed serially with the repository toolchain:

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
  --workspace /private/tmp/gpuio-document-html-gallery-20261004
```

Protocol **375 passed**; native **832 passed**, with two existing private-D-Bus
skips on macOS. Strict lint and the full OCaml build/expect tests/formatting pass. A fresh
independent installed-gallery build also passes with
`INDEPENDENT_EXTENSION_CONSUMER_PASS example=gallery run=False`.
The structural catalog audit and `git diff --check` pass.

GPUI Base reconstructs byte-for-byte: **233 files**, excluding only its generated
root Cargo.lock. Patch SHA-256:
`d22f669deb75e685b8bb9e1cea732754918b98feed5b4dd7f85c8f56e0d42d25`.
The existing GPUI patch is unchanged. No source pins or switches changed.

No AppKit window was opened for these checks. Physical keyboard/clipboard,
VoiceOver, GPU/performance and current Linux/hosted acceptance remain open under
OCH-17; this evidence does not substitute TestPlatform for them. Other document
catalog gaps remain tracked in [the review](../catalog/documents-review.md).
