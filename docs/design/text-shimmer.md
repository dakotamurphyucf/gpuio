# Text shimmer

OCH-41, implementation in progress. `Text_shimmer` currently supplies validated
configuration and paired standalone OCaml/Rust codecs. It does **not** yet animate
a mounted view, add a live bridge operation, or advertise a capability. The
existing `Loading.Kind.Shimmer` remains a rectangular placeholder effect.

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

These are implementation requirements, **not completed acceptance**:

- Decorate the ordinary text layout so enabling/disabling a title's progress state
  can preserve its native identity, selection and accessible source. It must not
  become a second text node, focus stop or live announcement.
- Reuse shaped glyph positions and the glyph cache. The source uses twelve soft
  mask layers; paint must respect clipping, alignment, truncation, wrapping,
  bidirectional layout, inherited typography and selection precedence. Color
  emoji must retain their original paint. Do not reshape text on each frame.
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
