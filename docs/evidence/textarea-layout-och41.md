# OCH-41 ordinary text-area layout — local checkpoint

2026-10-01, local macOS arm64 worktree, isolated stock OCaml 5.3 / Bonsai v0.17
and pinned Rust/GPUI dependencies. Uncommitted implementation; this is not a
published revision or release acceptance. All commands use
`GPUIO_JOBS=2 ./scripts/gpuio exec` unless noted.

## Verified behavior

- `dune runtest -j2 test/textarea_layout`: two expect tests pass. Defaults,
  bounds, single-line rejection, unchanged legacy editor config, independent
  Op78 bytes and retained reconciliation without seed replacement are covered.
- `cargo test -j2 -p gpuio-protocol --test text_area_layout --offline`: independent
  byte fixture, round trip, malformed/truncated/trailing input and margin bounds
  pass.
- `cargo test -j2 -p gpuio-native --features native-image-tests --lib
  editor::layout_tests --offline`: three rendered-host tests pass. Actual caret
  geometry changes with wrap and continuation indentation. Unrelated label and
  read-only updates preserve horizontal scroll; returning to default wrapping
  resets it. Native arrow navigation honors margins; oversized margins remain
  visible and repeated redraws do not oscillate. Layout changes retain entity,
  focus, text/selection/revision, marked composition and undo/redo across commit.
- GPUI Base reconstruction from the hash-verified cached upstream archive matches
  all **233 files** byte-for-byte (excluding generated Cargo.lock).
  Patch SHA-256: `7a87cb7b2c87a925762f398b14007683cb97b1f7c07e233dae96c02d21f58b2f`.

The initial native test incorrectly expected an explicit selection to apply
navigation margins. Source inspection established that selection uses minimal
reveal; the test now exercises actual `MoveDown` dispatch. The public contract
records that distinction. Auto-grow ignored explicit overrides in both native
margin paths; the adaptation preserves automatic defaults and honors overrides.

Additional completed checks:

- `cargo test -j2 -p gpuio-protocol --offline`: **307 passed** across the full protocol suite.
- Full native library run plus `--test textarea_layout_admission`: **555 passed,
  two existing ignored**, plus two admission tests. Invalid margins/wrong node
  kinds reject atomically, budget failures leave the old tree and revision, and
  removal returns retained bytes to zero.
- `dune build -j2 @runtest @fmt examples/gallery/main.exe`: full OCaml suite,
  formatting and gallery pass, including independent native example backends.
- `cargo clippy -j2 -p gpuio-native -p gpuio-protocol --all-targets --features
  native-image-tests --offline -- -D warnings`: passes.
- `cargo fmt --all --check`, `git diff --check`, and the structural component
  source audit pass. The source audit does not establish behavior acceptance.

## Remaining validation

Physical macOS keyboard/IME/clipboard,
whitespace appearance, accessibility/VoiceOver, resource/performance and installed
consumer acceptance remain open. TestPlatform tests create no OS windows and do
not establish physical desktop behavior. Current Linux build/unit/consumer checks
are still required; Linux desktop qualification remains deferred OCH-47.

Contract: [ordinary text-area layout](../design/textarea-layout.md).
