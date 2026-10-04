# Public document preview — OCH-41

The public API now exposes `Document.Config.create ?max_lines` and
`View.document ?on_preview`, including the Bonsai wrapper. The gallery adds a
compact Markdown preview with Expand/Collapse and a native presentation-status
label. The existing viewport document examples remain available separately.
See [the contract](../design/document-preview.md) and
[native clipping prerequisites](document-preview-prerequisites-och41.md).

## Behavior evidence

- Core expect tests reject invalid line budgets and incompatible modes/layouts.
  Changing or removing the limit emits only Op117, retains node/source identity,
  increments the configuration epoch, rejects stale observations and is silent
  for unchanged configuration. A clamped-rich event with no current limit is
  rejected.
- Independent OCaml/Rust golden bytes cover Op117 and Event76. Rust decoding
  rejects truncated packets, invalid Booleans, nonpositive epochs and line counts
  outside 1..4096. Event provenance validation rejects negative generations.
- Native admission rejects wrong node kinds and incompatible final modes,
  enforces monotone epochs, rolls back invalid batches atomically, and clears
  configuration when a node slot is recycled.
- A production presenter on TestPlatform checks native overflow, expansion,
  unchanged-frame silence, body collapse, source presentation, observer disabling
  and re-enabling, and unmount release. Limit changes preserve the native Markdown
  entity and exact prepared-snapshot allocation. Session checks reject wrong
  handlers, epochs, source generations and future source revisions.
- A stronger follow-up resets the real source to generation 2, waits for its
  worker result, and resizes the window from 220 to 2000 logical pixels. The
  observation changes from clamped to unclamped at the same configuration epoch
  and reports the new source revision/generation.
- The mailbox coalesces adjacent latest observations without crossing actions,
  configuration epochs or source generations. Eio dispatch checks the document
  registry before forwarding to Core/Bonsai.

The first presenter regression found that the initial mount path had not applied
preview configuration; it was fixed in the common render path. Full OCaml builds
also required explicit handling of the new event in Eio and both low-level bridge
examples; exhaustive matching remains enabled.

## Commands and coverage

Executed in the repository's isolated environment on macOS arm64:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j 2 -p gpuio-protocol
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j 2 \
  -p gpuio-native --test document_preview
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j 2 \
  -p gpuio-native --features native-image-tests,native-canvas-tests --lib
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j 2 \
  -p gpuio-native --features native-image-tests,native-canvas-tests \
  --lib document_view::preview_test
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --offline --locked -j 2 \
  -p gpuio-native -p gpuio-protocol \
  --features gpuio-native/native-image-tests,gpuio-native/native-canvas-tests \
  --all-targets -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @all @runtest @fmt
```

The protocol suite passes **374 tests**; native preview admission passes. The
full native library suite passes **826 tests**, with two existing private-D-Bus
skips on macOS. The stronger source-reset/resize assertion passes afterward;
only the test changed between those native runs. Strict lint and full OCaml
build/tests/formatting pass. A fresh installed public-gallery consumer also passes
with `INDEPENDENT_EXTENSION_CONSUMER_PASS example=gallery run=False`:

```sh
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery \
  --workspace /private/tmp/gpuio-document-preview-gallery-20261003
```

The official `./scripts/gpuio check-fmt`, catalog source audit and final `git diff --check` pass. No additional vendor
changes were needed beyond the previously reconstructed native prerequisites.

Logs are under `scratch/agents/root-20261003-release-notices/preview-*.log` and
are local evidence, not build inputs. These checks opened no AppKit window and
do not establish physical keyboard, IME, clipboard, VoiceOver, GPU or Linux GUI
acceptance. Hosted/Linux checks and OCH-17 release qualification remain open.
Standalone HTML, detailed internal styles and static document-extension APIs
remain separate document catalog gaps.
