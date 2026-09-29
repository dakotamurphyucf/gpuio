# Ordinary text foreground spans

Status: ordinary text spans are implemented for OCH-41's pinned presentation-label
review. Typed values, atomic native updates, View/Bonsai reconciliation and ordinary/
selectable rendering are integrated. The enhanced label helper, matching/masking
configuration and public gallery acceptance remain required; this is not completed
label parity or milestone release acceptance.

The pinned upstream label has secondary text and prefix/all-occurrence foreground
highlighting within a single shaped text flow. GPUIO's subtree-highlight API paints
background matches. Building a row of differently colored text boxes would change
line breaking, shaping and selection. `View.styled_text` instead keeps one logical
string with bounded foreground ranges over its ordinary text layout.

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

## Mounted contract

Op59 `Set_styled_text` replaces source and foreground runs in one transaction.
It is valid only for ordinary Text nodes. The native tree independently validates
source/ranges/colors before committing, includes span payloads in retained-memory
budgets, and rolls back failed batches. `Set_text` clears runs, including when its
source is unchanged. Removal releases the runs; stale node generations cannot
receive new ones. Existing operation tags retain their values.

Capability bit 47 (`CAP_STYLED_TEXT`) is required by the paired OCaml client and
advertised by this Rust host. The shared mask is `281474976710655`. Older hosts
reject the client's handshake rather than receiving an unknown operation.
This capability covers ordinary foreground runs; it does not promise the pending
higher-level label matching/masking API.

`View.styled_text ?key ?style content` and `Gpuio_bonsai.View.styled_text` preserve
the ordinary Text kind and identity. Reconciliation caches resolved wire colors,
so a theme change updates runs even for a physically shared View. First mount
submits an empty Create followed by the atomic source/runs operation, avoiding a
duplicate source payload. No intermediate empty frame is published. Equal content
emits no update; plain/styled transitions add or clear runs even for identical text.
Failed or discarded preparation leaves acknowledged state unchanged.

Ordinary and selectable paths use one GPUI `StyledText` layout. Foreground runs
compose with the existing search background underlay and selection decoration.
They do not introduce separate inline layout boxes. Copy and default accessibility
retain the complete logical source. Color-only updates preserve selection and shaped
geometry; source updates follow the existing ordinary selection policy. Explicit
span colors override inherited foreground within their ranges; gaps inherit the
current element text style. This does not add styled editing or a document resource.

```ocaml
let open Core in
let open Gpuio in
let open Or_error.Let_syntax in
let%bind secondary =
  Text_content.Span.create
    ~start_byte:6 ~end_byte:11 ~foreground:(Color.rgb_exn 0x777777)
in
let%map content = Text_content.create ~spans:[ secondary ] "Hello world" in
View.styled_text ~style:(Style.create_exn [ User_select true ]) content
```

## Required label integration

1. Add the enhanced presentation-label helper without replacing the existing
   simple helper. Secondary text uses muted runs; prefix/all-occurrence matches
   use explicit accent runs. Map Unicode scalar lowercasing back to original byte
   ranges, merge overlapping matches and define precedence. Do not use transformed
   byte offsets as original source offsets. Matching bounds and case behavior must
   be documented and tested before this helper ships.
2. Mask before generating/submitting the native value. The masked value must
   contain only replacement text and its ranges, with source-dependent formatting
   suppressed; original text must not appear in AX, copy or highlight metadata.
   This is display masking, not a secure-memory/zeroization contract.
3. Validate the higher-level secondary/match configuration, theme changes and
   masked/unmasked transitions through a public gallery example with keyboard,
   selection/copy and AX evidence.
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
checks; the mounted evidence below covers the lower-level view separately.

Historical data-only checkpoint (1d3a8ba, macOS arm64, 2026-09-29): all four
OCaml expect tests and all three focused Rust tests pass. The complete `gpuio-protocol` suite passes 227
tests, and all-target protocol Clippy with `-D warnings`, Dune `@fmt`, Rustfmt,
catalog source checks and diff checks pass. No GUI was launched for this data
foundation. Mounted integration was still pending at that checkpoint.

```sh
./scripts/gpuio exec dune build -j2 @test/text_content/runtest
./scripts/gpuio exec cargo test -p gpuio-protocol --lib text_content --locked -j2
```

## Mounted validation

The paired Op59 transaction fixture extends the original independent content
fixture without changing older tags. Native tree tests cover exact retained-budget
admission/rejection, malformed-update rollback, kind restrictions, same-source
plain reset, removal and stale generations. Reconciler expect tests cover one
initial payload, stable identity, no-op reuse, changed theme tokens, plain/styled
and empty transitions, and discarded/failed preparation.

`native_styled_text` checks the actual retained renderer and GPU in a background
window. It compares mixed-script/wrapped source positions before and after
foreground runs (exact line positions and less than 1/1024 logical pixel horizontal
tolerance for floating-point advance summation), reads painted glyph colors,
queries macOS AppKit AX labels, and
dispatches select-all/copy keys through GPUI with the native clipboard. It verifies
same-source selection identity/range retention on recolor and plain reset, then
native teardown. The clipboard is restored after the run. This is not physical
OS keyboard injection, IME, VoiceOver navigation or Linux desktop acceptance.
`native_highlight_view` additionally paints explicit foreground runs together with
search underlays in both ordinary and selectable paths, preserving its existing
selection-precedence and lifecycle assertions.

The macOS CI checks compile and run the new test; Linux graphical smoke includes
it under the existing informational policy. Hosted and Linux execution remain
pending until the consolidated milestone validation. Full gallery, installed
consumer, performance and release gates remain separate.

Local mounted checkpoint (macOS arm64, 2026-09-29): the full Dune `@all @runtest
@fmt` check passes, including eight text-content expect tests. All 229 protocol
tests, two styled-text native tree tests and eight existing tree regressions pass.
The background `native_styled_text` and expanded `native_highlight_view` runs pass;
the former confirms foregrounds on Latin, accented and CJK glyphs as well as the
mixed-script source and selection checks above. All-target native/protocol Clippy
with `native-image-tests` and `-D warnings`, Rustfmt and catalog source checks pass.
The public enhanced-label example and milestone-wide release gates remain pending.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt
./scripts/gpuio exec cargo test -p gpuio-protocol --locked -j2
./scripts/gpuio exec cargo test -p gpuio-native --test styled_text --test tree --locked -j2
./scripts/gpuio exec cargo test -p gpuio-native --features native-image-tests \
  --test native_styled_text --test native_highlight_view --locked -j2 --no-run
# Run each resulting native GUI binary separately under a bounded process wrapper.
```
