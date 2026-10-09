# Sidebar label styling — OCH-41

2026-10-02, macOS arm64, branch `milestone-07-gallery-release`, base `83eb87e`
plus local milestone changes. Stock OCaml 5.3/Bonsai v0.17/Core/Eio pins remain.
The [contract](../design/sidebar-styling.md) documents per-item/label styles and
the explicit experimental refinement rejecting whitespace-only destination names.
The 4,096-byte meaningful Unicode name bound is preserved without truncation.

Destinations now compose existing native links with stable passive icon/text
slots. No new protocol operation or dependency is needed. The Journeys gallery
offers **Style sidebar labels**, individual rounding, a featured label and an
independent count suffix alongside existing collapse and activation controls.

Three Core expect tests cover name boundaries, passive-style rejection, exact
public transactions, style update/reset, disabled handler rotation, compact mode,
icon/label independence and unmount dispatch. Six checked-in transactions are
generated from the public API and replayed by the native renderer; they complement
the independent protocol codec fixtures rather than claiming to replace them.

The native TestPlatform regression checks one exposed accessible destination,
retained focus handles, single label paint, independent item paint, pointer and
Enter/Space and simulated AX activation, disabled blur/paint/input fencing, compact suffix hiding
and cleanup. It discovered two production issues:

- Composed link children needed the existing decorative semantic wrapper and
  inherited disabled presentation. AccessKit retains raw hidden descendants;
  assertions follow ancestor visibility rather than counting raw labels.
- Deep composed sidebar rendering overflowed the default debug test stack.
  Default presentation, focus preparation and bulky style refinements now execute
  in separate helpers before recursive child traversal. The regression passes
  on the ordinary stack. An 8 MiB diagnostic run was used during investigation;
  no increased stack setting is required or committed. This is evidence for this
  real composition, not a claim of arbitrary maximum-depth qualification.

Checks completed at this checkpoint:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j2 \
  -p gpuio-native --features native-image-tests,native-canvas-tests --lib
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --offline --locked -j2 \
  -p gpuio-native -p gpuio-protocol --all-targets \
  --features native-image-tests,native-canvas-tests -- -D warnings
```

Full native suite: **661 passed, two existing private-D-Bus skips**. Strict lint
passes. Final full OCaml tests/format/gallery and the subsequent focused AX
action rerun pass. A fresh installed gallery consumer also builds successfully:
`INDEPENDENT_EXTENSION_CONSUMER_PASS example=gallery run=False`.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j2 \
  -p gpuio-native --features native-image-tests,native-canvas-tests --lib sidebar_labels_view_test
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery \
  --workspace scratch/agents/root-20260929-m7-resumed/sidebar-labels-installed-gallery
python3 scripts/audit_component_catalog.py
git diff --check
```

Catalog source/structure and whitespace audits pass. Unchanged wire codecs were
not rerun solely for this rendering/composition change.

No OS windows were opened. TestPlatform input and raw accessibility evidence do
not qualify physical macOS focus/IME/VoiceOver/GPU or Linux desktops. OCH-41 and
milestone 07 remain open, including the other catalog and release requirements.
