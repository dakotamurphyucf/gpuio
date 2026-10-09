# Native tab target appearance — OCH-41

2026-10-02, local macOS arm64, base `83eb87e` plus uncommitted milestone work.
This adds checked Core/Bonsai presentation for actual native tab targets: Tab,
Outline, Pill, Segmented and Underline, target height/padding, inter-tab gap,
shared styles and stable Choice-ID overrides. The gallery cycles the five variants
and toggles custom target sizes/hover while preserving its rich labels and editors.

Selection remains in `Choice.Config`; native focus and the active keyboard choice
remain in the existing owner. Updating appearance does not remount labels, rewrite
selection or erase a navigation request awaiting OCaml acceptance. Omission keeps
legacy styling; removing a supplied appearance explicitly resets it.

The paired unpublished epoch-3 protocol appends Op98 with a bounded optional
presentation. Core checks geometry, property/state scope, unique IDs and aggregate
work limits. Rust decoding independently bounds collections and declaration work,
including empty raw blocks; admission rejects semantic/layout policy fields outside
the target presentation scope, charges retained allocations and rolls back invalid
updates atomically. IDs absent from the current choices are intentionally ignored.

These are static GPUIO theme-aware variants with native hover and style states.
This does not claim the pinned styled layer's exact pixel defaults, indicator/color
animation, interactive suffixes, max-label-width layout or overflow/reveal behavior.
Those remain required [rich-tab work](../design/rich-tabs.md).

## Tests

Two Core expect tests cover geometry, forbidden fields/states, duplicate IDs and
aggregate limits, paired Op98 bytes, all five variants, no-op updates and explicit
reset. Appearance changes produce one presentation operation and no new choice,
handler, node or label owner.

Two protocol tests cover matching independent operation bytes, all five variants,
Unicode IDs and styled payloads, truncation/trailing bytes and malformed geometry,
identity/state/work bounds. A native admission test verifies tab-only use, atomic
rollback, unchanged selection, retained-byte accounting and full release on reset
and unmount.

A production-host TestPlatform regression runs with both plain and rich labels.
It checks actual target width/height and selected paint for every variant, per-ID
style precedence, semantic identity, native hover paint, keyboard routing,
focused-active paint while selection is awaiting an application echo, reorder,
disabled appearance, reset, no idle frame scheduling and cleanup. No physical
OS window was opened; this is not VoiceOver/OS keyboard or hardware-pixel evidence.

## Validation

Use `GPUIO_JOBS=2` and the isolated repository toolchain:

```sh
./scripts/gpuio exec cargo clippy --offline --locked -j2 -p gpuio-native \
  --all-targets --features native-image-tests,native-canvas-tests -- -D warnings
./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-native \
  --features native-image-tests,native-canvas-tests --lib
./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-protocol
./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-native \
  --test tab_appearance --test control_labels
./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe
python3 scripts/test_extension_consumer.py --example gallery --workspace <fresh-local-path>
python3 scripts/audit_component_catalog.py
git diff --check
```

Passed: **707 native tests/two existing private-D-Bus skips**, **341 protocol tests/
no skips**, five admission tests, strict all-target lint and targeted Core tests.
Full OCaml tests/format/gallery build pass. A fresh installed-package gallery
build reports `INDEPENDENT_EXTENSION_CONSUMER_PASS example=gallery run=False`.
The physical gallery driver now cycles variants and custom target sizes while
checking retained editor contents; syntax passes, but that desktop run is unexecuted.
Catalog and whitespace audits pass. Physical appearance/keyboard/VoiceOver, Linux required
automation and broader milestone release acceptance remain separate work.
