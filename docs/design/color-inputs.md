# Color selection (OCH-36)

Status: value foundation and pure native interaction policy implemented; mounted
controls, bridge, popup and public example remain in progress. No color-control
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
a native widget or schedules I/O. Wire codecs and protocol tags will be added
with the bounded color-control contract; these domain types are not an implicit
serialization format.

## Interaction policy

`rust/native/src/color_input_state.rs` owns committed and preview selections,
their HSLA representations, one optional interaction and one optional text draft.
The corresponding Rust contract lives in `color_input.rs`. Public OCaml control
interfaces and wire decoding are still being integrated; existing `Color_value`
interfaces are available independently.

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
loss, keyboard/AX input and gate handling; a private reusable presentation/input
boundary is being evaluated. No public slider API change or upstream patch has
been made. Base ColorSwatch provides a useful controlled radio/toggled
presentation and native focus; actual keyboard and accessibility acceptance is
still required. These are source findings, not mounted color-control acceptance.

The planned owner retains native channel editors/sliders, hue memory, draft,
baseline and revision. Preserve editing hue when a gesture becomes achromatic;
explicit RGBA replacement derives deterministic hue from that value. Native
preview updates and paint must not depend on a Bonsai round trip. Preview events
may coalesce within an interaction; commits/cancellations remain discrete
ordering boundaries. Cancel restores the complete baseline, and Set/Reset fences
late callbacks from the old interaction. Existing editor, focus, overlay and
ownership adapters should be reused. A mounted adapter experiment will select
the smallest safe integration boundary before exposing control interfaces.

Mounted lifecycle behavior, paired wire decoding and public controllers remain
to be implemented and validated. An eyedropper remains capability-specific
follow-on work unless a portable implementation is verified.

See [local foundation evidence](../evidence/color-inputs-och36.md). Full ticket
acceptance includes mounted native input/AX/GPU/lifetime tests, public API usage,
Linux builds/unit tests and required hosted checks; this document does not claim
those have passed.
