# Selectable list bridge and semantic metadata — OCH-41

2026-10-03, macOS arm64, milestone worktree based on `83eb87e`. This is a
bridge/admission checkpoint after the [pure selection model](selectable-list-model-och41.md),
not completion of the standalone list or the milestone. Existing GPUI, OCaml,
Bonsai and other dependency pins remain unchanged.

## Implemented

- Paired Op110 configuration and ordered Event74 list requests. Independently
  authored shared bin_prot fixtures cover operations/reset, every request kind,
  primary/secondary confirmation and new semantic roles. Existing tags stay fixed.
- Native admission checks list kind/role/handler, tree/table exclusivity, logical
  cursor membership, mounted option eligibility and unique sibling query ownership.
  Query relationships are checked after final structural parent links are known.
- Positive interaction generations retire stale policy/query/disabled input.
  Cursor/busy updates preserve queued relative navigation. Clearing configuration
  retains the generation watermark, preventing an old event from regaining validity
  when input is reinstalled.
- A transactional query-owner index revalidates editor changes without scanning
  unrelated siblings or logical rows. Transfers remove old registrations before
  installing new ones. Links are conservatively charged 128 bytes and released
  on reset; failure preserves the previous tree, registry and retained-byte count.
- ListBox/Option metadata with logical position, known/unknown total, selected and
  disabled state. Core validates constructors and transfers semantics to the
  managed row wrapper; native metadata preserves selection independently of focus
  and removes actions from disabled options. Metadata does not invent handlers.
- Session event admission validates current handler/revision/generation and enabled
  mounted keyed targets. Relative intents retain ordering through the bounded mailbox.

See the [accepted bridge contract](../design/selectable-lists.md#paired-bridge-contract).

## Local validation

Run repository commands with `GPUIO_JOBS=2 ./scripts/gpuio exec` (no extra `--`):

- `cargo test --offline --locked -j 2 -p gpuio-protocol`: **362 passed**.
- `cargo test --offline --locked -j 2 -p gpuio-native --features native-image-tests,native-canvas-tests --lib`:
  **797 passed, two existing private-D-Bus skips**.
- `cargo test --offline --locked -j 2 -p gpuio-native --test list_input --test lists --test tables --test tree_input --test accessibility --test session`:
  **35 passed**, including seven new list-input tests. A 100k logical collection
  query update touches only the editor and owner, with no structural traversal.
- `dune runtest test/view_api test/virtual_list`: passed paired OCaml fixtures,
  strict event decoding, checked metadata and real Core row-wrapper behavior.

- `cargo test --offline --locked -j 2 -p gpuio-native --test tree`: **eight passed**,
  including transactional rollback, bounded payload admission, randomized structural
  edits and small edits that do not copy/validate unrelated history.
- `cargo clippy --offline --locked -j 2 -p gpuio-native --all-targets --features native-image-tests,native-canvas-tests -- -D warnings`: passed.
- `dune build @runtest examples/gallery/main.exe`: passed all OCaml tests and gallery link.
- `GPUIO_JOBS=2 ./scripts/gpuio check-fmt` and `git diff --check`: passed.
No expected-output snapshots were promoted. Formatting used the repository toolchain.

## Still required

Production native focus/keyboard/pointer/query/IME and AX-action handlers; public
Core callbacks and query link resolution; Bonsai/Eio query and controller lifetime;
public gallery and independent consumer validation for the completed component.
The existing macOS selected-setter adaptation is TreeItem-specific: ListBoxOption
needs its own verified action support before claiming native selected setters.

These checks open no OS windows and establish no foreground keyboard, IME,
VoiceOver, GPU or physical macOS acceptance. Linux non-GUI gates remain required;
real Linux desktop qualification is deferred OCH-47. OCH-41 and OCH-17 remain open.
