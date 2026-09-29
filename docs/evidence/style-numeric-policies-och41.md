# Numeric, shorthand and remaining native style policies — OCH-41

This completes the next source-contract audit slice against GPUIX
`18e695ed0ee8121a7793413ca795e08eda2a13df`. It records deliberate typed API
choices, validates admission boundaries, and adds five native keyword sets to
the nine in the [alias audit](style-native-aliases-och41.md). It does not complete
specialized-root layout/input/paint behavior or release acceptance.

## Numeric contracts

| GPUIO input | Accepted domain | Pinned GPUIX difference |
| --- | --- | --- |
| `Length.px` / `percent` | Finite, absolute value ≤ 1,000,000; percent uses percentage points, e.g. 200 means twice the parent | GPUIX parses numbers, numeric strings, percentages and auto. GPUIO uses typed constructors and no CSS-string parser. |
| Width/height/min/max/basis | Nonnegative lengths or Auto | GPUIX width/height percentages ≥ 99.9% become full size; GPUIO preserves 200% as 200%. Its width/height Auto helper is a no-op; min/max Auto is also ignored. GPUIO can explicitly write native Auto. GPUIX basis takes pixels, while GPUIO also accepts percent/Auto. |
| Padding, gap, line height | Nonnegative definite lengths; Auto rejected | GPUIX padding/gaps take raw pixel numbers; line height only applies when > 0. GPUIO accepts zero line height and percentage leading as explicit values. |
| Margins and offsets | Signed lengths or Auto | GPUIX takes pixel numbers. GPUIO adds typed percentages/Auto. |
| Grow/shrink, border widths, corner radii | Finite 0..1,000,000 | GPUIX passes grow/shrink/radii to GPUI and clamps negative borders to zero. GPUIO rejects invalid values rather than silently correcting them. |
| Font size | Finite > 0, ≤ 1,000,000 logical pixels | GPUIX forwards the supplied number; GPUIO rejects zero/negative/nonfinite/oversized values. |
| Opacity | Finite 0..1 | GPUIO validates the domain before transport/admission rather than relying on a render helper. |
| Grid rows/columns | Integer 1..1024 | GPUIX rounds/clamps to 1..64. Grid minima require the count in GPUIX; see the finite-value audit. |
| Line clamp | Integer 1..1024 | GPUIX ignores values < 1 and casts the remainder to an integer. |
| Font weight | Integer 1..1000 | GPUIX accepts numbers, numeric strings and names, clamps to 1..1000 and defaults unrecognized strings to 400. GPUIO requires the validated integer. |
| Font family | Nonempty, at most 256 bytes | GPUIX passes the string through. The GPUIO limit counts bytes, not Unicode characters. |
| Shadows | At most eight; dimensions finite with magnitude ≤ 1,000,000; blur ≥ 0; signed offsets/spread; explicit inset flag | GPUIX exposes one shadow, clamps blur and spread to nonnegative values and has no inset member in its public shadow object. GPUIO supports a list, negative spread and inset explicitly. |

The named font-weight translations are: thin=100, extralight/extra-light=200,
light=300, normal=400, medium=500, semibold/semi-bold=600, bold=700,
extrabold/extra-bold=800 and black=900. GPUIO callers supply those integers;
there is no need to duplicate permissive string parsing in the FFI.

Typed `Color`, theme tokens and validated two-stop `Background` replace raw CSS
color/gradient strings. Gradient angles are finite 0..360, stops ordered in 0..1;
[gradient evidence](gradient-color-spaces-och41.md) covers encoding and focused
GPU behavior. Unresolved theme tokens do not become arbitrary native strings.
Additional bridge limits include accessible names of 1..1024 bytes and at most
128 input declarations per style-construction call. Shadow, length, style and
background public interfaces now document these numeric bounds.

## Ordered shorthands

GPUIX applies shorthand first and then a supplied side/axis longhand, regardless
of JavaScript object property order. This covers gaps, padding, margins, border
widths, corner radii and overflow. GPUIO styles are ordered declaration lists:
shorthands expand at their position, and the last assignment to each side/axis
wins. For direct translation, put the shorthand before its explicit overrides.

For example `[Padding 8px; Padding_left 20px]` leaves left padding at 20;
reversing the list makes it 8. `[Overflow Scroll; Overflow_x Hidden]` scrolls only
Y, while reversing the list enables both axes. Subsequent `Style.merge` inputs
also override earlier inputs. This preserves an intentional, composable OCaml
API rather than depending on unordered record/object semantics. GPUIX also
prioritizes `backgroundColor` over `background`; translate the former to a
`Background.solid` declaration placed last. Invalid CSS colors are ignored by
GPUIX; GPUIO colors validate explicitly and missing theme names return an error.

Hover and pressed layers accept layout as well as visual properties. The six
base-only declarations are pointer occlusion, pointer eligibility, user selection,
selection color, accessible name and inert policy. The older ledger
incorrectly described all structural fields as rejected; that text is corrected
and the public scope rule is tested. Widget-specific appearance scopes can be
narrower and still require component-by-component behavior review.

## Display, visibility and overflow

The pinned native `apply_styles` explicitly handles display `flex`/`grid` and
visibility `hidden`; it does not turn arbitrary string values into supported
native choices. GPUIO additionally offers `Display.Block`, `Display.Hidden` and
`Visibility.Visible` as typed choices. This audit of native branches is not a
claim that every upstream React-level behavior has been exercised.

Overflow `hidden` applies through GPUI's Styled helper; `scroll` is applied by
the stateful host, resolving axis longhands before the shorthand. GPUIO's public
`Visible` and `Clip` are additional explicit choices. The pinned GPUIX horizontal
host also forces flex/min-width-zero and wraps its child row; GPUIO callers
express layout intentionally through `View.row`/`column` and size/min-size styles.

Two-axis diagonal scrolling now passes the real-window
[native scroll regression](scrolling-och11.md#milestone-07-parity-two-axis-containers),
including nested boundaries, hover-axis replacement and disposal. This does not
establish identical overflow behavior inside native editors, lists, overlays or
all specialized roots. Display/visibility lifecycle and GPU evidence remain in
the [highlighting evidence](subtree-highlighting-och41.md) and
[selection audit](../design/selection-style-audit.md), with their stated limits.

## Verification scope

`style_validation_test.ml` checks public float/count/sign/Auto/magnitude domains,
UTF-8 byte counts, shadow limits and both shorthand orders. Independent native
`style_validation.rs` checks the corresponding admission boundaries and rejects
invalid transactions without changing source text, style, revision or retained
bytes. Every fixture closes its session and verifies zero retained bytes.
A native refinement test confirms 200 percentage points become a relative width
of 2.0, and a 25-point gap becomes 0.25.

The native keyword ledger now covers fourteen fields, plus the separate seven
finite declaration sets. Its verifier derives explicit native branches rather
than treating arbitrary TypeScript strings as accepted CSS. The entire pinned
native Rust source tree (31 files, verified Git blobs) was searched locally to
locate display/visibility handling; the relevant renderer/style/helper snapshots
are versioned. These structural and validation checks do not substitute for
specialized-root behavior, complete gallery/consumer runs or OCH-17 release gates.

Local macOS validation on 2026-09-29 used `./scripts/gpuio exec`:

- `dune runtest test/view_api -j2`: passed, including count limits and state scope.
- `cargo test -p gpuio-native --test style_validation --features native-image-tests
  --locked -j2`: all three boundary/atomicity tests passed.
- `cargo test -p gpuio-native --lib style::tests --features native-image-tests
  --locked -j2`: all eight refinement tests passed.
- Strict native/protocol all-target Clippy with the same features, `--locked -j2
  -- -D warnings`, both format checks, catalog verification and diff check passed.

No production behavior, dependency or fork changed in this checkpoint. No GUI
window, hosted CI or release acceptance is claimed from these tests.
