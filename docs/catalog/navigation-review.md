# Breadcrumb and pagination source review

OCH-41, 2026-10-02. Exact unmodified GPUI Kit snapshots from
`84f57fdfcb4910623fb0bb7f795b077e249f9271`:
`base-pagination.rs.txt`, `component-pagination.rs.txt` and
`component-breadcrumb.rs.txt` in `sources/`, SHA256-recorded in the manifest.
This supplements OCH-37's accepted baseline; physical release acceptance is open.

| Pinned behavior | GPUIO mapping |
| --- | --- |
| Controlled current/total page and change callback | `Pagination` validated application model and latest-state `Request` reducer. Empty data is supported intentionally; maximum is one billion. |
| Previous/next, first/last and numbered pages | `Navigation.pagination`, stable keys, native button keyboard and accessibility behavior. Current page has separate current-page metadata; focus need not follow selection. |
| Compact previous/next icon controls | `Pagination_layout.Compact` uses arrows and localized accessible labels; no hidden page/gap formatting. |
| Configurable visible page count | Bounded `siblings` 0..4, at most 13 page/gap items and four boundary controls. This differs from upstream's arbitrary visible-page count. |
| Ellipsis dropdown of hidden pages | Optional Core `on_gap`, plus managed `Gpuio_eio.Pagination` popover with at most seven shortcuts and direct native numeric entry. Deliberately avoids upstream's enumeration of the entire interval. Each gap retains its own direct native button and popup wrapper via `Navigation.Gap_popup`. The active gap exposes expanded state, button-relative placement and eligible-trigger focus return; compact/disabled models suppress content. |
| Disabled state and component sizing/styles | Native disabled admission, View styles and Navigation.Appearance. Per-control dimensions use ordinary style properties rather than upstream size presets. |
| Breadcrumb labels, disabled links and optional callback | Stable-ID Choice path; `is_navigable` supports passive intermediate members. Final member is passive current-location text even though upstream permits a callback. |
| Per-item and container breadcrumb styling | Optional pure `item_style` and shared Appearance/container style. Decorative separators use the existing border composition, not the exact upstream ChevronRight glyph. |
| Breadcrumb accessibility | Native links for actions; ordinary text for passive items. Current location metadata is explicit. Upstream's passive ListItem role is not reproduced. |

[Public chooser contract](../design/pagination-chooser.md) records request,
editor, asynchronous completion and lifecycle semantics. The
[evidence](../evidence/pagination-chooser-och41.md) distinguishes automated tests
from physical desktop validation. Styled interactive equivalence is implemented
locally. The [macOS walkthrough](../evidence/pagination-macos-och41.md) now covers
six theme/application-size combinations in root and fresh installed galleries:
breadcrumb keyboard routing, native entry/normalization, Cancel/Escape focus,
model-change dismissal, compact/empty/disabled states, bounded billion-page
shortcuts and retained application model versus retired native chooser. Screenshot
review found and repaired poor dark-theme button contrast; actual button pixel
checks now accompany input checks. Localization, VoiceOver, measured resources,
final-source hosted checks and broader release gates remain open.

Workflow `component/stepper` is covered separately by
[its contract](../design/workflow-stepper.md) and
[evidence](../evidence/workflow-stepper-och41.md); it is not a numeric stepper.
