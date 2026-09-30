# Text shimmer

OCH-41, implementation in progress. `Text_shimmer` supplies validated configuration
and paired standalone OCaml/Rust codecs. A native glyph painter now has independent
GPU evidence at fixed phases. It does **not** yet animate a mounted public view,
add a live bridge operation, or advertise a capability. The existing
`Loading.Kind.Shimmer` remains a rectangular placeholder effect.

The pinned [component source](../catalog/sources/component-shimmer.rs.txt) is
gpui-kit `84f57fdfcb4910623fb0bb7f795b077e249f9271`. The snapshot is verified against
the pinned archive and recorded in the catalog manifest under the existing
gpui-kit license. Attachment progress titles and typed Marker text need this
shared glyph effect. They are not functionally equivalent merely because the
placeholder effect exists.

## Configuration contract

`Text_shimmer.Config.create` is a pure constructor. No timer, callback or upload
task is created. `Spread` is abstract and constructed through validated functions;
physical direction and repetition are closed variants.

| Property | Contract |
| --- | --- |
| Duration | One sweep, default 2 seconds; 1ms through 60s, rounded up to whole milliseconds. |
| Relative spread | Band half-width as a fraction of text bounds, 0.05 through 1; default 0.3. |
| Pixel spread | Band half-width in logical pixels, 1 through 1,000,000, matching the general length ceiling. |
| Direction | Left-to-right or right-to-left, independent of writing direction. |
| Repetition | Once or Loop; default Loop. |
| Animated | Defaults true. False requests static ordinary text. System reduced motion must also suppress animation. |
| Highlight | Optional RGBA override. An explicit color token resolves at construction; recreate it when the application theme changes. Alpha is retained. Omission requires no token and requests the native inherited-text/theme default. |

Invalid values return errors; they are not silently clamped. This intentionally
differs from source builders that clamp spread and turn zero duration into 1ms.
The duration ceiling follows our native loading configuration's bounded transport
policy. Color, spread and duration are validated again on admission in both
languages. Configurations contain no source text, resource handles or collections.

The standalone encoding is duration (bin_prot integer), tagged spread (float64),
direction, repetition, animated Boolean and optional RGBA integer, in that order.
Both decoders reject malformed tags, nonfinite values, invalid ranges, truncation
and trailing bytes. Input is bounded to 32 bytes before parsing. Public domain
types do not expose generated deserialization as a validation bypass.

## Native integration requirements

The independent painter wraps `StyledText`, delegates its layout/prepaint/base
paint, then paints at most twelve clipped monochrome glyph layers. It introduces
no text copy, hit target, focus owner or callback into OCaml. Shaped visual
positions drive placement, including RTL runs and soft wraps; color emoji keep
their original paint. The adapter does not reshape text for each layer.

The default color follows the pinned source's actual premultiplied Oklab policy:
20% inherited text color and 80% theme foreground (dark) or background (light).
The combined twelve-layer peak opacity is 0.6 in dark mode and 0.75 in light mode;
explicit RGBA highlights preserve alpha before this layer weighting. Oklab math
uses double precision, with clamping only at the final sRGB conversion. The source
color helper was verified against the pinned archive (SHA-256
`d3c81fb0fdca3fce797c312addc5afd9767d5985dc56bbf73afecdd0019c4e01`).

Painter admission is all-or-static: at most 16,384 laid-out UTF-8 bytes, 256 logical
lines and 4,096 shaped glyphs. Exceeding any limit reports Capacity and leaves the
whole original text unchanged; it never paints a highlighted prefix. These limits
bound the adapter's work, not the underlying ordinary-text layout or application
memory. The future public View constructor/bridge must expose validated admission
and account for simultaneous instances before release. No animation is scheduled
by this paint-only adapter.

Remaining implementation requirements, **not completed acceptance**:

- Decorate the ordinary text layout so enabling/disabling a title's progress state
  can preserve its native identity, selection and accessible source. It must not
  become a second text node, focus stop or live announcement.
- Preserve the independently tested painter's geometry/clipping when adding the
  mounted ordinary/selectable-text paths, foreground spans and subtree highlights.
  Its selection-background pixel test is not real keyboard/clipboard acceptance.
- Keep animation scheduling in Rust. No frame callback or timer crosses into
  OCaml. Bound work for long labels and many visible simultaneous instances;
  record the chosen admission policy before exposing a View constructor.
- Use node generation/window ownership for animation state. Source/timing changes
  and remounts must have explicit restart semantics; cosmetic updates must not
  restart accidentally. One-shot completion must become idle. Hidden, clipped,
  removed and reduced-motion text must not sustain frame requests.
- Add checked transport, Core/Bonsai reconciliation and atomic rejection before
  advertising the feature. Preserve source identity when clearing the effect.
- Test actual GPU glyph paint (not only a changing animation phase), RTL/wrapping/
  ellipsis/emoji, selection and AX source, Full/Reduce transitions, one-shot idle,
  hidden/unmount/window-close cleanup, and independent instances. Then add public
  Attachment/Marker examples and an installed-library consumer.

## Current local evidence

The Core expect suite and Rust protocol suite share two independently constructed
byte fixtures covering all variants and optional color. Both enumerate 576 valid
boundary combinations and reject malformed configuration, every fixture
truncation, trailing data and invalid tags. Core also checks sub-millisecond
rounding and token resolution with preserved alpha. These tests establish the
configuration/codec contract only; native and release evidence remain pending.

Local macOS 14.5 arm64 commands, through the repository's isolated environment:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @all @runtest @fmt
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -p gpuio-protocol -j 2 --test text_shimmer
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -p gpuio-protocol -j 2 --all-targets -- -D warnings
python3 scripts/audit_component_catalog.py
git diff --check
```

All passed. No GUI window was opened for this configuration-only change. Required
Linux checks, mounted native acceptance and whole-release CI remain separate.

The subsequent native painter change passes these local commands:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -p gpuio-native -j 2 --lib text_shimmer
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -p gpuio-native -j 2 --lib --features native-image-tests -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -p gpuio-native -j 2 --features native-image-tests --test native_text_shimmer_paint --no-run
```

The built native executable ran under a 120-second process-group deadline wrapper;
no timeout occurred. For an ordinary rerun, omit `--no-run` from the last command.
The fixture opens one background macOS window and closes it on success or caught
failure. Final local execution returned zero and
`GPUIO_NATIVE_TEXT_SHIMMER_PAINT_OK`. It covers 24 Latin/Hebrew/Arabic/mixed-direction
width/alignment cases (narrow cases assert real wrapping), both ellipsis modes,
ancestor clipping, reversed physical sweep, static/reduced/transparent output,
empty/emoji-only text, all three capacity bounds, unchanged selection-background
pixels, and both default theme colors. Comparisons use actual GPU readback:
highlighted pixels must remain on the original glyphs, with a one-device-pixel
tolerance for color-dependent raster dilation. Text geometry/source must remain
identical; static and emoji-only images must be byte-identical. Paint calls are
bounded by twelve times the admitted glyph count.

These are fixed-phase painter tests, not continuous animation, scheduler/idle,
public View, OS keyboard/IME, screen-reader or Linux desktop acceptance. The
source rows remain incomplete until those applicable integration gates pass.
