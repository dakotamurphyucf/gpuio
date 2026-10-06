# Native chart inspection controls — OCH-41

Implementation based on `52ebe59`, 2026-10-06, macOS 14.5 arm64 / M1 Max.
**Local qualification passes.** The [contract](../design/chart-inspection.md)
adds typed details-card, crosshair and marker appearance through
`Chart_style.create ~inspection`. This page does not claim catalog/release acceptance.

## Implementation and completed checks

`Chart_inspection.Card`, `Crosshair` and `Marker` provide validated public types.
Native overlays retain exact prepared source summaries and selection IDs, with
optional anchored-card placement, axis/pattern/band controls and independent
marker appearance/status. Colors resolve through the style theme; ordinary
hover/preview needs no OCaml effect or new serialized source publication.

The style schema tag becomes -1, preserving a nonpositive namespace disjoint
from valid legacy palette counts. Both version-0 and earlier unversioned frames
are rejected. Paired literal fixtures cover the default inspector appended to
style/view records; all enum combinations, color bounds, finite dimensions and
coupled text/stroke constraints are validated. A boxed fixed-size native record
avoids inflating every transaction operation, and counts in retained accounting.
Options version 3, data version 1 and dependency/toolchain pins are unchanged.

Current local native tests pass **1007, with two existing skips**. Protocol tests
pass **424**. Focused OCaml chart/gallery/format checks and strict all-target
Clippy pass. Full OCaml checks and the independent installed-consumer walkthrough pass.
Native placement tests cover left/right/up/down choices, corner placement and
small/large viewport limits; they do not prove on-screen pixels alone.

A focused real macOS inspection walkthrough passes. It verifies default,
vertical dashed, horizontal translucent band, anchored card and marker-only
presets through actual chart pixels. It also checks card bounds, keyboard
selection, updates from 30 to 33, original-data access and resource teardown.
The anchored-card capture was visually inspected: both guides, border, source
summary and outlined selection marker match the preset. These are native
rendering/input checks, not VoiceOver or physical presentation timing.

## Findings and remaining validation

The first native compile needed the extracted module's GPUI interactive traits.
An invalid-tag fixture initially corrupted an identity byte rather than its tag;
the independent header sum was corrected to 43 bytes. No expectations were
promoted automatically. Lint caught the enlarged protocol enum before the
inspection record was boxed and charged.

An initial gallery capture lacked the new controls; it has no accepted executable
identity and is not used as evidence for this feature. The driver now logs the
launched executable's SHA-256. A subsequent identified run caught **zero changed
pixels** for a vertical dashed guide. Giving the dashed strip nonzero layout
area and clamping it inside the plot repaired the focused test: 158 changed
pixels, including 100 accent-colored samples. The solid band changes 7944 pixels,
anchored presentation 32900 and marker-only presentation 18870 in that run.
The pixel regression remains in the maintained driver.

Several complete runs lost their accessible/on-screen window. Temporary tracing
showed the GPUI window remained owned and AppKit reported it visible, unminimized
and in an unhidden app. CoreGraphics retained its identity/bounds in the all-window
list, but not the on-screen list; AXWindows returned an empty array. The owner
confirmed switching views during tests. These runs are failed/partial evidence,
not acceptance. All temporary runtime, vendor and driver tracing was removed.

Two separate issues were then corrected:

- The plot's BlockMouse hitbox swallowed wheel events despite owning no wheel
  gesture. BlockMouseExceptScroll preserves selection/capture while allowing the
  enclosing page to scroll. The unchanged Stacked bars reveal path reproduced the
  failure before the repair and passes afterward. Removing an extra overlay clip
  alone had not fixed it; that initial hypothesis was disproved.
- Three exact Ready-label expectations omitted the new Default inspection suffix.
  With the window kept visible, the AX dump showed the correct categorical state
  and the longer label. Updating those exact expectations made the full chart
  walkthrough pass; timeouts and visibility guards were not relaxed.

The clean root walkthrough now passes all seven families, mixed/categorical/stacked
and ordinal modes, every inspection preset's pixels, all four Cartesian directions,
selection/publication, original-data paging, style changes and scope teardown.
It closes/reaps the owned window and reports zero registered chart resources and
source bytes. The clean native suite, protocol suite, full OCaml build/tests/format
and strict Clippy pass. No VoiceOver or clipboard settings were changed.

An independent consumer stages the public OCaml libraries into its own prefix,
builds a composed native backend, and passes the same full chart walkthrough.
Its executable SHA-256 is
`116287156e0cb0ff5a1fb9784b6107b071555b6f080895cfeaf453962f1acb3d`.
No project switch or default is changed. This is local installed-package
qualification, not a new clean-machine release or Linux GUI claim.

The [source overlay, raw logs and images](chart-inspection-och41/evidence.tar.gz)
are preserved with a [hash manifest](chart-inspection-och41/manifest.json).
Failed diagnostic runs are retained separately from final passing checks; the
final source contains none of the temporary window/worker traces. Relative file
links in the eight changed contract/catalog/example documents were checked.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-protocol
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native --lib --features native-image-tests,native-canvas-tests
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j2 -p gpuio-native -p gpuio-protocol --all-targets --features native-image-tests,native-canvas-tests -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt
python3 scripts/test_gallery.py --executable _build/default/examples/gallery/main.exe --section charts --images scratch/agents/root-20261004-resumed/chart-inspection-qualified-images
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --run --gallery-section charts --workspace scratch/agents/root-20261004-resumed/chart-inspection-consumer
```

The [preset companion](../../examples/charts/samples/inspection.md) and
[page walkthrough](../../examples/gallery/charts_page.md) add partial OCH-48
coverage. Arbitrary rich tooltip rows, cursor-following placement, partial guide
spans and broader label/per-datum options remain explicit catalog work. No Linux
GUI, VoiceOver, performance or whole-release acceptance follows from these checks.
