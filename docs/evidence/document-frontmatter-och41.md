# Frontmatter descriptions — OCH-41 local evidence

Checkpoint: 2026-10-04, macOS arm64, working tree based on
`83eb87e865c86717a8bc51b9db6fe1f379d909a9`.
See [the contract](../design/document-frontmatter.md).

`Document.Markdown_options.Frontmatter.Description_list` extends Op119 with tag2
(`7700010200` for the paired fixture); Disabled/Code_block tags0/1 are unchanged.
The native worker lowers supported leading YAML into bounded label/value rows.
Unsupported syntax and mappings larger than 128 entries retain the YAML code
fallback. Original source remains available; labels and values have separate
native text fragments, selection and DescriptionList/Term/Definition semantics.

The parser is adapted from the pinned GPUI Kit component source at
`84f57fdfcb4910623fb0bb7f795b077e249f9271`; attribution and Apache-2.0 terms remain
in the new vendor file. No dependencies, switches or upstream revisions change.
The existing worker configuration and interpretation guards retain source/native
identity and reject stale option-only results and link callbacks.

Four TestPlatform renderer checks cover actual two-column geometry and narrow
reflow, native semantic structure, partial mouse selection, window selected-copy
routing, exact whole-source copy, append retention/replacement clearing and preview
clipping/expansion. Displayed-fragment matching also asserts actual painted
backgrounds for labels, values and ordinary body text, with no match spanning a
label/value boundary. That test exposed an existing Flow-mode bug: Inline layout
could not find the owning document's decoration layers before prepaint. A balanced
TextView request-layout context now supplies the same owner as virtual rows.

Three integrated parser checks cover supported folded/literal/empty/Unicode
metadata, unsupported syntax and exact source preservation, and the 128/129-row
boundary without truncation. The existing retained Host parser-options regression
also installs Description_list, verifies separate fragments and stable source/
view identity, then verifies teardown. The copied vendor parser's own nine unit
tests are not included in the gpuio-native test count.

The public Documents gallery compares code/descriptions, supplies unsupported YAML
and reset examples, and retains append/selection/style controls. This is a built-in
renderer, not the separately required arbitrary static document plugin SDK.

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
  --workspace /private/tmp/gpuio-document-frontmatter-gallery-20261004
```

Protocol **378 passed**; native **847 passed**, two existing private-D-Bus skips
on macOS; strict all-target Clippy, full OCaml `@all @runtest @fmt`, and a fresh
independent installed-gallery build passed (`run=False`). The final local gallery
build also covers a notice-only wording correction after the full OCaml check.
Official formatting, the structural catalog audit and `git diff --check` pass.
A trailing empty context line in the vendor patch was removed with matching hunk
counts; final reconstruction still matches exactly. This changed patch formatting
only, not the tested source.

GPUI Base patch SHA-256:
`cd2b92e4d0111739339aaba7085622b87ca7a429c41bba1fdcfe058b2637325f`.
Reconstruction from the verified pinned archive matches all 234 files, excluding
only the snapshot root Cargo.lock. GPUI patch unchanged. No physical GUI window,
OS clipboard/IME/VoiceOver, GPU readback, Linux desktop or hosted acceptance is
claimed. OCH-41 remains open for custom actions, highlighter/defaults and static
document registration; OCH-17 physical/resource/distribution/release gates remain.
