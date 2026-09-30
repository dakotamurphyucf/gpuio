# Text shimmer

OCH-41, implementation in progress. `Text_shimmer` supplies validated configuration
and paired standalone OCaml/Rust codecs. The native glyph painter and retained
clock have independent GPU/frame-lifecycle evidence. They do **not** yet animate a mounted public view,
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

## Native painting

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
and account for simultaneous instances before release. The fixed-phase constructor
schedules no animation; the retained Owner below attaches a weak native driver.

## Native timing and ownership

`text_shimmer_clock::Owner` is one native node/window lifetime. A mounted adapter
owns its sole strong handle. Paint drivers and queued frame callbacks hold only
weak references; neither keeps a removed owner or its clock alive. A window can
share one native monotonic `Clock` among its owners. Timelines begin independently
on the first eligible paint, rather than following the upstream component's
globally synchronized loop phase.

| Change | Timing behavior |
| --- | --- |
| New owner/remount | Starts at phase zero on its first eligible paint. |
| Source, duration, direction or repetition | Restarts. |
| Highlight color, spread or external style | Preserves elapsed time. |
| `animated=false`, reduced motion, clipped or omitted content | Pauses; a later eligible paint resumes. |
| Completed Once | Remains complete through cosmetic or playback changes; source/timing changes or remount restart it. |
| Invalid configuration or source over 16,384 UTF-8 bytes | Rejects atomically, preserving old source/configuration/time. |

Construction of a rendered element advances prior running time and disarms its
owner **before layout**. Visible paint re-arms it only when the painter admits the
effect. This matters because a fully clipped element can skip paint entirely.
Empty/whitespace text, emoji-only paint, transparent highlights and painter-capacity
fallbacks do not request recurring frames. The host must explicitly suspend a
retained owner omitted by a conditional branch, and drop owners on unmount/close.
The future tree adapter must apply these rules to its actual keyed-node lifecycle.

There is at most one outstanding GPUI frame callback per owner. The callback
carries no source, configuration or phase; delivery checks the current owner and
reduced-motion state before notifying its native view. Source/config updates
invalidate old paint-driver stamps without accumulating replacement callbacks.
An already queued wake may drain after suspension/removal, but cannot restart an
ineligible or retired owner. Closing the window releases its remaining owner even
with a pending weak wake. No timer or frame callback crosses into OCaml.

Native time is monotonic and backward samples are clamped. Loop time is reduced
modulo the duration before float conversion; Once time saturates at completion.
The tests use a controlled native clock and explicitly deliver GPUI's frame
callbacks. They measure this effect's frame demand, not application FPS, real-time
latency, idle power or whole-application performance.

## Remaining live integration

These are implementation requirements, **not completed acceptance**:

- Decorate the ordinary text layout so enabling/disabling a title's progress state
  can preserve its native identity, selection and accessible source. It must not
  become a second text node, focus stop or live announcement.
- Preserve the independently tested painter's geometry/clipping when adding the
  mounted ordinary/selectable-text paths, foreground spans and subtree highlights.
  Its selection-background pixel test is not real keyboard/clipboard acceptance.
- Keep animation scheduling in Rust. No frame callback or timer crosses into
  OCaml. Bound work for long labels and many visible simultaneous instances;
  record the chosen admission policy before exposing a View constructor.
- Connect the tested native Owner to actual node generations/windows and verify
  conditional, managed-row and style-state visibility/opacity. The independent
  fixture does not establish retained-tree cleanup or whole-window frame demand.
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

These are fixed-phase painter tests, not public View, OS keyboard/IME,
screen-reader or Linux desktop acceptance. The source rows remain incomplete
until their applicable integration gates pass.

The subsequent retained-clock change passes seven combined shimmer unit tests
and strict native Clippy, using the same commands above. It also builds
`--test native_text_shimmer_clock --test native_text_shimmer_paint --no-run` with
`native-image-tests`. Both built executables pass under the 120-second deadline
wrapper, returning zero; the clock fixture prints
`GPUIO_NATIVE_TEXT_SHIMMER_CLOCK_OK` and `GPUIO_NATIVE_TEXT_SHIMMER_CLOSE_OK`.
Its background window checks visible GPU progress, one-shot final pixels/idle,
fifty renders coalesced into one pending wake, style/configuration reuse,
clipped/omitted/Reduce/static/transparent/empty/emoji/capacity idle paths,
Reduce arriving between paint and wake, independent owners, weak unmount,
fresh remount and owner disposal on window close. The fixed-phase painter suite
passes again after the clock integration. Neither test needs desktop focus.
