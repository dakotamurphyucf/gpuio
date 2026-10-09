# Decorative all-tabs menu icons — OCH-41

2026-10-02, macOS arm64, base `83eb87e` plus uncommitted milestone work.
`Tab_bar.Menu.create ~icons` accepts explicit Choice-ID keyed `Icon.Decoration`
values. Full configured names and native selected checks remain beside icons.
Duplicate/unknown IDs, oversized maps and active icon styles fail before bridge
submission. The frame's existing combined 4096-node/128-level bound still applies.
The Navigation gallery registers a scoped SVG and exposes **Show tab menu icons**.

Menu icons are ordinary retained native Icon placements, acquired when the tree
is accepted. Closed and offscreen rows keep their readers without constructing
row elements or requesting geometry-specific icon rasters. A native-only factory
creates fresh passive elements for visible virtualized rows. It holds a weak View
and guards current choice and child-vector identities. Generic child traversal
excludes these slots, so they do not also render beside the trigger.

Op102 remains unchanged. Select with menu presentation may have no children or
one structural slot per choice, each containing zero or one decorative leaf Icon.
Plain Select remains childless. Native admission rechecks descendant mutations,
including named images, active styles, nonstructural wrappers, wrong child kinds,
invalid shape and removal of menu presentation without removing its slots.

## Behavior evidence

- Checked public API tests cover duplicate/unknown IDs, nonpassive styles and
  oversized maps. Eight exact public OCaml transactions replay independently in
  Rust: initial icons, reorder/relabel, source/style replacement, partial/full
  reset, re-add, menu removal and complete disposal.
- Native admission tests retain icon identities across reorder/replacement and
  reject malformed descendant edits atomically with unchanged retained bytes.
- Two production-host tests run on TestPlatform with real SVG registrations and
  decoding. They retire registrations while mounted readers remain, verify closed
  and offscreen icons have no measured raster request, navigate to visible rows,
  check exact tint/decoded BGRA pixels and atlas presence, retain identities on
  reorder, replace one source/style, remove and re-add icons, then tear down by
  unmount or by closing an open-menu window. Both require retired readers and tree
  bytes to reach zero before global image-service cleanup. Weak render callbacks
  do not retain the closed View; no late events or idle frames remain.
- Decorative icon descendants do not add Image accessibility nodes. Menu names,
  checks, disabled state, focus and activation keep the prior menu contract.

## Reproduction and limits

Use the repository environment and `GPUIO_JOBS=2`:

```sh
./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe
./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-native \
  --test choice_menu_icons --test choice_menu
./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-native \
  --features native-image-tests,native-canvas-tests --lib
./scripts/gpuio exec cargo clippy --offline --locked -j2 -p gpuio-native \
  --all-targets --features native-image-tests,native-canvas-tests -- -D warnings
./scripts/gpuio exec cargo fmt --all -- --check
python3 scripts/test_extension_consumer.py --example gallery --workspace <fresh-local-path>
```

Full local checks pass: **715 native tests/two existing private-D-Bus skips**,
fourteen tab/menu admission/replay tests, strict all-target Clippy, Rust formatting
and full OCaml tests/format/gallery build. A fresh installed-gallery consumer
build also passes (`run=False`). No protocol
encoding changed; the preceding 345-test protocol result covers the unchanged
wire definitions. No OS windows opened. Decoded pixels/atlas checks on TestPlatform
do not qualify real GPU presentation, physical keyboard or VoiceOver behavior.
Tab indicator/color motion, flat split groups and physical/release gates remain open.
