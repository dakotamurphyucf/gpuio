# Ordinary text foreground spans

Status: data and codec foundation under implementation for OCH-41's pinned
presentation-label review. No native opcode, mounted view, rendering or completed
label-parity claim is made at this checkpoint. Existing plain text remains
unchanged. The remaining integration below is required before acceptance.

The pinned upstream label has secondary text and prefix/all-occurrence foreground
highlighting within a single shaped text flow. GPUIO's ordinary text currently
has one inherited foreground; its separate subtree-highlight API paints background
matches. Building a row of differently colored text boxes would change line
breaking, shaping and selection. The intended equivalent keeps one logical string
and adds bounded foreground ranges to its ordinary text layout.

## Value and transport contract

`Text_content` is an immutable, validated value, not a document registration,
editor or resource owner. It contains at most 262144 valid UTF-8 bytes and 4096
foreground spans. `Text_content.Span.create` takes half-open byte offsets and a
`Color.t`; the containing value checks sorted, nonoverlapping ranges within the
source and at Unicode scalar boundaries. Adjacent spans and uncovered gaps are
valid. Empty content and an empty span list are valid. Gaps use the ordinary
inherited foreground. Scalar boundaries do not imply grapheme boundaries.

Colors may use theme tokens. `Expert.to_wire ~theme` resolves them at submission;
missing tokens are recoverable errors. A theme change must update resolved colors
without replacing the logical source or selection. The wire representation is
one string plus a list of `(start_byte, end_byte, RGBA)` records, each using
bin_prot int64 fields. RGBA is in 0..0xffffffff. Native validation independently
checks counts, ranges, order, source boundaries and colors before admission.
`Expert.of_wire` revalidates before creating a public value; a generated binary
reader is not a validator.

The standalone native decoder caps total input bytes and checks declared string
and list lengths before allocation. Exact source/span limits are accepted;
invalid UTF-8, truncated/trailing data, negative/oversized colors, invalid ranges
and over-limit declarations are rejected. There is no locale, timer, FFI callback,
font lookup, file operation or asynchronous work in these constructors.

## Required mounted integration

1. Append an atomic `Set_styled_text` operation without reusing old tags. Store
   retained spans only on ordinary Text nodes, account for their bounded memory,
   and validate updates atomically. Ordinary `Set_text` must clear old runs.
2. Add `View.styled_text` and the Bonsai alias. Reconciliation compares resolved
   text/run values, including theme changes, avoids duplicate source submissions,
   and removes spans when returning to plain text. Keep the existing `View.text`
   interface and behavior.
3. Render the source through a single GPUI `StyledText` with foreground ranges in
   both ordinary and selectable text paths. Background highlighting composes with
   the same layout; copy/accessibility use the complete logical string. Color-only
   updates preserve selection. Text changes follow existing selection policy.
4. Add the enhanced presentation-label helper without replacing the existing
   simple helper. Secondary text uses muted runs; prefix/all-occurrence matches
   use explicit accent runs. Map Unicode scalar lowercasing back to original byte
   ranges, merge overlapping matches and define precedence. Do not use transformed
   byte offsets as original source offsets. Matching bounds and case behavior must
   be documented and tested before this helper ships.
5. Mask before generating/submitting the native value. The masked value must
   contain only replacement text and its ranges, with source-dependent formatting
   suppressed; original text must not appear in AX, copy or highlight metadata.
   This is display masking, not a secure-memory/zeroization contract.
6. Validate paired transaction bytes, native atomic rejection/accounting, actual
   GPU foregrounds, line wrapping, mixed scripts, selection/copy, theme changes,
   AX semantics and masked/unmasked transitions through a public gallery example.
   Complete local checks and the milestone's consumer/hosted/release gates.

## Foundation evidence

The independent OCaml/Rust fixture is `Aé世界` with adjacent byte ranges 1..3 and
3..9, leaving the first byte inherited. It uses two concrete colors including an
unsigned 32-bit value needing the wider bin_prot integer encoding. Its fixed hex
is asserted separately in both languages; neither implementation generates the
other's expected fixture.

OCaml tests cover numeric/contextual range validation, scalar boundaries inside
joined emoji and combining sequences, overlaps/order, exact resource limits,
malformed wire admission and theme-token changes. Rust tests cover independent
bytes, all truncated fixture prefixes, trailing data, invalid UTF-8/colors/ranges,
exact limits and rejected oversized declarations. These are constructor/codec
checks; they do not establish native rendering or label acceptance.

Local macOS arm64 checkpoint (2026-09-29): all four OCaml expect tests and all
three focused Rust tests pass. The complete `gpuio-protocol` suite passes 227
tests, and all-target protocol Clippy with `-D warnings`, Dune `@fmt`, Rustfmt,
catalog source checks and diff checks pass. No GUI was launched for this data
foundation. Native mounting, GPU/selection/AX evidence and the public label
example remain unimplemented, as listed above.

```sh
./scripts/gpuio exec dune build -j2 @test/text_content/runtest
./scripts/gpuio exec cargo test -p gpuio-protocol --lib text_content --locked -j2
```
