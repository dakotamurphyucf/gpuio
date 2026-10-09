# Native editor range geometry — OCH-41

Checkpoint 2026-10-04, macOS arm64. Worktree based on
`83eb87e865c86717a8bc51b9db6fe1f379d909a9`, with substantial uncommitted changes.
This checkpoint repairs the native helper. The subsequent [public query
checkpoint](editor-range-api-och41.md) implements the OCaml query, codecs/controller
and gallery example. Neither OCH-41 nor OCH-17 is complete.

## Reproduction and repair

The pinned `InputBaseState::range_to_bounds` constructed a rectangle from only
the range's first and last positions. A rendered TestPlatform test on
`"abcdefghij\nx"`, range `5..12`, reproduced origin `(48, 0)` and size
`(-38.4, 52)` logical pixels. The baseline test failed rather than accepting the
negative width; log `editor-range-baseline-001.log`. The original helper is
unchanged in [the pinned snapshot](../catalog/sources/base-input-base-state.rs.txt).

The helper now validates UTF-8 source boundaries/order/length, translates masked
source offsets, and unions rendered glyph spans and endpoint carets across all
retained intervening lines. It reuses the range-background painter's shaped-cell
geometry through a shared visitor. Soft-wrap endpoints have explicit affinity;
each visual row's caret aligns with its own painted width. The cached layout
records source revision, masking mode and the actual glyph origin, including
alignment/scroll translation. Queries reject text or mask changes that have not
yet been painted. Read-only inspection leaves text, selection, composition and
history unchanged. See [the contract](../design/editor-range-geometry.md).

Bounds remain unclipped window-content coordinates from the last completed paint.
Both endpoints must be laid out; overscan can include offscreen rows. This is not
physical visibility, presentation or general code-editor/LSP geometry acceptance.

## Local validation

Commands use `GPUIO_JOBS=2 ./scripts/gpuio exec`. Logs are in ignored
`scratch/agents/root-20261003-release-notices/`; they are not build dependencies.

- Focused native repair: four tests passed in `editor-range-native-002.log`.
  A subsequent full run initially found a test-ordering mistake: separate
  TestPlatform updates allowed a paint before the assertion intended to precede
  paint. The corrected fixture performs request and observation within one
  update. No production behavior was weakened to satisfy that assertion.
- `cargo test -p gpuio-native --lib --features native-canvas-tests,native-image-tests --offline --locked -j2`:
  **912 passed, two existing macOS private-bus skips**;
  `editor-range-full-native-003.log`. Six new tests cover multiline intermediate
  extents, Unicode boundary rejection, stale source/layout and nonmutation,
  masked source coordinates and mask toggles, soft wraps/end affinity,
  coherent scroll coordinates/off-layout absence, empty carets and center/right
  alignment. The initial full run also caught a fixture using the wrong Style
  constructor; the final run contains the corrected public style operation.
- Base input-related tests: **268 passed**, `editor-range-base-input-001.log`:

  ```sh
  GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --manifest-path vendor/gpui-base/Cargo.toml --lib input --offline --locked -j2 \
    --config 'patch."https://github.com/zed-industries/zed.git".gpui.path="/Users/dakotamurphy/gpuio/vendor/gpui"' \
    --config 'patch.crates-io.accesskit_macos.path="/Users/dakotamurphy/gpuio/vendor/accesskit-macos"' \
    --config 'patch.crates-io.taffy.path="/Users/dakotamurphy/gpuio/vendor/taffy"' \
    --config 'profile.dev.debug=0' --config 'profile.dev.package."*".opt-level=1'
  ```

  Absolute override paths refer to this checkout. This runs the retained Base
  source with the existing local lock, without altering other switches.
- `cargo clippy -p gpuio-native -p gpuio-protocol --all-targets --features native-canvas-tests,native-image-tests --offline --locked -j2 -- -D warnings`:
  passed; `editor-range-clippy-001.log`.
- `dune build -j2 @runtest examples/gallery/main.exe`: passed, including the
  independent gallery backend; `editor-range-full-ocaml-001.log`.
- `./scripts/gpuio check-fmt`: passed; `editor-range-format-check-001.log`.
  Catalog audit (146 modules / 43 families), four edited-document relative-link
  checks and `git diff --check` also passed.

The existing `block 0.1.6` future-compatibility notice remains. No OS window,
physical shaping/IME or Linux execution is included in these results.

## Maintained source

The four Base file changes are appended to `third_party/patches/gpui-base.patch`.
The source pin is unchanged; the recorded patch SHA-256 is
`9f84521e14527bfdeec850c2a58eb5fc36d8395777f5c4f06a78f1ea74979bcc`.
Reconstruction from the hash-checked local pinned archive reproduces all **235
files** exactly, excluding only the ignored local Cargo lock and build directories.
Logs: `editor-range-reconstruct-001.log` and
`editor-range-reconstruct-compare-001.log`.

## Remaining acceptance

The subsequent API checkpoint covers the typed query, explicit revision/lease
behavior, paired codecs, Eio delayed-response/close tests and public example.
Qualify physical macOS shaping, masking, IME, accessibility and visual behavior on the
final sources. Reordered text uses the existing shaped-cell visitor; TestPlatform
results do not prove real bidirectional shaping. Required Linux nongraphical
checks and all broader milestone-07 release gates remain open.
