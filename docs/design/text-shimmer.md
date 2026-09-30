# Text shimmer

OCH-41, implementation in progress. `Text_shimmer` now connects typed Core/Bonsai
text descriptions, atomic live transport and the native retained Host to one
shaped glyph layout. Plain/selectable text, foreground spans and search underlays
share that layout. A background mounted fixture verifies GPU output, logical
selection/copy and native source labels. Capability advertisement, public gallery
and installed-consumer acceptance remain open. `Loading.Kind.Shimmer` remains a
separate rectangular placeholder effect.

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
direction, repetition, animated Boolean, optional RGBA integer and optional
appearance (foreground RGBA, background RGBA, dark Boolean), in that order.
Both decoders reject malformed tags, nonfinite values, invalid ranges, truncation
and trailing bytes. Input is bounded to 64 bytes before parsing. Public domain
types do not expose generated deserialization as a validation bypass.

## Appearance and application themes

`Appearance.create ~dark ~foreground ~background ?theme ()` resolves application
colors once. Pass the result as `Config.create ~appearance`; update it when the
application theme changes. Appearance changes are cosmetic and preserve elapsed
time. With no override the Host uses the system window's light/dark semantic
palette. This default does not pretend that an application's custom theme must
match the operating system. Explicit highlight color still overrides the target;
appearance selects the layer opacity. The gallery supplies its actual palette.

The adapter reads GPUI's accumulated paint opacity through a narrow read-only
[core accessor](gpui-core-adaptation.md#read-only-accumulated-opacity). Zero opacity
suppresses the overlay and wakes even when it comes from an ancestor or native
interaction state; partially transparent text uses normal GPUI compositing.

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
memory. The public View constructor/bridge enforces the source bound; each native
declaration reserves 1,024 bytes in the existing window/session payload budget. The fixed-phase constructor
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
| Highlight color, appearance, spread or external style | Preserves elapsed time. |
| `animated=false`, reduced motion, clipped or omitted content | Pauses; a later eligible paint resumes. |
| Completed Once | Remains complete through cosmetic or playback changes; source/timing changes or remount restart it. |
| Invalid configuration or source over 16,384 UTF-8 bytes | Rejects atomically, preserving old source/configuration/time. |

Construction of a rendered element advances prior running time and disarms its
owner **before layout**. Visible paint re-arms it only when the painter admits the
effect. This matters because a fully clipped element can skip paint entirely.
Empty/whitespace text, emoji-only paint, transparent highlights and painter-capacity
fallbacks do not request recurring frames. The host must explicitly suspend a
retained owner omitted by a conditional branch, and drop owners on unmount/close.
The Host owns a generation-keyed map and a per-window clock. Accepted updates
retire removed/cleared owners immediately; each render suspends existing owners
before constructing visible text. Owners share the Tree source Arc, including
replacement allocations with equal text (for example, a span-only update). Retained
branches keep their paused owner; virtual eviction/remount must use a fresh one.
Native checks cover retained tab visibility, hidden source replacement, responsive
branch selection during window resize, and managed-list eviction/remount. An
offscreen row whose description still exists pauses its current owner; removing
that description drops the owner. Reusing the node slot with a new generation
starts a fresh timeline. These are lifetime checks, not list performance claims.

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

## Public description and atomic transport

`View.with_text_shimmer view (Some config)` decorates ordinary `View.text` or
`View.styled_text`; `None` clears it. Core and Bonsai have the same contract. The
key, source, style, metadata and spans remain on the same text node; this is not a
wrapper or action/focus owner. Enabling requires valid UTF-8 of at most 16,384
bytes. Clearing imposes no effect-specific source bound. Other kinds reject both
set and clear. The mounted Host adapter is connected; broader public component
and release acceptance remain in progress.

Appended operation 61 is `Set_text_shimmer (node, Config option)`. Configuration
updates/clear preserve node generation, source and spans; ordinary plain/styled
source updates preserve the decoration. Native source admission checks the final
transaction state, so growing source beyond the limit while clearing the effect
succeeds in either operation order. Invalid configuration, kind, stale generation,
source size or aggregate budget rejects the entire transaction without changing
the acknowledged revision or any node. Operations before 61 and the capability mask remain unchanged. The unreleased
operation-61 configuration now includes optional appearance; rebuild paired halves
together during this experimental stage.

A native declaration reserves 1,024 bytes within existing 64 MiB/window and
256 MiB/session retained-payload admission. The allowance covers optional config,
clock-owner state, map allocation and one weak queued callback; this is not a
measurement of process/allocator overhead. The Host owner shares the
Tree's source `Arc`, already charged by its UTF-8 size. Clearing/unmount releases
the allowance; paused and hidden declarations retain it. Fixed node slots remain
separately bounded by the tree's existing node-count policy. No glyph cache or
source-sized duplicate is introduced by this admission change.

## Remaining integration and acceptance

These are implementation requirements, **not completed acceptance**:

- Complete application-level lifetime acceptance through the public components.
  Native fixtures cover ordinary text owners, retained tabs, responsive branches,
  managed-row pause/eviction/remount, native interaction-state opacity,
  visibility/clipping, clear, generation replacement, independent windows and
  owner disposal before application shutdown.
- Measure aggregate visible work/frame performance and whole-window idle/resource
  behavior. Per-text glyph limits and owner admission do not establish application
  FPS, latency, power or a global paint-work budget.
- Finish foreground OS keyboard/clipboard, screen-reader and public gallery/
  installed-consumer acceptance. Dispatched GPUI keys are not physical input.
- Add Attachment/Marker examples and complete their source behavior review before
  accepting those families or advertising the shimmer capability.

## Current local evidence

The Core expect suite and Rust protocol suite share two independently constructed
byte fixtures covering all variants and optional color. Both enumerate 576 valid
boundary combinations and reject malformed configuration, every fixture
truncation, trailing data and invalid tags. Core also checks sub-millisecond
rounding and token resolution with preserved alpha. These tests establish the
configuration/codec contract only; later native stages are recorded below.

Local macOS 14.5 arm64 commands, through the repository's isolated environment:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @all @runtest @fmt
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -p gpuio-protocol -j 2 --test text_shimmer
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -p gpuio-protocol -j 2 --all-targets -- -D warnings
python3 scripts/audit_component_catalog.py
git diff --check
```

All passed. No GUI window was opened for this configuration-only change. Required
Linux checks and whole-release CI remain separate. Mounted evidence appears below.

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

The following description/transport/admission stage passes the full local Dune
build, expect suite and formatting check. Three additional Core/Bonsai expect
cases cover exact operation-61 set/clear packets, kind/UTF-8/source bounds,
metadata preservation, no-op reuse, discarded preparation, plain/styled source
changes and clearing while growing the source. The independent Rust live packet
case checks the same bytes, strict tags, truncations and invalid configuration.
Four native non-GUI tests cover atomic rollback, source-order semantics, shared
source/span allocations, exact payload-budget boundaries, stale generations and
independent Session windows with clear/close release. Both existing styled-text
admission regressions and eight shimmer clock/painter/color unit tests also pass.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @all @runtest @fmt
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -p gpuio-protocol -j 2 --test text_shimmer
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -p gpuio-native -j 2 --test text_shimmer --test styled_text
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -p gpuio-native -j 2 --lib text_shimmer
```

No GUI window was opened for this stage. The first combined Dune test/format run
reported formatting differences; applying the pinned formatter resolved them,
with no expectation promotion or test-behavior change. These pure transport/
admission checks do not establish mounted rendering, keyboard/AX, public component
examples/consumer, Linux or release acceptance.

Strict protocol and native Clippy also pass for this stage:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -p gpuio-protocol -j 2 --all-targets -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -p gpuio-native -j 2 --lib --test text_shimmer --test styled_text --features native-image-tests -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all --check
python3 scripts/audit_component_catalog.py
git diff --check
```

### Mounted Host integration

The mounted stage passes on macOS 14.5 arm64 through the repository's isolated
environment. The fixture uses accepted Session transactions, the actual Host,
GPU readback, controlled native clocks and real AppKit source-label queries:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -p gpuio-native -j 2 --features native-image-tests --test native_text_shimmer_view --no-run
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -p gpuio-native -j 2 --lib --tests --features native-image-tests -- -D warnings
```

The built executable passed under a 120-second process-group deadline, printed
`GPUIO_NATIVE_TEXT_SHIMMER_VIEW_OK`, closed both background windows and exited zero.
Its checks cover unchanged wrap/selection/copied Unicode source, span colors,
search backgrounds, exactly two logical source accessibility labels, application
theme overrides, twenty renders coalesced into one pending wake, native hover/
pressed/focus opacity, ancestor visibility/display/clipping, reduced motion and
static playback, clear, one-shot completion and generation replacement. A second
actual Host window progresses independently and remains live after the first
closes; both owners retire before application shutdown. GPUI-dispatched key/mouse
events and AppKit label queries do not establish physical keyboard/IME or
screen-reader acceptance.

Review found an owner retaining the previous allocation after a value-equal
styled-source update. A unit regression first failed on the required shared-source
identity, then passed after the owner adopted the current Tree allocation without
resetting elapsed time or invalidating its paint driver. The final mounted fixture
also verifies this sharing after an actual span update. This protects the existing
source-memory accounting; the fixed owner reservation is not an RSS measurement.

The final full Dune `@all @runtest @fmt` check, five protocol tests, four native
admission tests, two existing styled-text admission tests, eight shimmer unit
tests and strict protocol/native Clippy all pass. The standalone painter, retained
clock and existing styled-text native fixtures also pass with the integration.
Initial fixture failures were corrected without relaxing assertions: creation now
respects dense node-slot ordering, and native accessibility waits for the existing
lazy AppKit initialization before querying labels. The first gallery build found
an incorrect style constructor; the final full build passes after using `Radius`.

The read-only GPUI opacity accessor reconstructs byte-for-byte from the pinned
archive plus the checked patch. Catalog audit, workflow lint, formatting and diff
checks pass. The public Presentation page now includes a paused-by-default preview
with source refresh, width, direction, playback, enable/clear and explicit theme
controls. It compiles with the full gallery; normal-launch interaction and fresh
installed-consumer acceptance remain pending. No capability or catalog-family
completion is claimed from this checkpoint. Required Linux and hosted release
checks remain open.

### Retained branches and managed rows

The expanded mounted fixture also passes retained tab hide/show with preserved
phase, hidden-source replacement with a restarted phase, and responsive branch
changes from actual window resize without changing the bridge revision. Visible
phases produce real glyph pixels; hidden owners drain pending callbacks without
requesting more frames. A 100,000-logical-row managed list supplies twelve
materialized descriptions. Scrolling to an unloaded distant page leaves no stale
glyph paint and pauses those still-described owners. Evicting their descriptions
releases every observed owner immediately; reusing the slots with new generations
on the distant page starts at phase zero. This does not measure a 100,000-row
loaded application or establish frame-time/resource release budgets.

This exercise found a separate search-scope redraw loop: creating matching work
for a hidden scope requested a refresh; frame-end cleanup retired the unpainted
scope, so the next render recreated it. The bounded native run timed out and a
process sample showed repeated effect flushing/drawing. The Host now checks
logical/committed native visibility before preparing that work. A follow-up
regression failed when the last hidden scope did not recover on pointer leave.
Frame-end cleanup now commits pending native visibility samples even when no
matcher remains, enabling recovery without bridge traffic. The final fixture
asserts scope retirement/restoration and bounded synchronous redraws on hiding.

On macOS 14.5 arm64, the expanded `native_text_shimmer_view` executable passes with
both `GPUIO_NATIVE_TEXT_SHIMMER_LIFECYCLE_OK` and the original mounted success marker.
The complete `native_highlight_view` fixture also passes its GPU, queued-observation,
virtual-row/window, scroll, retained tab/disclosure, responsive, native hover/
pressed/focus, navigation and deferred-surface regressions. Both ran under separate
120-second process-group deadlines, exited zero and closed their windows. The
first new lifecycle fixture rejected an unreachable old subtree before testing
behavior; it was corrected to preserve the native connected-tree invariant.

The native library suite passes 409 tests, with its two existing private-D-Bus
cases excluded by this macOS command. Strict native Clippy, Rust formatting,
catalog audit and diff checks also pass:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -p gpuio-native -j 2 --lib
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -p gpuio-native -j 2 --lib --tests --features native-image-tests -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all --check
python3 scripts/audit_component_catalog.py
git diff --check
```

No public API, wire format or dependency pin changes in this lifecycle stage.
These background/GPUI-dispatched checks do not add physical input, VoiceOver,
Linux desktop, public-consumer or whole-application performance acceptance.
