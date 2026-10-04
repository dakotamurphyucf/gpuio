# Slider behavior review — OCH-41

Source review, 2026-10-02. This records functional coverage and missing styled
behavior; it does not declare slider-family release acceptance.

## Pinned sources

Unmodified `crates/base/src/slider.rs` and `crates/component/src/slider.rs` from
GPUI Kit `84f57fdfcb4910623fb0bb7f795b077e249f9271` are saved under `sources/` and
hashed in [the manifest](sources/manifest.json). They were extracted from the
matching local Git commit, not a moving branch or patched vendor file. These
snapshots are research inputs, not runtime/build dependencies.

## Functional mapping

| Pinned functionality | GPUIO equivalent and intentional differences |
| --- | --- |
| Single or two-thumb range; initial/programmatic value | `Slider.Value`, mount-only `initial`, Eio `replace` and `replace_if_unchanged`. A mounted control keeps its mode and thumb identities. Mode changes require remounting. |
| Minimum, maximum and step | Validated `Numeric.Domain` uses finite f64 values and a min-anchored grid. Pinned upstream pointer input rounds with `(value / step).round() * step`; GPUIO intentionally uses its shared domain normalization, including an irregular maximum endpoint. |
| Linear and logarithmic position mapping | `Slider.Config.scale`; logarithmic domains require positive minimum. Mapping and native pointer input already exist; the gallery now demonstrates both scales. |
| Horizontal and vertical input | `Slider.Config.axis`; vertical values increase upward. Both layouts are now exposed by the gallery. Changing axis/scale/domain cancels an active gesture and retains the owner. |
| Disabled behavior | Config disable and inherited subtree policies block native mutation/focus as appropriate. Read-only separately preserves focus/read behavior. Programmatic replacement remains permitted. Gallery controls expose these policies. |
| Change and Release events | GPUIO distinguishes Observed, Drag_started, Preview, Committed and Cancelled. Preview can coalesce within a gesture; lifecycle boundaries cannot. The gallery displays preview and committed values separately. |
| Track/indicator/thumb composition | GPUIO retains one native owner and built-in track/thumb geometry. Ordinary View styles decorate the outside, but there is no public arbitrary native track/thumb replacement API. `Slider.Appearance` now exposes bounded part geometry and colors; native hover/pressed rings follow application motion preference. |
| Keyboard and accessibility | Native arrows, page steps, Home/End, Escape cancellation, individual range-thumb Tab stops, AX values/bounds and increment/decrement/set-value actions. Existing native evidence is recorded under OCH-34; this review adds no physical desktop acceptance. |

Public interfaces are [Core Slider](../../lib/core/slider.mli) and
[Eio Slider](../../lib/eio/slider.mli). The
[ownership contract](../design/numeric-inputs.md#slider-ownership-and-event-contract)
describes exact revisions, cancellation, geometry invalidation and bounded delivery.

## Styled behavior and remaining gaps

The pinned styled component exposes `reverse()`: for a single slider it fills
from the thumb to the maximum, without reversing values or interaction; ranges
ignore it. GPUIO now maps this to `Slider.Fill.Remaining` in `Slider.Appearance`.

Upstream uses background color for the selected rail and text color for the
thumb, caller corner radii, and a spring-driven three-pixel hover/pressed ring.
GPUIO now provides separate track/fill/thumb/focus-ring colors and bounded rail
thickness/radius, thumb size, target size and focus-ring width through Appearance.
Defaults preserve the former 4/12/20 geometry and shared foreground behavior.
Theme updates reconcile the presentation independently of editing config.

The mounted TestPlatform check verifies both axes and single/range fills against
painted geometry, independent colors, focused-owner retention, live capture across
paint-only updates, and cancellation on a target-size change. Reset restores the
defaults. See the [presentation contract](../design/slider-presentation.md) and
[checkpoint evidence](../evidence/slider-presentation-och41.md).

Native hover/pressed rings now use independent critically damped thumb springs,
with one weak pending frame per slider and no bridge events or idle frame demand.
Reduced motion settles immediately; hidden, inactive, disabled, read-only, clipped
and transparent controls stop. Deterministic native tests cover pointer capture,
range-thumb independence, policy changes, removal and close. A scalar rail radius
is supported; upstream per-corner rail radii and arbitrary native track/thumb
replacement are not exposed. Physical input/visual/AX acceptance and measured
resource validation remain open. The family is not fully accepted.

## Gallery checkpoint

`examples/gallery/slider_preview.ml` now demonstrates the existing single/range
API, axis/scale updates, disabled/read-only behavior, preview versus committed
values, reset, cancel-drag and individual thumb-focus commands. Reversed single fill, independent colors and larger thumbs now also
use the public Appearance API. Controller
construction stays outside the dynamic presentation branches. Feedback uses a
request epoch and page-deactivation guard so late replies cannot replace newer
feedback. The numeric page composes this preview alongside numbers, OTP and
rating.

`GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @check examples/gallery/main.exe`
passes locally on macOS, dirty worktree based on `83eb87e`. This is compile
coverage for the changed public example, not a new native or physical-input
result. No OS windows were opened. Styled gaps above, physical gallery behavior,
accessibility, resources and release validation remain open.

`GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @fmt examples/gallery/main.exe`,
`python3 scripts/audit_component_catalog.py` and `git diff --check` also pass.
No new model tests were added for this gallery-only composition change; the
underlying slider engine was not modified.
