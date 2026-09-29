# Finite grid and text style values — OCH-41

The value ledger now reviews seven sets: cursor, text overflow, gradient color
space, grid column minimum, grid row minimum, whitespace and text decoration.
This is a source/API audit with codec, validation and native refinement checks;
it does not establish every component root's layout, paint or input behavior.

## Pinned sources and mapping

The unchanged GPUIX revision is `18e695ed0ee8121a7793413ca795e08eda2a13df`.
The catalog snapshots now include `packages/native/src/style.rs` and
`packages/native/src/renderer.rs`, alongside the existing TypeScript declarations
and license. Their exact hashes are in `sources/manifest.json`. The native
`apply_styles` implementation explains behavior that a TypeScript union alone
cannot establish. No GPUIO dependency or runtime implementation changes here.

| GPUIX field and values | OCaml property and values | Contract |
| --- | --- | --- |
| `gridColumnMin`: `zero`, `min-content`, `max-content` | `Grid_column_minimum`: `Zero`, `Min_content`, `Max_content` | Minimum for repeated column tracks; pair with `Grid_columns`. |
| `gridRowMin`: same three values | `Grid_row_minimum`: same three values | Minimum for repeated row tracks; pair with `Grid_rows`. |
| `whiteSpace`: `normal`, `nowrap` | `White_space`: `Normal`, `No_wrap` | Explicit normal wrapping or no wrapping; normal can replace a prior no-wrap refinement. |
| `textDecoration`: `none`, `underline`, `line-through` | `Text_decoration`: `None`, `Underline`, `Strikethrough` | Explicit replacement of both decoration flags; GPUIO additionally offers `Underline_and_strikethrough`. |

GPUIX applies each grid minimum only inside the corresponding track-count branch;
it rounds/clamps that count to 1–64. GPUIO uses validated integer counts 1–1024.
A GPUIO minimum alone creates a one-track template; a subsequent count preserves
its minimum. Supply both the count and minimum for a direct GPUIX translation.
Changing one axis preserves the other axis and both existing counts.

The decoration mapping expresses the public visual intent, not identical helper
composition. GPUIX calls `underline`, `line_through` or `text_decoration_none` on
its element. GPUIO always writes both underline and strikethrough thicknesses:
`None` writes zero for both, `Underline` suppresses strikethrough, and
`Strikethrough` suppresses underline. This also makes suppression explicit over
inherited or component-provided decorations. Omitting/unsetting the property is
a separate operation and should not be substituted for `Text_decoration None`.
This audit does not assume that GPUIX's dependency helper has our replacement
semantics, or establish arbitrary nested document/control inheritance behavior.

## Validation scope

- `test/view_api/style_values_test.ml` fixes the twelve field encodings for these
  public choices, including the additional combined decoration.
- `rust/native/tests/style_values.rs` independently checks the same bytes and
  verifies all valid values can publish. Negative and upper-bound invalid values
  reject the entire transaction, preserving source text, revision and retained
  bytes; closing releases session resources.
- `rust/native/src/style.rs` tests all nine column/row minimum combinations,
  independent axis updates, count preservation, minimum-before-count ordering,
  decoration replacement over existing decorations and nowrap-to-normal reset.
  These exercise the production GPUI refinement function without a window.
- `scripts/audit_component_catalog.py` derives these finite keyword sets from the
  pinned declarations and requires exactly one mapping per keyword to a real
  public constructor. It checks snapshot hashes and reference existence; it is
  deliberately not a behavioral acceptance test.

Remaining value audits include the native-renderer aliases behind unrestricted
TypeScript strings, `alignContent: normal` and conditional reset semantics,
pointer occlusion versus event delivery, `userSelect: auto`, numeric policies and
specialized/deferred roots. OCH-41 and OCH-17 remain in progress. No additional
macOS GUI, Linux desktop, full gallery or release acceptance is claimed here.

Local macOS validation (2026-09-29): `./scripts/gpuio exec dune runtest
test/view_api -j2` passed; `cargo test -p gpuio-native --lib style::tests
--test style_values --features native-image-tests --locked -j2` passed all five
refinement tests. That name filter excludes the integration tests, so
`cargo test -p gpuio-native --test style_values --features native-image-tests
--locked -j2` was run separately and passed both tests. Cargo commands used
`./scripts/gpuio exec`. No application window was opened.
Strict native/protocol all-target Clippy with `native-image-tests` and
`-D warnings`, Rust formatting, OCaml formatting, catalog verification and
`git diff --check` also passed. Hosted CI and platform GUI acceptance were not
rerun for this test/documentation checkpoint.
