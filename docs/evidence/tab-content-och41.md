# Structured tab content — OCH-41

2026-10-02, local macOS arm64, base `83eb87e` plus uncommitted milestone work.
Core and Bonsai expose `View.Tab_content` and `View.tab_bar_with_content` for
independent prefix/suffix controls, Default/Custom/Hidden labels and a maximum
whole-tab width. ChoiceConfig owns selection and complete accessible names.
Stable Choice-ID slots retain native editors and callbacks through reorder and
presentation changes. The Navigation gallery adds close/reorder/restore actions
and long-name truncation using only public APIs.

Prefix/suffix hit areas do not activate the surrounding tab. Compound tab keys
run only when the tab list itself holds focus. Independent controls remain usable
when a choice is unselectable; explicit ancestor Disabled/Inert still suppresses
the subtree. Default labels ellipsize, custom labels clip, and Hidden/icon-only
tabs are exempt from the width cap. Prefix/suffix do not shrink. Extremely small
caps can clip adornments; padding may impose a larger box. Fully clipped controls
are skipped by Tab traversal; partially visible controls remain eligible.

Paired unpublished epoch-3 Op99 carries optional width and ordered label modes.
Rust independently validates payloads, structural slots and passive label content,
including nonstructural child updates; malformed batches roll back atomically.
All parts and wrappers fit within 4096 nodes/128 levels. Retained mode allocations
are charged and released on reset/unmount. Core rejects unknown/duplicate IDs,
blank configured names, interactive/observed labels and excessive trees. The
shared passive-label validator now also rejects hover observers in both runtimes.

## Coverage

Three Core expect tests check invalid content/geometry/IDs/names/bounds, independent
Op99 bytes, keyed reorder without remount, latest action callbacks, no-op renders,
reset and retired callback rejection. Exact public create/reorder/reset/dispose
transactions are shared with a Rust native-tree replay test. That boundary review
caught and fixed empty base-style blocks on structural slots: the slot factory
now explicitly uses Style.empty rather than the generic container default.
The protocol test independently agrees on
bytes and checks every truncation, trailing data, invalid dimensions and excess
modes. Native admission covers label-only passivity, forbidden wrapper behavior,
late binds/hover/style/metadata, reset shape, rollback and retained-memory teardown.

A production-host TestPlatform test checks measured tab/editor/Close dimensions,
pointer, keyboard and simulated accessibility Close actions without tab selection,
editor arrow keys/typing, reorder with retained draft and semantic identity,
independent disabled-choice Close, ancestor-disabled suppression, label selection,
custom decorative content, fully clipped Close traversal, icon-only width exemption,
idle frame scheduling and complete unmount cleanup. These tests open no OS window;
they are not physical keyboard/IME, VoiceOver or GPU-pixel qualification.

The physical Navigation driver contains an additional close/reorder/restore and
truncation walkthrough, authored but unrun. No new Linux execution is claimed.
Overflow/reveal, indicator/color motion, full macOS qualification and OCH-41/OCH-17
acceptance remain open.

## Reproduction

Use the repository's isolated environment with `GPUIO_JOBS=2`:

```sh
./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe
./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-protocol
./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-native \
  --test tab_content --test tab_appearance --test control_labels
./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-native \
  --features native-image-tests,native-canvas-tests --lib
./scripts/gpuio exec cargo clippy --offline --locked -j2 -p gpuio-native \
  --all-targets --features native-image-tests,native-canvas-tests -- -D warnings
python3 scripts/test_extension_consumer.py --example gallery --workspace <fresh-local-path>
python3 scripts/audit_component_catalog.py
git diff --check
```

Passed: full OCaml tests/format/gallery build, **342 protocol tests/no skips**,
**708 native tests/two existing private-D-Bus skips**, seven admission/replay tests
strict all-target lint and a fresh installed-gallery consumer build (`run=False`).
Catalog/whitespace audits and Python driver syntax checks also pass. No physical
desktop test was run for this checkpoint.
