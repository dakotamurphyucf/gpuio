# Journey and workspace source review

OCH-41, 2026-10-02. Seventeen exact source snapshots from GPUI Kit
`84f57fdfcb4910623fb0bb7f795b077e249f9271` are retained in `sources/manifest.json`:
base `nav_stack`, `tabs`, `resizable/{mod,panel,resize_handle}`; component
`sidebar/{mod,menu,group,header,footer}`, `carousel/{mod,carousel,state,scroll_mask}`
and `tab/{mod,tab,tab_bar}`. The archive SHA256 is
`909c00c97bbfce11607eef7d3eccd01201d591af9d26cd9ebadfe0c90fca502f`.
The component root's `resizable` module reexports the base types; there is no
separate styled resizable implementation to infer from that module name.

This review separates existing functional equivalents from remaining source-level
gaps. It does not accept entire families or replace physical release testing.
Earlier OCH-15/OCH-37 delivery evidence describes their original scope; broader
pinned-source coverage below remains part of OCH-41.

## Sidebar

| Pinned behavior | GPUIO equivalent or remaining work |
| --- | --- |
| Groups, recursive menu items, active/disabled state | `Sidebar.Item`, `Group`, `create`, selected ID and explicit expanded IDs. Bounded to 4096 items/depth16/128 groups/256 KiB metadata, independently of normal view resource quotas. |
| Destination activation and independent caret | `Request.Select` and `Request.Toggle`, reduced against current eligibility. Selecting preserves expansion by default; caret never navigates. |
| `click_to_open`, `click_to_toggle` | Opt-in `Item.Activation.Expand` and `Toggle`; default `Select_only`. Expand is idempotent; toggle applies once per accepted request. One typed policy replaces the upstream two-Boolean precedence rule. Both still select the destination. In compact mode the stored preference changes while descendants remain hidden. |
| `default_open` and retained native keyed expansion | GPUIO makes expansion application-owned from creation onward. Collection replacement preserves surviving expansion; a changed activation policy is read when reducing the request. Programmatic `Sidebar.select` changes only selection. |
| Icon/offcanvas/never, left/right | `Collapse`, `Side`, bounded `Appearance` widths and native allocation motion; retained offcanvas content becomes inert immediately while exiting. `Content_policy` explicitly controls native retention; Bonsai/Eio lifetimes remain application-owned. |
| Icon, suffix, context menu | `Decoration` uses registered SVGs, ordinary suffix views and scoped commands. Compact mode hides suffixes and descendants while preserving destination labels/tooltips. |
| Header/footer and group label | Public responsive header/footer callbacks run during OCaml view construction; group labels and styles are ordinary retained views. No callback runs from native layout. |
| Root/item/current/group styling | `Appearance` with theme tokens and normal highlight styles. |
| Per-item label-only style | `Decoration.style` and `label_style` refine one link and its passive text separately. Stable icon/text slots preserve destination identity across updates/reset/compact mode; native links preserve the full 4096-byte name. The experimental constructor now explicitly rejects blank names. See the [contract](../design/sidebar-styling.md) and [local evidence](../evidence/sidebar-styling-och41.md); physical qualification remains open. |

The Journeys gallery exposes the three activation policies while retaining the
sidebar. [Sidebar activation evidence](../evidence/sidebar-activation-och41.md)
records current local checks. Existing physical sidebar coverage is historical
OCH-37 evidence; the new policy walkthrough is not physically qualified yet.

## Navigation history

| Pinned behavior | GPUIO equivalent or boundary |
| --- | --- |
| Push, root-protected pop, forward, replace, clear, depth and current view | `Navigation_stack` bounded immutable history and `Entry` payloads. Instance IDs separate repeated visits. Payloads are never serialized by the history model. |
| Outgoing page retained during transition, interruption and reduced motion | `View.navigation_stack`, native slide/fade/immediate motion and `Content_policy`. Native input/focus eligibility follows the selected page rather than paint order. |
| Animated versus immediate per operation | Choose `Motion.immediate` or a native motion value when constructing the accepted updated view. Selection/history stay controlled by the application. |
| Change events from owned Rust history | Application reducer operations already know the accepted history change; no duplicated Rust-owned history or second authoritative selection is introduced. |
| Custom `NavPage` renderer with operation/phase/per-frame progress | Built-in native slide/fade presets and the native extension SDK provide the customization boundary. The standard API deliberately does not call OCaml per frame or serialize native `AnyView` objects. A source-level callback signature is not a separate promised OCaml component. |

See `lib/core/navigation_stack.mli`, `View.navigation_stack` and the original
[OCH-37 behavior evidence](../evidence/navigation-components-och37.md). Full current
macOS accessibility, resource/performance and gallery acceptance remain required.

## Carousel

| Pinned behavior | GPUIO equivalent or remaining work |
| --- | --- |
| Selected index, item count, previous/next/first/last, looping and axis | `Carousel` uses stable item IDs and a checked revision lineage. Current-model request reduction owns selection; relabel/reorder/removal semantics are explicit. |
| Keyboard, pointer axis lock, drag preview/snap, wheel bursts and nested scrolling | Existing native carousel input/gesture adapter, retained page transition owner and child input precedence. Local tests include nested popup focus/hover and disposal; latest desktop qualification remains open. |
| Previous/next and numbered pagination controls | `View.carousel` default controls, or `show_controls:false` and application-composed controls emitting typed requests. |
| Root/content/track/item styling and measured arbitrary item extents | `View.carousel_track` now supplies retained variable-extent items, per-item styles, measured stops and duration/easing movement. Both-axis host tests cover neighboring controls, padding, resize and paint/input/AX coordinates. Viewport keys and native automatic deadlines have controlled-clock host evidence. Measured background dragging now has both-axis host, cancellation and native-editor precedence evidence. Measured trackpad/wheel now has pixel preview, quiet-deadline, burst ownership and nested-scroll host evidence. An explicit control group supplies pointer/keyboard focus behavior, scope-wide automatic pause and card set metadata. Clipped-card focus/input/AX, nested-scroll reveal, retained modal independence and anchored-popover suspension/resumption have host evidence. The Journeys gallery now includes a public measured-card preview, and a Bonsai driver test checks ordered request reduction and stale-event fences. Full AX and physical gallery acceptance remain open. Full-page `View.carousel` remains a separate mode. |
| Continuous loop runway/rebase for the measured track | Track geometry distinguishes finite, immediate boundary jump and continuous movement. Continuous samples rebase with one retained owner per card; pure periodic-coverage and host placement tests pass. This duration/easing adaptation does not claim the pinned styled spring trajectory. Gesture integration and physical/resource acceptance remain open. |
| Auto advance | GPUIO's explicit additional contract: one bounded pending proposal, native deadline after settled paint, pause on interaction/hidden/inactive/reduced motion, no missed-tick catch-up. It is not an inferred pinned `CarouselState` feature. |

Measured-track support requires a distinct native layout/gesture contract preserving
application selection, stable item identity, bounded layout data, child focus,
clipping and lifecycle. It remains OCH-41 v1 catalog work, not an unannounced
post-v1 deferral. Core callbacks must remain asynchronous. The existing full-page
mode and its guarantees must continue working.

## Tabs and resizable workspace

| Pinned behavior | GPUIO equivalent or remaining work |
| --- | --- |
| Base Tab/TabList semantics, selected/disabled state and click | `View.tab_bar` + checked `Choice.Config`, native roving focus/arrow/Home/End selection and skipped disabled tabs. `Workspace` separately owns bounded application tab order, active ID and payloads. |
| Retained panel state | `View.tab_panel` and application-owned keyed content preserve native editors/lists; removal/task cancellation stays explicit. |
| Styled pill/outline/segmented/underline tabs, icon/prefix/suffix and per-tab styling | `View.tab_bar_with_labels` now supplies checked decorative labels (including icons/badges) with stable Choice-ID slots and configured accessible names. The Navigation gallery exposes rich/plain labels. `Tab_bar.Appearance` now supplies five static variants, native target geometry and shared/per-ID state styles, with plain/rich native tests and public gallery controls. `View.tab_bar_with_content` adds ordinary interactive prefix/suffix controls, decorative/default/hidden labels and maximum whole-tab width, with retained keyed owners and separate activation. The gallery adds close/reorder/restore/truncation controls. `Tab_bar.Motion` now supplies interruption-preserving indicator springs and Pill inherited-foreground fading, with reduced-motion/hidden/idle/teardown and actual native scene tests. The gallery exposes an animation toggle; see [motion evidence](../evidence/tab-motion-och41.md). Physical appearance/AX acceptance remains open. See the [rich-tab review/design](../design/rich-tabs.md). |
| Native overflow scrolling and external scroll handle | `Tab_bar.Viewport` owns horizontal native scrolling; typed serialled `Reveal_request` replaces an exposed Rust handle. Controlled selection alone does not scroll. Keyboard/assistive focus and child focus reveal current measured targets; reorder, resize, hidden state, request lifetime, user wheel offsets and teardown have native regressions. The public gallery contrasts Select last and Reveal last. See the [contract](../design/rich-tabs.md). Physical desktop qualification remains open. |
| Tab-bar prefix/suffix/trailing space | `View.tab_bar_frame` supplies explicit fixed prefix/suffix slots and ordinary trailing content inside the native scroller. Empty fixed slots leave no extra gap. All three tab presentations retain owners through changes, and trailing controls have independent focus/actions. Exact public transactions replay in native admission/layout tests; see [frame evidence](../evidence/tab-frame-och41.md). Physical qualification remains open. |
| All-tab menu | `Tab_bar.Menu` and `View.tab_bar_frame ~menu` now supply full-name rows, selected checks, disabled options, native keyboard/typeahead and menu accessibility, with shared controlled choice requests and current-model/stale-owner fencing. Public transaction and production-host tests cover insertion, reorder/relabel/disable, independent viewport state and teardown. Explicit keyed `Icon.Decoration` rows now retain native asset readers while closed/offscreen and render only visible rows, with source/style/reset/window-close tests. **Physical qualification remains required.** See [menu evidence](../evidence/tab-menu-och41.md) and [icon evidence](../evidence/tab-menu-icons-och41.md). |
| Horizontal/vertical panel groups and live resizing | `View.split_pane` provides two native panes; `View.split_group` now provides measured flat groups with stable panel IDs, captured native dragging, keyboard/AX resize and final observations. Both preserve retained child owners. |
| Panel min/max, hiding and programmatic resize | `Split_group.Panel` supplies each panel's full range, initial preference and visibility. `Config` supplies reset generations and serialled one-shot `Resize_request`; reorder/hide preserve keyed preferences. Native constraint redistribution and public transaction replay have local tests. |
| Arbitrary groups, each panel's full range, custom handle appearance | `View.split_group` and Bonsai forwarding now connect the [typed design](../design/split-group.md) to checked native admission, retained Host ownership and asynchronous observations. `Appearance` supports shared/per-ID paint states, separate hit/paint extents and optional passive grips. The Navigation gallery demonstrates reorder, hide/show, insertion/removal, bounds, resize/reset and grips with a retained editor. Production TestPlatform tests cover child/focus lifetime, clipping, accessibility metadata, one-shot requests and cleanup. See [bridge/gallery evidence](../evidence/split-group-bridge-och41.md), [widget evidence](../evidence/split-group-widget-och41.md) and [foundation evidence](../evidence/split-group-foundation-och41.md). Physical macOS and release qualification remain open. |

These workspace differences remain within OCH-41's component review, independently
of deferred comprehensive docking (OCH-43). Do not use the docking deferral to hide
basic tabs/split functionality. Baseline evidence is in
[agent workspace](../evidence/agent-workspace-m4.md); current gallery examples are
Navigation and Journeys. The source ledger remains open until required feature
work and actual platform evidence are complete.

Measured-track implementation has started with a
[typed API/ownership design](../design/carousel-track.md) and a
[geometry/layout foundation](../evidence/carousel-track-foundation-och41.md).
Duplicate snap positions, unequal item sizes, real GPUI measurements and
single-owner looping constraints now have focused evidence. The
[OCaml model and standalone payloads](../evidence/carousel-track-model-och41.md)
also pass direct and installed-package checks. Core/Bonsai view construction,
paired transport, checked tree admission and native publication/deadline state now
exist. Measured native rendering, same-frame pointer positioning and retained
neighbor buttons now have adapter tests. Motion, native gestures/keyboard, automatic clock integration, related controls
and clipped-card focus/input now have adapter evidence. The public Journeys
gallery and Bonsai driver are implemented and build from installed packages.
Complete accessibility and physical gallery/resource qualification remain open;
see the [gallery checkpoint](../evidence/carousel-track-gallery-och41.md).

## Sidebar physical follow-up — 2026-10-08

The [desktop walkthrough](../evidence/sidebar-macos-och41.md) now passes local
and installed-consumer branch policies, actual Return/Space and pointer actions,
twelve theme/scale/style geometry cases each, compact/offcanvas preferences and
page remount. It distinguishes exposed AX identity from platform objects retired
while hidden. The earlier sidebar physical-gap wording above is superseded for
this recorded scope; VoiceOver, motion timing, measured resources, other nested
options and consolidated release acceptance remain separate.

## Measured carousel physical follow-up — 2026-10-08

The [desktop navigation evidence](../evidence/carousel-track-macos-och41.md)
qualifies a subset of the existing physical plan locally and in the installed
consumer: native keys/editor retention, both axes/themes, clipped-neighbor pointer
input, reorder/resize, immediate looping, disabled controls and page remount.
Gesture ownership/cancellation, automatic timing, full focus/VoiceOver, additional
scales/windows and measured resources remain open.

Its [automatic-policy follow-up](../evidence/carousel-track-macos-och41.md#automatic-pauseresume-follow-up--2026-10-08)
now passes five actual pause conditions, fresh native intervals and page-retired
timers in both binaries. Gesture capture and the remaining physical/release scope
stay open.
