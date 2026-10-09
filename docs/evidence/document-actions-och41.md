# Declarative document actions — OCH-41

Local working-tree checkpoint, 2026-10-04, macOS arm64, based on
`83eb87e865c86717a8bc51b9db6fe1f379d909a9`. This is one catalog implementation slice;
it does not complete OCH-41 or qualify a release.

`Document.Actions` configures bounded native code/table buttons and independent
Copy visibility. Core and Bonsai `View.document ~on_action` receive immutable
snapshots with source revision/generation and input metadata. Eio rejects released
sources and obsolete generations before callback dispatch. Op120/Event77 append
the checked protocol; configuration epochs fence stale callbacks after A/B/A.
See the [contract](../design/document-actions.md).

Four production Host/TestPlatform tests cover Markdown/HTML pointer clicks, actual
AX focus plus Enter, forward/reverse painted tab traversal, disabled controls,
queued code/table payloads, source ranges, HTML span absence and teardown. They
also cover config/interpretation/source-reset rejection, preview clipping and
expansion, native test clipboard Copy, Copy visibility, code/control nonoverlap,
narrow wrapped geometry, retained selection/prepared-text identity and virtual
scroll-height restoration. A mailbox test checks large snapshots against byte
limits, FIFO delivery, bounded encoded batches and accepted-input-before-Stopped
ordering. These are native renderer tests on TestPlatform, not physical macOS
input, OS clipboard or VoiceOver acceptance.

The tests found a focus bug: the host used exact TextView focus equality and
replaced child-button focus with its root fallback on redraw. The document now
recognizes focused descendants. Tab traverses painted controls after TextView's
logical links; offscreen virtual control navigation still requires qualification.
Action rows sit below code in normal flow, with wrapped buttons and visible focus/
disabled states. Config changes invalidate TextView measurements and enclosing
managed-list rows. The GPUI Base action layout patch reconstructs exactly.

The public Documents gallery includes custom actions, enabled state, Copy controls
and payload notices for Markdown/HTML. Its initial Markdown includes a snippet.
This work also repairs the earlier unsupported-YAML example button: it had been
placed under an incompatible HTML branch and is now reachable on Markdown.

## Validation

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j 2 -p gpuio-protocol
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j 2 \
  -p gpuio-native --features native-image-tests,native-canvas-tests --lib
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j 2 \
  -p gpuio-native --test document_actions
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --offline --locked -j 2 \
  -p gpuio-native -p gpuio-protocol \
  --features gpuio-native/native-image-tests,gpuio-native/native-canvas-tests \
  --all-targets -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @all @runtest @fmt
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery \
  --workspace /private/tmp/gpuio-document-actions-gallery-20261004
```

Protocol **380 passed**; native **852 passed**, two existing private-D-Bus skips
on macOS; action admission **1 passed**. Strict all-target Clippy passes after
removing one redundant unit-value binding. Full OCaml `@all @runtest @fmt` passes,
including Core/Bonsai/Eio and all examples. A fresh independent installed-gallery
build passes (`run=False`), as do official formatting, the structural catalog audit
and `git diff --check`. The initial full OCaml run caught and led to fixes for the
Bonsai callback signature and exhaustive event cases in two low-level examples.

GPUI Base patch SHA-256:
`7826b01cfa05a587727d8c3dd84ec6cf552a314044d7b6261971ea9df004ef50`.
Reconstruction from the hash-verified pinned archive matches all **234 files**,
excluding only the root snapshot Cargo.lock. GPUI patch and dependency pins are
unchanged. No physical GUI window was opened for these tests. No Linux desktop,
GPU/IME/VoiceOver, hosted CI, signing, publication or clean-machine acceptance is
claimed. Arbitrary static document action renderers, highlighters/defaults and
parser/renderer registration remain required catalog work.
