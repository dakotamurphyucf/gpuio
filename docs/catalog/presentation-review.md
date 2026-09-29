# Presentation behavior review (OCH-41)

Review baseline: GPUIO `8d0c2ce`, with the status-bar addition below. Upstream
Longbridge revision `84f57fdfcb4910623fb0bb7f795b077e249f9271`. This is a partial
review of the 17 modules assigned to the presentation family, not family or
release acceptance. The source snapshots linked below are exact pinned files,
checksummed in [the manifest](sources/manifest.json), covered by the adjacent
upstream Apache license, and verified by `scripts/audit_component_catalog.py`.

## Status bar: functional equivalent

Source: [component/status_bar](sources/component-status-bar.rs.txt), including
`StatusBar::{new,left,right}`, `ParentElement::extend`, `Styled`, and `render`.
Public equivalent: [Presentation.status_bar](../../lib/core/presentation.mli).
The upstream renderer, not just its public method names, determines this mapping.

| Surface | GPUIO contract and evidence |
| --- | --- |
| Left, center children, right regions | `~leading`, `~center`, `~trailing`. Each accepts an arbitrary ordinary view; multiple children compose with `View.row`. The new optional center slot closes the previously missing region. |
| Adaptive center alignment | With both ends, center in the remaining space; with only the leading end, align to the end; otherwise align to the start. It does not promise alignment with the window's absolute midpoint. Native AX geometry verifies all four arrangements. |
| Empty regions | Optional ends are absent, an empty center still supplies flexible space. Omitting `center` preserves the existing two-region layout, including the flexible leading region. |
| Styling | `Appearance` supplies border and muted foreground; `~style` refines the root, and each supplied child has its ordinary styling API. GPUIO uses its own padding/gap/font defaults and inherits the background instead of copying upstream theme tokens. Ordinary wrapping/overflow policy applies; upstream's unconditional region clipping is not imposed on application controls. |
| Events and commands | The bar registers none. Buttons/editors inside slots retain their normal asynchronous action/command delivery. It does not create a separate native delegate or synchronous OCaml callback. |
| Accessibility and focus | The bar is structural, not automatically a live region or focus stop. Supplied controls retain their roles/actions. The gallery explicitly labels the outer group, queries actual button bounds, checks native identity with `CFEqual`, and sends real Return input after requesting focus. Applications opt into status announcements with existing accessibility metadata. |
| State and disposal | Slots have stable internal keys. Adding/removing other slots or changing appearance preserves surviving node/handler identity; removed actions stop dispatching. No new task, controller, registration, timer, or native resource owner exists. Unchanged descriptions emit no reconciliation operations. |
| Platforms | Current geometry/input evidence is local macOS arm64. This pure public composition uses the existing cross-platform primitives; fresh Linux build/consumer gates remain required, and Linux desktop behavior remains OCH-47. |

The gallery's **Presentation** page lets users toggle each region and activate
the center action. It uses public Core/Bonsai APIs. Focused native reproduction:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 \
  @test/view_api/runtest examples/gallery/main.exe
python3 scripts/test_gallery.py --section status-regions --images scratch/status-regions
```

Local macOS results (2026-09-29): the complete view API expect suite passes,
including surviving/retired action checks for all eight slot combinations in
both appearances. The external gallery passes **48 combinations** (eight slot
combinations × two appearances × three preview sizes), including 24 real Return
activations, actual between-region geometry, focus, native identity across end
changes, absent AX descendants, and clean process shutdown. Markers:
`GALLERY_STATUS_REGIONS_OK` and `GPUIO_GALLERY_AX_OK: section=status-regions`.
Dark/light screenshots were visually inspected. Preview sizes change logical
fonts/control sizes; this does not claim physical display-scale transitions,
VoiceOver acceptance, arbitrary narrow-window layout, or release performance.

The first driver run failed because its text-field reader cannot read numeric
checkbox values. The corrected driver waits for the actual affected region's
presence/absence and measured geometry. No product failure or user interference
was inferred from that diagnostic.

The broader `--section core` walkthrough also passes, covering its existing form
editing/submission, validation retention, theme/size changes, second-window
isolation, repeated page unmount/remount, avatar and loading checks. Its first
run left the initial editor draft unchanged after sending keys immediately after
an AX focus request. The driver now uses the existing observed-focus readiness
helper before those keys; the unchanged text assertion passes. That result does
not establish whether focus timing was the only cause of the earlier failure.

Full local `dune build -j2 @all @runtest @fmt` also passes, as do the structural
catalog audit, Python compilation and diff checks. The full build first found
two earlier low-level examples with exhaustive matches missing the already-added
`Document_diff_event` constructor. Their explicit ignored-event lists now include
it; neither example displays diff documents. No wildcard or warning suppression
was added. No new Rust/protocol/fork code is part of this status-bar change.
Fresh installed-consumer, hosted macOS/Linux and release gates remain outstanding.

## Badge: functional gap remains

Source: [component/badge](sources/component-badge.rs.txt). The public surface
includes `new`, `dot`, `count`, `icon`, `max`, `color`, child composition and size.
The renderer wraps ordinary content and overlays a dot, count, or icon. Numeric
zero hides the overlay; values above the configurable maximum render `max+`.
Dots sit at the top/end, icon badges at the bottom/end. It registers no action.

GPUIO's `Presentation.badge` is currently a rounded text chip with optional
leading content, tone, variant, size and ordinary root style. It is useful and
tested, but **it is not this overlay badge**. `Presentation.marker` is not a
replacement either. Existing absolute positioning can express an overlay, but
there is no reviewed public composition/example proving count caps, zero hiding,
anchoring, contained control focus and readable non-color status semantics.

Required follow-up within OCH-41/OCH-33's accepted family: provide a coherent
overlay composition/API without breaking the existing text chip, validated
count/max semantics, and a public example with native geometry, accessibility,
actions and update/teardown evidence. Do not mark this module equivalent merely
because both libraries have a function named `badge`. No post-v1 deferral has
been accepted for this gap.

## Label: configuration review remains incomplete

Source: [component/label](sources/component-label.rs.txt). Beyond `new` and
`Styled`, it exposes `secondary`, `masked` and `highlights`, with
`HighlightsMatch::{Prefix,Full}`. The renderer combines primary/secondary text,
muted styling, a bullet per Unicode scalar when masked, and case-insensitive
prefix/all-occurrence highlighting. It has no action or persistent controller.

GPUIO's `Presentation.label` supplies ordinary text with Label semantics and
style. Secondary text can compose through ordinary views, and the separately
implemented `Highlight` system supplies scoped searches/ranges and GPU painting.
These facts do not yet prove the particular secondary/prefix/masking behavior.
There is no label masking configuration in the current public helper.

Required follow-up: review an explicit public recipe or API for secondary text,
prefix/all-match styling and masked display, including non-ASCII boundaries and
accessibility/copy behavior. A visually masked string must not accidentally
expose its original text through AX/copy/highlight metadata. Use validated source
offsets; do not copy upstream's lowercased-byte offset arithmetic without checking
Unicode expansion. This is a remaining audit/implementation item, not a claim of
an upstream bug or a new secret-entry widget contract.

## Remaining presentation modules

`base/link`, and component `alert`, `attachment`, `bubble`, `description_list`,
`empty`, `group_box`, `kbd`, `link`, `marker`, `message`, `separator`, `setting`,
and `tag` retain pending detailed reviews. Prior OCH-33 implementation evidence
still applies to the GPUIO APIs it actually tests. Nested types/builders, settings
field behaviors, style/semantics and commands must be checked before broader
equivalence claims. Other catalog families, consumer/CI/distribution checks and
the complete OCH-17 acceptance scope remain open.
