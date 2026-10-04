# Document link activation — OCH-41 local evidence

Checkpoint: 2026-10-04, macOS arm64, working tree based on `83eb87e`.
See [the contract](../design/document-link-activation.md).

## Behavior

Markdown/HTML links now deliver input source, mouse button and five release-time
modifier flags to `View.document ~on_navigate`. `Document.Navigation.Link` has
`{ url; activation }` payload; the optional activation preserves unknown provenance
when decoding the older URL-only variant. The gallery reports metadata, and the
chat application's pattern match uses the new record. No implicit URL opening
is introduced. Keyboard also identifies synthetic accessibility activation;
Touch records long-press separately. This is routing data, not a raw native event.

Navigation tag2 appends metadata under the unchanged Event30 envelope. Existing
Link/Line bytes remain unchanged. Paired OCaml/Rust fixtures cover mouse flags,
keyboard and touch. Event decoding rejects invalid/oversized/empty URLs,
truncation, trailing bytes, unknown variants/buttons, non-Boolean flags and
impossible nonmouse modifiers. Core tests preserve values, use the latest callback,
and reject stale handler/source identity and unmounted nodes.

Three focused native tests exercise actual rendered Markdown/HTML links on
TestPlatform: all five buttons, release-time modifiers differing from the press,
Tab/Enter routing, current source identity, old-picture rejection after generation
reset, and no ambient URL opening. Touch/synthetic keyboard conversion is tested
separately; no physical touch or accessibility claim follows from that helper.
Existing native selection-disabled link tests now check the richer keyboard event.

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
  --workspace /private/tmp/gpuio-document-activation-gallery-20261004
GPUIO_JOBS=2 ./scripts/gpuio check-fmt
python3 scripts/audit_component_catalog.py
git diff --check
```

Protocol **377 passed**; native **836 passed**, with two existing private-D-Bus
skips on macOS. The three focused native tests and three new OCaml expect tests
(protocol/Core) pass. Full OCaml build/tests/format pass.

Strict all-target Clippy, the independent installed gallery (`run=False`), official
formatting, catalog audit and diff checks pass. The installed gallery workspace is
`/private/tmp/gpuio-document-activation-gallery-20261004`.

No vendor, source pin or switch changes were needed for link metadata. The prior
styling fork reconstruction remains applicable and its snapshot still matches.
No AppKit window opened. Physical macOS, current Linux/hosted, notices/distribution
and remaining catalog/release acceptance stay open; neither OCH-41 nor OCH-17 is
complete.
