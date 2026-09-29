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

## Badge: functional equivalent

Source: [component/badge](sources/component-badge.rs.txt), including `new`, `dot`,
`count`, `icon`, `max`, `color`, child composition, size and actual anchoring/zero
behavior. GPUIO's preexisting `Presentation.badge` remains the text chip.
`Presentation.Overlay_badge` and `Presentation.overlay_badge` now provide the
separate overlay composition through ordinary public views.

| Upstream behavior | GPUIO contract |
| --- | --- |
| Count, maximum (default 99), hide zero | `Overlay_badge.count ?max ~label value` validates nonnegative integers; zero omits the overlay; values above max display `max+`. Zero maximum and `Int.max_value` are safe, with no arithmetic increment. |
| Dot | `Overlay_badge.dot ~label` creates a labeled circle; it has no timer or completion state. |
| Icon | `Overlay_badge.icon` borrows an existing SVG `Icon.Config`; meaningful/decorative descriptions and asset ownership remain explicit. No decoding or I/O occurs during view construction. |
| Wrapped child and anchoring | `overlay_badge appearance ~badge content` keeps content under a stable keyed wrapper. Count/dot attach top-right, icon bottom-right. Badge changes/removal do not remount the underlying control. |
| Size/color/style | Three typed presentation sizes, tone and explicit appearance palette. Root `~style` and `~badge_style` provide ordinary style refinement. Defaults use GPUIO's geometry/colors rather than copying upstream's negative count offsets and theme tokens; ordinary ancestor clipping still applies. |
| Events/commands | No badge-owned callbacks, focus stops, timers or controllers. Underlying controls retain their normal native actions and asynchronous delivery. Pointer eligibility/occlusion and selection policy on the decoration stay passive. |
| Accessibility | Counts/dots require a nonblank UTF-8 label without NUL, at most 4096 bytes. Count labels describe the uncapped value in application language. Each is one image semantic node; the displayed cap does not become a second AX string. Icons preserve their existing meaningful/decorative contract. Zero and decorative icons expose no badge node. No automatic live announcements. |
| Ownership and retirement | Count/dot need no resources. Icons use application registrations and normal mounted leases. Changes preserve body identity, use the latest accepted callback and fence delivery on full unmount. |

Public example:

```ocaml
let badge =
  Presentation.Overlay_badge.count ~label:"150 unread messages" 150
  |> Or_error.ok_exn
in
Presentation.overlay_badge Presentation.Appearance.dark ~badge
  (View.button "Inbox" ~on_click:(fun () -> `Open_inbox))
```

The **Presentation** gallery includes zero, 7, 150 with caps 99/9, activity dot,
verified and decorative SVG modes, all three badge sizes and both appearances.
Its icon registration is acquired in a fresh page scope and released on departure.
Meaningful badge semantics supplement the child's existing label; applications
should use the child's own accessible name when status needs to be announced as
part of a single control name.

Reproduction after the ordinary gallery build:

```sh
python3 scripts/test_gallery.py --section badges --images scratch/badges
```

Local macOS arm64 native evidence (2026-09-29): **36 kind/size/theme cases** pass
actual AX anchors and unchanged 170×48 content bounds, body identity (`CFEqual`),
exact uncapped labels, absent zero/decorative nodes and no duplicate cap string.
**72 real pointer/Return activations** pass; count/dot/meaningful-icon pointer
coordinates lie inside the badge's measured bounds, proving input reaches the
underlying button. Tab moves straight from the content to the next ordinary
control. Both cap settings preserve the full count description. Leaving the
page returns image/chart/canvas counts and registered source bytes to zero;
revisiting reacquires the icon. Marker: `GALLERY_OVERLAY_BADGE_OK`, followed by
`GPUIO_GALLERY_AX_OK: section=badges`. Count/icon screenshots were inspected.
Pure tests cover malformed labels, negative values, zero/max-zero/max-int,
localized labels, all kind/size/appearance updates, stable body identity/latest
actions, idle repeat commits and full unmount fencing.

Full local `dune build -j2 @all @runtest @fmt` passes, including the public gallery
and existing consumers built within the repository. The expanded `--section core`
walkthrough also passes both the 48 status-region and 36 badge cases, followed by
the existing editing/validation/submit, themes/sizes, independent second window
and repeated page teardown checks. Final badge screenshots use success tone for
verification icons and were inspected. Structural catalog hashes, Python
compilation and diff checks pass. These are not fresh installed-consumer or hosted
CI results; those release gates remain outstanding.

The first native run deliberately checked pointer input over the count and found
that the initial inert visual-text child blocked it: the existing inert wrapper
inserts a pointer shield. The final count is a single styled text root with the
uncapped image label, eliminating both that shield and duplicate AX text. Native
inert behavior was not changed. No Rust/protocol/fork additions were needed.

These are local API/geometry/input/resource checks, not a full VoiceOver journey,
physical display transition, arbitrary custom overflow/layout, performance budget
or Linux desktop acceptance. The other catalog and release gates remain open.

## Label: functional equivalent

Source: [component/label](sources/component-label.rs.txt). Beyond `new` and
`Styled`, it exposes `secondary`, `masked` and `highlights`, with
`HighlightsMatch::{Prefix,Full}`. The renderer combines primary/secondary text,
muted styling, a bullet per Unicode scalar when masked, and case-insensitive
prefix/all-occurrence highlighting. It has no action or persistent controller.

Public equivalent: `Label.create` plus `Presentation.styled_label`; the existing
simple `Presentation.label` remains available. The [ordinary text-span contract](../design/text-content.md)
describes the shared Text node, atomic transport, one-layout rendering and retained
selection behavior used by this composition.

| Surface | GPUIO contract and evidence |
| --- | --- |
| Primary/secondary | A single UTF-8 source joins primary and optional secondary with one space. The separator and secondary use muted foreground; absent secondary adds no separator. Empty source/secondary cases have expect coverage. |
| Prefix/all matches | `Label.Match.prefix` and `all` validate a query. Matching lowercases scalars independently and maps matches to original byte ranges; overlapping/adjacent results coalesce. Matches override secondary coloring. Expanding `İ`, accented text, CJK neighbors, overlap, empty query and split-source boundaries have explicit tests. |
| Case policy | Locale-independent scalar lowercasing, without normalization, context-sensitive final sigma or full case folding. `SS` does not match `ß`; composed/decomposed forms are not silently equated. A match inside a lowercase expansion covers the entire original scalar. This explicit policy avoids indexing original text using transformed byte offsets; it is not a claim to reproduce every upstream Unicode edge behavior. |
| Masked display | One bullet U+2022 per source scalar, including the secondary separator. Only the replacement string and empty runs remain in the value; source-dependent formatting is suppressed. The value retains no query. Native default AX and copy receive only bullets. This is display masking, not secret entry or memory erasure; caller-owned source and explicit metadata remain caller-owned. |
| Styling | `Appearance` supplies primary/muted/accent colors; `~style` refines inherited primary/layout properties. Native foreground runs preserve one text flow. Ordinary selection is opt-in with `User_select true`; font/line-height and overflow remain normal application styles. Defaults are GPUIO's, not a pixel-identical clone of upstream. |
| Bounds/work | Combined source/output <=262144 UTF-8 bytes, query <=4096 bytes, lowered temporary source/query <=1048576/16384 bytes, <=4096 final foreground runs. Overflow returns `Or_error`, never truncation. The KMP search and mapping/coalescing are linear in source/query/occurrences. Matching happens in value construction; native painting never calls OCaml. |
| Identity/disposal | The helper emits one ordinary Text leaf, no controller/task/registration. Same-key configuration changes retain native identity. Palette-only updates change resolved colors; masked updates atomically replace source/runs. Gallery page departure releases native state. |
| Evidence/platform | Five Core expect tests cover values, budgets and public reconciliation/theme/masked payloads. Local macOS gallery passes 48 theme/width/secondary/match/mask cases, actual Command+A/C, exact clipboard source, native identity and original-source absence from masked AX. Light/dark/masked screenshots were inspected. Linux desktop remains deferred; consumer/hosted/release gates remain separate. |

The gallery's **Text with context** card exposes matching, secondary, masking and
compact/wide controls. Run the focused native driver with:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 \
  @test/label/runtest examples/gallery/main.exe
python3 scripts/test_gallery.py --section labels --images scratch/label-gallery
```

The test explicitly sets UTF-8 locale for macOS clipboard commands; default
`pbpaste` encoding otherwise produced non-UTF-8 bytes in the first harness run.
It restores prior clipboard text and closes its window. The combined core-gallery walkthrough and full Dune build/tests/format also pass,
including explicit compact-versus-wide wrapping checks. It does not establish
IME or VoiceOver navigation acceptance. Broader gallery and release evidence
is recorded separately in [the gallery report](../evidence/gallery-och41.md).

## Group box: functional equivalent

Source: [component/group_box](sources/component-group-box.rs.txt), including
`GroupBox`, `GroupBoxVariant`, `GroupBoxVariants`, `Styled`, `ParentElement`, and
the renderer. Public equivalent: `Presentation.group_box` and `Group_variant`.
The pinned source is preserved verbatim and checksummed in the source manifest.

| Surface | GPUIO contract |
| --- | --- |
| Normal, Fill, Outline | `Group_variant.Plain`, `Filled`, `Outline`. The header sits outside the body panel; Filled decorates the body with the appearance's raised surface, Outline adds its border, and both pad the body. Plain adds neither panel nor padding. |
| Existing card | `Card` remains the default, preserving the original outer border/surface/padding around all slots. It is an additional GPUIO appearance, not an upstream variant. |
| Title and arbitrary children | Optional `~header` and ordinary body view list. Optional `~footer` is an additional GPUIO slot. All accept controls and independently styled rich views; no string-only restriction. |
| Title/content/root styles | `~header_style`, `~body_style`, `~footer_style` refine the stable wrappers; root `~style` refines the outer layout. GPUIO uses its own palette, spacing and radii; no pixel-identical upstream theme promise. |
| Identity/state | `~key` uses ordinary scoped reconciliation identity. Stable internal header/body/footer keys preserve body controls when slots, appearance, variant or styles change. This helper owns no controller/model/task/registration. Model retention is the caller's policy; it does not reset caller state on variant changes. |
| Events/commands | None owned by the group. Children retain asynchronous input delivery and current callbacks. Removed slot actions and complete unmount are fenced by ordinary reconciliation. |
| Accessibility/focus | Structural Group semantics, no added focus stop or automatic live announcement. Applications supply an accessible group name with `View.with_accessibility`; children retain their normal roles/actions. This helper does not automatically turn a header into a heading. |
| Builder utilities | Typed OCaml variants replace Rust convenience builders and string conversion helpers. No implicit string parser with unknown-value fallback is introduced. |

Example:

```ocaml
Presentation.group_box appearance
  ~variant:Outline
  ~header:(View.text "Workspace")
  ~body_style:(Style.create_exn [ Gap (Length.px_exn 16.) ])
  [ View.checkbox ~state:Checked ~on_toggle "Synchronize changes" ]
```

The gallery's **Structure with flexibility** card exposes all four appearances,
header/footer visibility and independent wrapper refinements around a persistent
checkbox and action. Its scalar Bonsai model persists while the page is inactive;
the native subtree unmounts, becomes inaccessible, and remounts from that model.
This differs from scoped native resources, which are released on page departure.

Reproduction:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 \
  @test/view_api/runtest examples/gallery/main.exe
python3 scripts/test_gallery.py --section groups --images scratch/group-gallery
```

Validation evidence is recorded in [the gallery report](../evidence/gallery-och41.md).
The expect regression checks default compatibility, both palettes and all
variant/slot combinations, retained checked controls/latest callbacks, retired
slot actions, idle repeat commits and full-unmount fencing. Native checks measure
actual header/body/footer positions and style overrides, query checkbox state and
identity, and drive real keyboard/pointer actions. These scoped checks do not
constitute a full VoiceOver journey, physical display-scale transition, arbitrary
application layout, Linux desktop or release performance acceptance.

## Links: configuration foundation, native composition still pending

Both [base/link](sources/base-link.rs.txt) and
[component/link](sources/component-link.rs.txt) have been reviewed in detail.
They are **not yet functional-equivalent acceptance rows**. Existing
`Presentation.link` covers text, styles, asynchronous actions and disabled fencing;
composed content and explicit Tab participation/order remain gaps. The legacy
styled component's ineffective disabled flag and pointer-only behavior are not
contracts to reproduce. The base link's injected navigation agrees with GPUIO's
application-owned opening policy.

The [composed-link design](../design/composed-links.md) records every builder
mapping, validated configuration, transport fixture, native focus-order hazard
and required implementation/validation phases. `Link.Config` plus bounded OCaml/
Rust data codecs are implemented foundations only: no new mounted capability is
advertised. Both link rows remain pending native/gallery closure.

## Remaining presentation modules

The link rows above remain incomplete. Component `alert`, `attachment`, `bubble`, `description_list`,
`empty`, `kbd`, `link`, `marker`, `message`, `separator`, `setting`,
and `tag` retain pending detailed reviews. Prior OCH-33 implementation evidence
still applies to the GPUIO APIs it actually tests. Nested types/builders, settings
field behaviors, style/semantics and commands must be checked before broader
equivalence claims. Other catalog families, consumer/CI/distribution checks and
the complete OCH-17 acceptance scope remain open.
