# Checkable control appearance

OCH-41 implementation contract, 2026-10-01. Applies to checkbox, switch and
radio-group indicator presentation. This is in progress; a validated appearance
value alone does not establish that a public View renders it.

## Public shape

`Control_appearance.t` is immutable, validated and independent of selected value.
`create` accepts `size` (default 18 logical pixels), optional `switch_width`
(default size × 5/3), `gap` (default 8), `label_position` (`After`, default, or
`Before`), `indicator_style` and `mark_style` (default empty). Size is 8..128,
switch width is size..256, and gap is 0..128; all are finite. The switch width is
validated even for appearances currently used on checkboxes/radios, so reusing a
value cannot introduce invalid geometry. Checkbox and radio indicators are
size × size. The switch is switch_width × size. Text inherits the normal root
text style and is not scaled automatically.

The public consumers are optional `appearance` arguments on
`View.checkbox`, `View.switch` and `View.radio_group`, mirrored by Bonsai. Native
selection, callback generation, focus and identities are independent of this
value. Removing the argument resets default appearance without remounting.
Radio appearance affects each option's indicator/gap/label order; root direction
continues to control option arrangement. This does not affect tabs or popup
choices, which have their own presentation contracts.

For checkbox/switch, appearance gap is a component default: an explicit root
gap style may override it, preserving the existing styling contract. For radio
groups it is the spacing within each option, independent of root inter-option
spacing. Before/After specifies child order; an explicit reversed root flex
direction still has its usual layout effect. Indicator dimensions do not shrink
to make room for text; ancestor clipping/overflow policy remains authoritative.

## Paint and state rules

Styles reuse typed `Style.t`, with a deliberately bounded scope:

- Both parts accept Base, Checked, Indeterminate and Disabled. Merge in that
  order, using at most one of Checked/Indeterminate. Disabled is last and also
  reflects inherited disabled state. Inert suppresses input without inventing a
  disabled appearance. Focused, Hovered and Pressed remain root styles; part
  declarations for those states are rejected, not silently ignored.
- Indicator styles accept Background, Foreground, Opacity, border widths/color
  and corner radii. The default background is transparent; border is one pixel
  using inherited foreground. Default corner radii are size/6 for checkbox and
  size/2 for radio/switch. Corners clamp to the GPUI half-shortest-side rule.
- Mark styles accept Foreground and Opacity. Unspecified mark foreground inherits
  indicator foreground, which in turn inherits the root foreground. Indicator
  opacity affects the whole indicator, with mark opacity multiplied additionally.
  Ancestor opacity and clipping still apply through GPUI's native paint methods.
- The mark is the check/mixed dash, radio dot or switch thumb. Geometry scales
  proportionally with size. A switch thumb uses a size/6 inset and size × 2/3
  diameter, moving between the two track ends. Checkmark stroke scales from the
  existing 2-pixel stroke at size 18. No implicit animation or per-frame OCaml
  callback is introduced.
- At most 128 normalized declarations across both parts. Structural properties,
  cursor/input/accessibility changes, typography, shadows and nested state
  structures are rejected. Theme tokens resolve at submission; unresolved tokens
  fail before a transaction is sent. Native admission independently validates
  the same scope and finite geometry.

This closes explicit dimensions and selected/disabled part-color capabilities
identified in the pinned selection review. Rich labels and independent radio/Tab-order composition have separate
[labels](control-labels.md) and [navigation](checkable-navigation.md) contracts.
Their implementation and pending desktop evidence are tracked independently;
this appearance contract does not establish whole-family acceptance.

## Transport and validation

The record order is size, switch width, gap (three float64 values), label position
(Before tag 0 / After tag 1), indicator styles, then mark styles. Reuse the bounded
style field encoding. An independently specified fixture must be checked by both
languages. Test malformed/truncated data, invalid geometry, disallowed states and
properties, aggregate limits, theme resolution, and atomic reset/placement.

Op67 `Set_control_appearance (node, appearance option)` is restricted to
Checkbox, Switch and RadioGroup. Capability bit 60 advertises the paired
implementation; the experimental release requires the full current mask.
`None` resets appearance. Native admission validates the part scope independently
and accounts for retained style allocations. Reconciliation compares resolved
colors, so theme changes update presentation even if the abstract appearance is
unchanged. Semantic configuration and handlers remain separate. The independent
`control-appearance-operation.hex` fixture covers setting and resetting.

Required native evidence includes scaled marks at minimum/default/maximum sizes,
short/wide switch tracks, empty labels, both label positions, mixed and disabled
states, clipping/opacity, radio per-option disabled state, style reset, focus and
callback continuity, and cleanup on unmount/window close. Headless geometry and
scene assertions supplement real GPU/AX/public-gallery acceptance; they cannot
replace it. The gallery must show the differences through public APIs.

## Implementation checkpoint — 2026-10-01

The validated Core value, resolved wire record, standalone bounded Rust decoder,
native part-scope/refinement/accounting helpers and scalable mark geometry are
implemented. `control-appearance-default.hex` and `control-appearance-states.hex`
were specified independently and match both encoders. Three Core expect tests
cover fixtures, geometry/state/property rejection and re-resolution under a new
theme. Two protocol tests cover fixtures, truncation/trailing bytes, invalid
geometry/orientation and the aggregate declaration budget. Native checks cover
part scope, disabled/mixed precedence and containment at minimum/default/maximum
sizes, including minimum-width and wide switch tracks.

The full local OCaml test/format/gallery build passes. The native library passes
473 tests with two existing ignores; the two focused protocol tests and strict
native/protocol all-target Clippy also pass. These are local macOS builds and
headless tests. The initial OCaml build exposed a `Float` local-open name
collision in the bounds helper; distinct binding names corrected it without
changing the bounds contract.

The subsequent integration adds the optional Core/Bonsai View arguments,
resolved reconciliation, Op67/capability60, atomic native admission/accounting and
scalable native painting. The old fixed indicator helper is replaced; omitting
appearance keeps the 18-pixel defaults. A headless scene test passes scaled
radio/switch bounds, ancestor/part opacity multiplication, clipping and idle
unmount. Native transaction and protocol integration tests pass for set/reset,
invalid-update rollback, wrong-kind placement, retained handlers and accounting.
The first integrated OCaml build found an optional-argument order mismatch in
`radio_group`; its implementation was corrected to match the public interface.

Five Core expect tests and the full local OCaml test/format/gallery build pass.
The added expects check paired set/reset bytes and all three controls preserving
identity/current callbacks through theme updates, unchanged submissions, reset
and disabling. A production-View TestPlatform regression passes live indicator
size and label ordering, retained focus, inherited disabled paint/input blocking,
reset, released replaced configurations, unmount accounting and idle behavior.
Focus changes/removal schedule one normal redraw; the test drains it before
asserting the absence of ongoing wakes.

The Controls gallery now has a
public-API preview for custom/default appearance, 18/32-pixel sizes, label order,
mixed checkbox state, radio selection, disabled/inert input and live themes.
Its build passes; actual GPU/AX/public-gallery and independently installed-consumer
runtime validation remain open. This checkpoint does not close
the selection family or milestone.

Final local integration checks pass: full Rust workspace (without desktop-test
features), 475 native library tests with two existing ignores using
`native-image-tests,native-canvas-tests --lib`, strict native/protocol all-target
Clippy with those features, Rust formatting and the catalog checksum audit.
The workspace check exposed historical aggregate-capability assertions still
using 2^60−1 and treating bit 60 as unknown. They now independently expect 2^61−1,
accept the new bit, and continue rejecting unknown bit 61. No GUI fixtures ran.
No current-head hosted CI, publication or release acceptance is claimed.

## Native acceptance preparation — 2026-10-01

A fresh independent gallery builds against staged installed public packages via:

```sh
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery \
  --workspace scratch/control-appearance-consumer
```

Use a fresh workspace. This passed with `run=False`; it neither launched the
consumer nor establishes clean-machine or native desktop qualification.

`rust/native/src/control_appearance_native_test.rs`, invoked by
`native_image_views`, now contains physical GPU readback assertions for 21
size/value cases across checkbox, switch and radio: 8/18/128-pixel indicators,
checked/unchecked/mixed marks, both switch thumb endpoints, compounded opacity,
ancestor clipping, disabled colors, legacy reset, native owner retention and idle
cleanup. It also checks native macOS AX roles and retirement. Its pixel fixture
uses a small explicit caption font so radio label metrics cannot move the
indicator at the minimum size. Reused node slots advance generations.

The fixture links with:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native \
  --features native-image-tests,native-canvas-tests \
  --test native_image_views --no-run
```

Strict all-target native Clippy with those features also passes. **The fixture's
runtime assertions are unverified.** The new source is test-only; a linked binary
is not a GPU, AX or idle-performance result.

The public desktop walkthrough is:

```sh
python3 scripts/test_gallery.py --section control-appearance
# Alternatively, point --executable at the installed consumer's main.exe.
```

It is included in `all` and checks actual indicator-color bounds through theme,
size and label-order changes; native identity through appearance reset; mixed
values; actual Space/Return focus and callbacks; radio AX activation; inherited
Disabled, stale Inert actions and page retirement. Captures retry boundedly because
AX publication can precede physical presentation. Python compilation, CLI
discovery and offline coordinate/crop checks at 1×/2× pass. **Its desktop assertions
are unrun.** Offline synthetic pixels establish sampling behavior only.

The latest read-only preflight still returned no CoreGraphics window enumeration
and `AXIsProcessTrusted = false`. No app was launched for that preflight, and no
new GUI fixture was run. The prior black-window cause remains unresolved; these
preflight observations do not establish its cause. Actual repository and installed
consumer GPU/AX/input acceptance, wider selection gaps and OCH-17 gates remain open.
