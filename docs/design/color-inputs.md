# Color selection (OCH-36)

Status: value foundation, Core control contracts, paired standalone codecs and
pure native interaction policy implemented; mounted controls, retained bridge,
runtime controllers, popup and public example remain in progress. No color-control
capability is advertised yet.

## Concrete values

`Gpuio.Color_value.Rgba.t` is four validated bytes in red/green/blue/alpha order.
RGB uses encoded sRGB and alpha is straight, not premultiplied. It converts
losslessly to a concrete `Gpuio.Color.t`. Theme references remain the separate
`Color.token` concept; selecting a color never resolves an ambient theme token.
`Value.Empty` differs from transparent black and every other concrete color.

`Hsla.t` validates finite hue in 0–360 degrees and finite saturation, lightness
and alpha in 0–1. Hue 360 becomes zero; signed zero becomes positive zero.
Out-of-range caller input is rejected. HSL arithmetic uses encoded sRGB, not a
linear-light color space. This is not a wide-gamut or HDR selection API.
HSLA-to-byte conversion uses `floor(channel * 255 + 0.5)`, after bounding internal
conversion roundoff. Alpha 0.5 therefore becomes 128, not 127. Float-to-byte
quantization error is bounded by 0.5/255 plus floating-point arithmetic error.
RGBA-to-HSLA uses zero hue for achromatic colors. Byte colors round-trip through
HSLA without a change in the sampled reference tests; no preservation of an
arbitrary original HSLA representation is promised after byte quantization.

Hex parsing accepts exactly 3, 4, 6 or 8 ASCII hex digits with one optional leading
`#`. Short digits expand by multiplication by 17. Whitespace, signs, Unicode
lookalikes and other CSS syntaxes are rejected. Formatting produces uppercase
`#RRGGBB` when alpha is 255, otherwise `#RRGGBBAA`. Parsing/classification examines
at most nine bytes; longer text is rejected before scanning or allocation.
`Hex_draft.parse` classifies empty text, incomplete text (`#` or 1/2/5/7 digits),
valid colors and invalid text. Classification does not rewrite input. In
particular, an empty edit is not an implicit successful Clear command.

Pure Rust counterparts live in `gpuio_protocol::color_value`; neither side owns
a native widget or schedules I/O. The wire representation is described separately
below; public value types do not expose bin_prot readers.

## Core interface and wire boundary

`Gpuio.Color_input` now exposes validated `Labels`, `Palette_entry` and `Config`,
abstract revisions and interaction identities, immutable native snapshots,
commands and typed observations/errors. `Labels.english ~control` supplies the
default English channel labels; `Labels.create` accepts explicit localized names.
For example, this constructs a description without allocating native resources:

```ocaml
let config =
  let open Core.Or_error.Let_syntax in
  let%bind labels = Gpuio.Color_input.Labels.english ~control:"Accent color" in
  let%bind color = Gpuio.Color_value.Rgba.of_hex "#7C5CFC" in
  let%bind entry = Gpuio.Color_input.Palette_entry.create ~color ~label:"Violet" in
  Gpuio.Color_input.Config.create ~labels ~palette:[ entry ] ()
```

Snapshots expose the current and committed concrete values, unquantized editing
channels, optional interaction/draft, and value-validity flags. Expert imports
validate before constructing public snapshots. Their window/node identities stay
separate from revisions and interaction IDs. `View.color_input` describes a retained native leaf with a stable controller key,
a configuration, an initial seed and typed observations. Native rendering and the
Bonsai/Eio runtime controller are still being integrated.

`Color_input_wire` and Rust's bounded `decode_color_*` functions encode the same
ordered contracts. Concrete RGBA is a nonnegative bin_prot int64 in
`0x00000000..0xFFFFFFFF`, with bit layout `0xRRGGBBAA`. Value tags distinguish Empty
from Color. HSLA carries four binary64 values in hue/saturation/lightness/alpha
order. Configuration carries seven labels, bounded palette entries, alpha policy,
allow-empty, disabled and read-only. Snapshots carry revision, value, committed,
channels, interaction, draft and the two validity flags, in that order.

Standalone configuration payloads are bounded to 98,304 bytes; events/responses
to 4,352 bytes; commands to 32 bytes. The largest valid configuration is 97,308
bytes. Rust decoding bounds each length/count before allocation and requires
full payload consumption. Semantic validation rejects invalid UTF-8, packed
colors, nonfinite/out-of-range channels, inconsistent channel/RGBA values,
future/nonpositive interaction IDs, impossible draft/interaction shapes and
draft classifications inconsistent with their grammar. Started events carry
the starting revision as their interaction ID. Forbidden draft classification
requires a syntactically valid candidate with nonopaque alpha; whether the
current configuration forbids it remains the native owner's responsibility.

OCaml generated wire readers are internal data readers, followed by semantic
validation when importing public values. They are not advertised as standalone
untrusted-input entry points. The retained bridge appends kind 41, operation 47 (`Set_color_input`) and event
52 (`Color_input_event`). Correlated command/response envelopes remain pending.
Dependency pins and the advertised capability set remain unchanged.

## Interaction policy

`rust/native/src/color_input_state.rs` owns committed and preview selections,
their HSLA representations, one optional interaction and one optional text draft.
The corresponding Rust contract lives in `color_input.rs`. Core descriptions and
standalone codecs share these contracts; runtime controllers and native widget
integration are still in progress.

Channel input uses degrees for Hue (0–360) and percentages for Saturation,
Lightness and Alpha (0–100). Text uses the existing numeric draft grammar,
including decimal/exponent notation; values are validated without snapping to
integer slider steps. Hex input uses the strict grammar above. Draft storage is
bounded to 4,096 UTF-8 bytes and excludes NUL/newline/carriage return. Oversized
or structurally rejected drafts do not change the policy owner. Valid hex input
that becomes achromatic preserves the previous editing hue. An explicit Hue
channel edit changes that remembered hue even while the color is gray.

Each successful interaction start receives its positive starting revision as an
identity. Preview/finish callbacks must carry that identity; callbacks from a
cancelled interaction fail even after a new interaction starts. A preview can
change HSLA or raw draft text without changing rounded RGBA bytes. It is an
observation of editing state, not necessarily a valid candidate for commitment.
While text is being composed, the last valid color preview is preserved and
finish reports Composing. Invalid, incomplete or empty text cannot commit. Empty
text does not invoke Clear. Cancellation restores the full committed HSLA/value
baseline and removes the active draft; mounted children must synchronize to that
restored baseline without leaking old callbacks.

Set and Reset check their optional revision guard and new value before cancelling
an active interaction. Set installs an explicit concrete value; Reset restores
the original mounted seed under the current policy. They remain permitted while
disabled/read-only, emit Cancelled followed by Observed if needed, and never
impersonate a user commit. A Reset whose original seed is no longer allowed
fails without changing an active edit. Discrete keyboard/AX/palette/Clear input
interrupts an active edit and applies from the committed baseline, emitting
Cancelled then Committed. Completing a pointer or text interaction emits its
own Committed observation. These inline commits remain separate from Apply in
a future composed popup.

Configuration has seven explicit localizable labels, each nonblank and at most
4,096 bytes. A palette has at most 256 entries with concrete RGBA and nonblank
labels of at most 256 bytes. NUL labels are rejected. Configurations account for
actual retained String/Vec capacity. Palette entries can become incompatible
with alpha policy; the mounted adapter must disable those choices, while the
policy owner rejects an attempt to choose them. Label/palette changes preserve
an active edit. Alpha/empty-policy changes and newly disabled/read-only state
cancel it before publishing the new configuration observation.

Opaque-only input requires alpha exactly 1 during HSLA editing; values slightly
below 1 are not accepted merely because byte quantization would round to 255.
Historical nonopaque or Empty values survive a restrictive policy change and
expose `value_allowed`/`committed_allowed` flags. No implicit alpha coercion is
performed. Explicit Set/Reset must satisfy the current policy.

Each operation emits at most two events and reserves all needed revisions before
mutation. Revision exhaustion leaves both state and configuration unchanged.
The owner contains no queue or timer. The mounted adapter must atomically admit
each event batch, coalesce previews only within the same owner/interaction, and
fault the owner if required output is lost. Faulted and closed owners reject
further mutations. Visibility/modal/window activation and native leases are
adapter gates; explicit cancellation remains available when input is blocked.
Actual editor/capture/focus/resource cleanup is not proven by these pure tests.

## Native adapter direction

The evaluated base source is `vendor/gpui-base/src/color_picker.rs`, upstream
`84f57fdfcb4910623fb0bb7f795b077e249f9271`. Its `ColorPickerState` combines slider
Change and Release into one Change event and truncates byte channels in hex
formatting. Its preview reset does not restore every child slider. GPUIO therefore
needs a native policy owner with explicit preview, commit and cancel boundaries.
Base sliders do distinguish Change from Release. Further source inspection shows
that their AX Increment/Decrement handlers call `set_value`, which notifies the
entity without emitting a SliderEvent, and their private drag flag has no public
cancel method. Direct event subscription alone therefore cannot satisfy the
color owner contract. GPUIO's existing slider adapter already implements capture
loss, keyboard/AX input and gate handling. The initial color adapter follows
those capture/input gates directly over the color policy. Reusing the standalone slider model would introduce a second,
step-quantized channel value, so color channels retain their unquantized HSLA
values in one owner. No public slider API change or upstream patch has been made.
Base ColorSwatch provides controlled radio/toggled presentation, but owns a
private keyed focus handle. GPUIO uses the corresponding GPUI radio/toggled
primitives with explicit retained handles so palette parts join its modal and
managed-list focus registry. Mounted channel/palette keyboard and pointer checks
now pass; full OS accessibility and text-editor acceptance remain pending.

The planned owner retains native channel editors/sliders, hue memory, draft,
baseline and revision. Preserve editing hue when a gesture becomes achromatic;
explicit RGBA replacement derives deterministic hue from that value. Native
preview updates and paint must not depend on a Bonsai round trip. Preview events
may coalesce within an interaction; commits/cancellations remain discrete
ordering boundaries. Cancel restores the complete baseline, and Set/Reset fences
late callbacks from the old interaction. Existing editor, focus, overlay and
ownership adapters should be reused. The native adapter now renders channels
and palette/clear controls directly from that owner. Text editors and public runtime controllers are next.

Mounted lifecycle behavior and public runtime controllers remain to be
implemented and validated. An eyedropper remains capability-specific
follow-on work unless a portable implementation is verified.

See [local foundation evidence](../evidence/color-inputs-och36.md). Full ticket
acceptance includes mounted native input/AX/GPU/lifetime tests, public API usage,
Linux builds/unit tests and required hosted checks; this document does not claim
those have passed.

## Retained ownership and event admission

The controller key determines retained identity. Callback-only updates and changes
to the initial seed do not reset an existing owner. Configuration updates retain
the original seed, including when it becomes disallowed by the new policy. A new
identity requires an initially allowed value; a native owner created after several
configuration operations in the same transaction preserves the tree-admitted seed.
Removing and recreating the identity creates a new seed and event generation.

The native tree accounts for retained configuration storage and enforces leaf
shape and handler presence. Core dispatch fences window/node/handler identity,
accepted tree revisions, monotonically increasing observation revisions and
validated snapshots. An invalid future observation cannot advance that counter.

Color event batches contain one observation or an allowed two-event transition:
Started/Preview, Preview/Committed, or Cancelled followed by Observed/Committed.
Pairs require the same route and consecutive revisions. Preview completion also
requires a matching candidate and pointer/text source; text must be valid and
not composing. Admission checks both queue count and bytes before replacing any
previous preview. On failure the queue remains unchanged. Coalescing is limited
to adjacent previews in the same route, interaction and committed baseline;
commit, cancel, command-result and render events remain ordering barriers.

## Initial mounted channels and palette

`color_input_view.rs` retains one native policy and explicit focus handles for four
channels, up to 256 palette entries and Clear. `color_input_channels.rs` handles
continuous pointer fractions without integer-step quantization. Arrow and AX
increment/decrement actions use one degree/percentage point; Page Up/Down use ten.
Home/End select channel endpoints (hue 360 canonically wraps to zero). The ramps
use at most six native gradient segments; repaint never waits for OCaml.

Pointer start and final preview/commit observations use atomic color batches.
Capture is retained across stable paints, released on cancel/configuration changes
and denied while disabled/read-only or outside the native input gate. Restricted
alpha controls and incompatible palette entries are disabled. Palette/clear
activation goes through GPUI's native click/keyboard routing. Configuration can
retire focus handles and clears focus from newly unavailable parts.

Host hooks cover Escape, visibility/modal exclusion, window deactivation, native
close and managed-list retention. These hooks are implemented; only the scenarios
listed in the evidence ledger are currently validated. Native form metadata is
admitted without overriding the control's role/actions. Internal focus-part IDs
are u16, allowing all 256 palette entries plus other controls without aliasing.

The initial mounted test covers channels and palette. Hex/channel text editing,
correlated runtime commands, popup composition, public example, comprehensive
AX/IME/lifecycle/workload validation and capability advertisement remain pending.
