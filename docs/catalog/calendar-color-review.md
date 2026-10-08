# Calendar and color-picker source review

OCH-41, 2026-10-02. The eight unmodified snapshots of base calendar/date picker/
color picker and component color picker/time sources come from GPUI Kit
`84f57fdfcb4910623fb0bb7f795b077e249f9271`. Their paths and SHA256 hashes are in
`sources/manifest.json`. This extends the accepted OCH-35/OCH-36 baseline;
it does not establish complete catalog or physical release acceptance.

## Calendar and date picker

| Pinned capability | GPUIO mapping and remaining work |
| --- | --- |
| Civil single date and date range | `Calendar.Selection`, validated years 1..9999, native draft and current constraints. `Range_start` explicitly represents an incomplete range. |
| Disabled date matcher callback | Bounded declarative min/max, dates, ranges and weekdays. Every-day versus endpoints-only range policy is explicit. No synchronous OCaml predicate is called from Rust layout or input. Arbitrary predicates require application-side compilation to supported constraints. |
| First weekday and month/year navigation | `Config.first_weekday`, localized `Labels`, native Days/Months/Years presentations and keyboard navigation. Sunday versus Monday is a configurable default difference. |
| Ambient today and year-range preference | Explicit application-supplied `today` and civil constraints replace the upstream local clock and default +/-50-year navigation range. No time zone is inferred. |
| Several simultaneous months | `Calendar.Appearance.months` displays 1..12 consecutive wrapping panes with one selection/focus owner and a separate native display anchor. Local native tests cover cross-month ranges and unique AX targets; physical qualification remains open. |
| Customized day/header item rendering, theme and size presets | `Calendar.Appearance` exposes bounded cell height, spacing, padding, radius, outline and selected/hover/today/focus/muted theme colors; outer styles remain available. Fully arbitrary native cell/header render closures are not exposed through the serialized API. `View.Calendar_content` now supplies checked passive per-date/header/navigation content with native semantic targets and default fallback. Op89, atomic admission, retained focus/draft, hidden animation suspension and teardown have local tests. Gallery event badges update live. Independent `Calendar.Viewport` observations now drive bounded grid-date loading through Op95 and generation-checked callbacks; selection/cursor snapshots remain unchanged. Physical qualification remains open. |
| Controlled popup, clear, placeholders and localized formatting | `Gpuio_eio.Date_picker`, explicit app-owned formatted trigger label, ordinary popover placement/dismissal/focus return. Clearing can be a draft preset. `view_with_trigger` supplies checked passive formatted content; the gallery composes an adjacent clear action that cancels then sets the application value. Shared `View.popover` now exposes expanded/dialog-popup state on direct button anchors, with native identity, dismissal, focus, nested-state and gating evidence. Physical validation remains open. The pinned trigger is a display control, not an editable date parser. |
| Labeled single/range presets | `Date_picker.Preset`, bounded unique-ID collection and guarded `Gpuio_eio.Date_picker.select_preset`. Gallery date presets change the open draft. Explicit Apply remains required. |
| Commit on calendar/preset selection | Deliberate difference: the accepted GPUIO picker contract requires explicit Apply. Cancel/Escape/outside dismissal discards the entire opening. It must not copy upstream immediate commit/close behavior accidentally. |
| `component/time` export | `time/mod.rs` exports only calendar/date_picker; this is not evidence of an upstream time-of-day widget. The adjacent utils source is historical helper code, not a separate user-facing family. |

The current six-week civil grid and bounded range arithmetic retain their existing
independent tests. Upstream presentation code is comparison evidence, not a reason
to replace validated date arithmetic or weaken picker session/revision checks.
See [preset contract](../design/date-picker-presets.md) and the original
[calendar evidence](../evidence/calendar-och35.md).

## Color input and picker

| Pinned capability | GPUIO mapping and remaining work |
| --- | --- |
| Optional full-precision color and HSLA channels | `Color_value`, native `Color_input` channels and editors. Achromatic hue survives edits; hex serialization does not quantize slider state. Alpha and empty-value policies are explicit. |
| Hex draft and commit | Native Rust-owned hex editor with selection/composition; invalid/incomplete text does not replace the valid value. |
| Palette selection and selected swatch | Labeled palette entries, native keyboard/focus/AX routing, selected state and transparency checkerboard. Existing limit is 256 entries. |
| Featured colors plus grouped built-in palette | `Palette_section.featured/group` and `Config.palette_sections` group up to 256 native slots without replacing their owner. The gallery shows favorites and nine labeled color families; applications supply their own localized labels/colors. Duplicate values remain distinct focusable slots. Native grouping/size/draft/AX and atomic admission checks are recorded in the [presentation evidence](../evidence/color-presentation-och41.md). |
| Palette/HSLA tabs | `Color_input.Panels.all/tabs` selects simultaneous or native Palette/Channels presentation, retaining one model and five editors. Hex remains visible; hidden channel edits follow blur policy and drags cancel. One roving tab stop, arrow/Home/End, direct AX actions, policy gates and owner teardown have TestPlatform evidence. Gallery inline/popup examples use tabs. Physical acceptance remains open. |
| Hover preview | Native palette hover now displays a transient swatch and separate reserved hex caption. It preserves editor text/selection/composition/history, snapshot and application value, and emits no edit events. Policy/lifetime/capture guards and native TestPlatform checks are recorded in the [contract](../design/color-palette-preview.md) and [evidence](../evidence/color-palette-preview-och41.md). Physical validation remains open. |
| Theme, size, label/icon and popup anchor | Ordinary outer style/popover configuration and checked rich triggers support formatted swatches/captions. `Color_input.Appearance` now supplies bounded swatch/featured/channel geometry, gaps, padding, radii and themed selection/hover borders with retained native ownership. Shared `View.popover` provides expanded/dialog-popup state on direct button anchors; physical rendering/AX acceptance remains open. |
| Selecting swatch or committing hex closes; slider changes commit in place | Deliberate difference: GPUIO popup uses a draft and explicit Apply for all edits. Its inline input still reports Preview/Committed/Cancelled gestures. |

The ledger remains open for these gaps and gallery behavior. Source review and
controller tests are not physical macOS input/VoiceOver/GPU evidence, and no new
Linux desktop acceptance is implied. See the original
[color input evidence](../evidence/color-inputs-och36.md).

## Installed follow-up — 2026-10-05

[The installed picker walkthrough](../evidence/installed-picker-presets-och41.md)
now qualifies live event-content target retention, date draft presets and guarded
Apply/Cancel/Escape/read-only behavior, direct date selection, popup color panel
retention and explicit commit/cancel, clear actions and trigger expansion/focus.
It also reproduces and repairs a native nonmodal focus-context loss. This narrows
the historical physical gaps above; it does not qualify multi-month layout,
calendar/panel physical navigation, hover-preview pixels or the whole family.
VoiceOver qualification remains pending. The owner lifted the earlier testing
hold on 2026-10-05; actual screen-reader evidence is still required.

[A further installed walkthrough](../evidence/installed-calendar-color-och41.md)
qualifies 1/2/3/12-pane logical viewports and unique date targets against an
independent grid oracle, actual cross-month range keys, month/year navigation,
selection/owner retention and remount. It also checks 27 palette-preview GPU
cases, invalid draft/history retention, policy cancellation, two themes/three
sizes and actual roving Palette/HSLA keyboard navigation. Full-pane pixel geometry,
all native channel gestures, broader constraints/locales and release performance
remain separate; earlier TestPlatform-only evidence is not reclassified as OS work.
