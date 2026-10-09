# Default monospace selection — OCH-41

Local checkpoint, 2026-10-04, macOS arm64, dirty worktree based on
`83eb87e865c86717a8bc51b9db6fe1f379d909a9`. This addresses the default-font
policy identified in the [theme review](../catalog/style-theme-review.md).
It does not establish clean-machine font availability or physical glyph quality.

## Change

After Base initialization and before native window creation, GPUIO chooses its
default monospace family once per GPUI application. It prefers the platform
default and then known monospace alternatives listed by the text system, falling
back to `.SystemUIFont` if none is listed. An explicitly configured nondefault
family is preserved without enumeration. The result lives in Base's typography
token, shared by rich inline/code blocks and source/code/diff editor containers.
The source container no longer hardcodes a separate platform font. Other theme
fields and public per-view style values are not mutated by initialization.

The gallery's text-overflow preview now selects `.SystemUIFont` instead of Menlo,
so it no longer depends on that macOS-specific family. This preview demonstrates
ellipsis behavior, not a guarantee of fixed-width text.

See [the contract](../design/default-fonts.md). There is no new dependency, vendor
patch, wire format, FFI callback, recurring timer or process-global font cache.
The native font inventory is queried at most once during application initialization,
not during document paint or theme updates.

This is not described as a reproduced universal crash fix: the pinned GPUI core
already attempts generic fallback fonts. That fallback may be proportional and
only panics if every candidate fails. Likewise the final system-family fallback
here is not guaranteed monospace. Full font/glyph availability remains an OS and
distribution validation concern.

## Local checks

Commands use `GPUIO_JOBS=2 ./scripts/gpuio exec`. Scratch logs are under
`scratch/agents/root-20261003-release-notices/` and are not build dependencies.

- `cargo test -p gpuio-native --lib --features native-canvas-tests,native-image-tests --offline --locked -j2 font_defaults`:
  **four passed**. Candidate tests cover both macOS and Linux lists, default-first
  choice, alternate priority independent of inventory order, final alternate,
  empty/unrelated inventories and explicit-family preservation without probing.
  TestPlatform checks initialization once per application, independent applications,
  and preservation of other theme fields. Initial unused-mut warnings in the
  fixture were removed before the full run. Log `default-font-native-001.log`.
- `cargo test -p gpuio-native --lib --features native-canvas-tests,native-image-tests --offline --locked -j2`:
  **904 passed, two existing macOS private-bus skips**, log
  `default-font-full-native-001.log`. This includes existing document render,
  selection, layout and lifetime regressions. It does not prove each candidate
  family was installed or visibly rendered on either OS.
- `dune build -j2 @runtest examples/gallery/main.exe`: **passed**, log
  `default-font-full-ocaml-001.log`, including the independent gallery backend.
- `cargo clippy -p gpuio-native -p gpuio-protocol --all-targets --features native-canvas-tests,native-image-tests --offline --locked -j2 -- -D warnings`:
  **passed**, log `default-font-clippy-001.log`.
- `./scripts/gpuio check-fmt`: **passed**, log
  `default-font-format-check-001.log`. Catalog audit, all 228 source-snapshot
  hashes/unique manifest entries, edited-document relative links and
  `git diff --check` pass. Structural coverage remains 146 modules in 43 families.

The existing `block 0.1.6` future-compatibility notice and macOS duplicate-library
link warnings remain. Linux candidate-order logic is tested on macOS; this is
not a fresh Linux build or Linux desktop result.

## Remaining acceptance

Validate rich/source documents and gallery text on clean macOS machines, including
the intended shipped font set and glyph coverage. Record startup cost on named
reference hardware rather than inferring it from TestPlatform. Linux physical
font/rendering qualification belongs to OCH-47; required Linux nongraphical checks
remain required. The broader theme-file example, native scrollbar preference
information and full milestone-07 acceptance are still open.
