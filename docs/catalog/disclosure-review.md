# Accordion and collapsible source review

OCH-41, 2026-10-02. Exact unmodified base/component `accordion.rs` and
`collapsible.rs` from GPUI Kit `84f57fdfcb4910623fb0bb7f795b077e249f9271` are saved
as four snapshots under `sources/`, SHA256-recorded in the manifest. This review
extends OCH-37's accepted baseline; it does not reopen that historical ticket or
declare OCH-41 complete.

| Pinned behavior | GPUIO mapping |
| --- | --- |
| Controlled open state; single/multiple accordion | `Disclosure` with stable IDs and ordered latest-model Toggle/Expand/Collapse requests. Required-single mode is also available. |
| Disabled group and item | Application model policy and native disabled triggers; historical expanded disabled items may remain open. |
| Trigger/panel and optional mounting | `View.disclosure`, `View.accordion`, explicit `Content_policy.Retain/Unmount`, native expanded state and focus restoration. |
| Heading role/level around the trigger | New `View.accordion_with_labels` wraps each native toggle in validated Heading 1..6, default three. Plain legacy helpers retain their previous structure. |
| Icons and arbitrary title elements | Passive rich labels in `accordion_with_labels`, one native toggle per item, accessible Choice name. Interactive auxiliary header content uses `disclosure_with_header` with a dedicated toggle. |
| Bordered group/item, size presets, title hover and content style | Ordinary group/trigger/panel styles and per-item outer style; label styles own typography/icons. Exact upstream size presets and default chevron assets are not separate APIs. |
| Non-content children of Collapsible stay visible | Ordinary container/header plus `View.panel` or `View.disclosure_with_header`; the application controls each region explicitly. |
| Reversible measured reveal in styled accordion and optional collapsible `motion_id` | `Disclosure.Motion.standard` / custom validated spring on all panel/disclosure helpers. Native measured reversal, immediate settled layout, reduced-motion and lifetime cleanup. Parent-dependent and deferred-style layouts fall back to immediate behavior; see the contract. |
| Keyboard/accessibility | Native grouped arrows/Home/End, Enter/Space/AX, disabled skipping, expanded state and focus return. Current rich-header TestPlatform evidence supplements historical OCH-37 physical coverage; it does not supply new physical release acceptance. |

The [presentation contract](../design/disclosure-presentation.md) records rich
label bounds, ownership, measured reveal and explicit layout limits. The gallery now
shows richer titles, optional/required/multiple expansion, disabled group/item
an Animate toggle and a real native draft with explicit retention policy. The
[evidence](../evidence/disclosure-presentation-och41.md) records actual coverage.
Physical macOS visuals/input/IME/VoiceOver, resources and final release checks
remain open; local TestPlatform checks do not establish release acceptance.
