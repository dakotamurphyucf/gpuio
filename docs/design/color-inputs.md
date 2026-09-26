# Color selection (OCH-36)

Status: value foundation implemented; native controls, bridge, popup and public
example remain in progress. No color-control capability is advertised yet.

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

## Native implementation direction

The evaluated base source is `vendor/gpui-base/src/color_picker.rs`, upstream
`84f57fdfcb4910623fb0bb7f795b077e249f9271`. Its `ColorPickerState` combines slider
Change and Release into one Change event and truncates byte channels in hex
formatting. Its preview reset does not restore every child slider. GPUIO therefore
needs a native policy owner with explicit preview, commit and cancel boundaries.
Base sliders do distinguish Change from Release. Base ColorSwatch provides a
useful controlled radio/toggled presentation and native focus; actual keyboard
and accessibility acceptance is still required.

The planned owner retains native channel editors/sliders, hue memory, draft,
baseline and revision. Preserve editing hue when a gesture becomes achromatic;
explicit RGBA replacement derives deterministic hue from that value. Native
preview updates and paint must not depend on a Bonsai round trip. Preview events
may coalesce within an interaction; commits/cancellations remain discrete
ordering boundaries. Cancel restores the complete baseline, and Set/Reset fences
late callbacks from the old interaction. Existing editor, focus, overlay and
ownership adapters should be reused. A mounted adapter experiment will select
the smallest safe integration boundary before exposing control interfaces.

Opaque-only policy must reject nonopaque explicit replacements and preserve
historical values visibly when policy changes, rather than silently stripping
alpha. Final configuration, palette quotas, command/event types and native
lifecycle semantics are still to be implemented and validated. Inline gesture
commit and application confirmation of a popup draft are distinct operations.
An eyedropper remains capability-specific follow-on work unless a portable
implementation is verified.

See [local foundation evidence](../evidence/color-inputs-och36.md). Full ticket
acceptance includes mounted native input/AX/GPU/lifetime tests, public API usage,
Linux builds/unit tests and required hosted checks; this document does not claim
those have passed.
