# All-tabs menu behavior — OCH-41

2026-10-02, macOS arm64, base `83eb87e` plus uncommitted milestone work.
Core/Bonsai `View.tab_bar_frame ~menu:Tab_bar.Menu.default` adds the fixed caret
before the suffix. The public gallery uses this API. Full tab names, current
order, selected checks, disabled state and the same controlled selection callback
flow through the existing Choice protocol. Op102 enables native menu presentation
on the internally generated Select. See the [design](../design/rich-tabs.md).

## Evidence

- Expect tests check invalid labels, whitespace Choice IDs, no-op reconciliation,
  current disabled/removed callback rejection and opcode pairing.
- Seven exact public transactions cover insertion into an existing frame,
  reorder/relabel, individual and whole-control disabling, re-enable, menu removal
  and complete disposal. Rust admission independently decodes/replays the fixture,
  compares tab/menu data, retains tab identity and confirms zero retained bytes.
  Wrong owners and stale generations reject atomically.
- Production-host TestPlatform tests verify Button/Menu/MenuItemRadio roles,
  selected checks and visible disabled rows, bounded rendering of a 40-row menu,
  Down/Home/End, typeahead over renamed labels, actual key-down/key-up activation,
  accessibility Click, current-data event fencing, Escape/Tab/outside cancellation,
  disabled closure, retained viewport/focus owners and final teardown. Menu
  selection leaves the tab viewport unchanged. Settled draws schedule no idle frames.
- Protocol tests check exact Op102 bytes, both Boolean values, truncated messages
  and invalid Boolean rejection.

## Reproduction and current limits

Use `GPUIO_JOBS=2` and the repository environment:

```sh
./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe
./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-protocol
./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-native --test choice_menu
./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-native \
  --features native-image-tests,native-canvas-tests --lib
./scripts/gpuio exec cargo clippy --offline --locked -j2 -p gpuio-native \
  --all-targets --features native-image-tests,native-canvas-tests -- -D warnings
./scripts/gpuio exec cargo fmt --all -- --check
python3 scripts/test_extension_consumer.py --example gallery --workspace <fresh-local-path>
```

Full local validation passes: **713 native tests/two existing private-D-Bus skips**,
**345 protocol tests/no skips**, twelve tab/menu admission/replay tests, strict
all-target Clippy, Rust formatting and full OCaml tests/format/gallery build.
A fresh installed-gallery consumer build also passes (`run=False`). These checks do not open OS windows and do not qualify
physical macOS keyboard, VoiceOver or GPU behavior. The physical gallery driver
has an authored, unrun all-tabs selection/no-scroll assertion. Decorative icons
in menu rows and tab indicator/color motion remain required OCH-41 work.
