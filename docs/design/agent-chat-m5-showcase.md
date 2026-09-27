# Milestone 5 agent-chat showcase

Status: implementation plan for OCH-46, reconciled with the live OCH-23–26 and
OCH-33–39 scopes on 2026-09-26. **The review inspector, run diagram, artifact
history, source explorer, results table, generation/connection/date/color settings,
review feedback, artifact tour, grouped destination sidebar transcript/query presentation and richer motion are implemented. Responsive layouts and combined acceptance are
pending.** See [current evidence](../evidence/agent-chat-m5.md).
Existing component examples and library tests do not establish chat integration.
Finish OCH-39 and the integrated acceptance before closing this milestone.

## Product layout and ownership

Keep the conversation, composer and streaming transcript central. Add a
discoverable **Explore workspace** action opening an artifact inspector beside
the conversation. Its sources, run results, diagram and review views should feel
like useful parts of one agent workspace. Put generation, appearance and the
explicitly simulated connection/schedule flows in a settings sheet. A short
feature tour links to these same working surfaces rather than creating duplicate
controls disconnected from application state.

Reuse the existing workspace, tabs, splits, window ownership and palette. Share
conversation/run data through the existing conversation scope; keep inspector
navigation, selected artifact and settings drafts per window. Keyed virtual cells
observe data; their eviction must not cancel conversation or data tasks. Native
hidden-content policy and Bonsai activation remain separate. Retained editors
keep drafts, while explicitly unmounted views release their controllers/tasks.

Extract focused modules beside `examples/agent_chat/runtime/workspace.ml` for
inspector, settings and fixture state, with `.mli` contracts. Avoid accumulating
every feature's state and implementation in `workspace.ml`. Use public Core,
Bonsai and Eio APIs; integration defects belong in the library with regression
coverage. There must be no private bridge path or synchronous OCaml render call.

## Component coverage matrix

Each row is a required working flow. The API column is the implementation entry
point, **not evidence that the chat already uses it**. Implementation must add
exact chat source links and passing walkthrough/evidence links to every row.

| Ticket / family | Public API entry point | Planned reachable flow and observable behavior |
| --- | --- | --- |
| OCH-23 static extension | `Gpuio_example_counter`, `View.extension`; [chat Review](../../examples/agent_chat/runtime/review.ml), [Inspector](../../examples/agent_chat/runtime/inspector.ml) and generated consumer backend | Implemented review checkpoint counter: pointer/OS keyboard events, typed step properties and acknowledged resets; actual 0–100 count and 1–10 step limits. [Public test](../../scripts/test_agent_chat_review.py), [local evidence/screenshots](../evidence/agent-chat-m5.md). Combined workload and hosted gates still pending. |
| OCH-24 retained canvas | `Canvas`, `Canvas_scene`, `View.canvas`; [Diagram](../../examples/agent_chat/runtime/diagram.ml), [Run_diagram](../../examples/agent_chat/runtime/run_diagram.ml) | Implemented simulated run diagram: native select/drag/pan/zoom, stage activation, selected-object coordinates and accessible stage descriptions. Window-scoped scene survives page changes. [Actual macOS walkthrough](../../scripts/test_agent_chat_diagram.py), [evidence](../evidence/agent-chat-m5.md). Combined workload still pending. |
| OCH-25 springs | `Animation.Spring` and typed animation specs | Implemented stage-context spring reveal via [Chat_motion](../../examples/agent_chat/runtime/chat_motion.ml) and [Inspector](../../examples/agent_chat/runtime/inspector.ml). [Full/Reduce native walkthrough](../../scripts/test_agent_chat_motion.py) measures actual 112-pixel geometry, interruption, hidden context and keyboard/navigation/remount behavior. |
| OCH-25 sequences and synchronized repetition | `Animation` sequence/repeat/group APIs | Implemented two-stage staggered workspace destination reveals and shared native response activity in the header/composer through [Chat_motion](../../examples/agent_chat/runtime/chat_motion.ml). [Chat walkthrough](../../scripts/test_agent_chat_motion.py) checks real active-response/input/cancel flows; [native program evidence](animation-programs.md) covers timeline/group semantics. Combined chat idle/bridge/resource measurements remain pending. |
| OCH-26 container breakpoints | `Container_query`, `View.container_query` | [Responsive](../../examples/agent_chat/runtime/responsive.ml) selects compact conversation actions, sidebar branding and inspector heading/navigation from assigned native widths. The inspector uses a stable retained split; closing it expands the conversation without replacing its editor. [Native AppKit walkthrough](../../scripts/test_agent_chat_responsive.py) covers pointer/keyboard resizing, hidden alternatives, draft retention, independent windows, themes and Full/Reduce. |
| OCH-33 avatar/fallback | `Avatar`, `View.avatar` | Implemented contributor portrait/initials with real scoped SVG decoding and deliberately malformed PNM fallback, restoration and remount reuse. [Contributor_portrait](../../examples/agent_chat/runtime/contributor_portrait.ml), [native walkthrough](../../scripts/test_agent_chat_tour.py), [evidence](../evidence/agent-chat-m5.md). No network image dependency. |
| OCH-33 badge/tag/marker/label | `Presentation` | Implemented labelled metadata, Ready markers and removable full-query score filters in [Results](../../examples/agent_chat/runtime/results.ml) and [Chat_message](../../examples/agent_chat/runtime/chat_message.ml). [Native walkthrough](../../scripts/test_agent_chat_presentation.py). |
| OCH-33 link/separator | `Presentation` | Implemented contributor hover-card link navigates to the actual Sources route; feedback uses a section separator. [Review_feedback](../../examples/agent_chat/runtime/review_feedback.ml), [native walkthrough](../../scripts/test_agent_chat_feedback.py). No network dependency. |
| OCH-33 group box/settings groups/description list | `Presentation` | Generation/connection settings now use `Presentation.settings_group` in [Settings](../../examples/agent_chat/runtime/settings.ml). Scheduled reviews use `Presentation.description_list` in [Schedule_settings](../../examples/agent_chat/runtime/schedule_settings.ml). Review checkpoint and feedback cards use `Presentation.group_box` in [Review](../../examples/agent_chat/runtime/review.ml) and [Review_feedback](../../examples/agent_chat/runtime/review_feedback.ml). |
| OCH-33 empty state/alert/banner | `Presentation` | Implemented empty-filter recovery, query/stream failure alerts with existing retry actions and a local-simulation banner in [Results](../../examples/agent_chat/runtime/results.ml) and [Workspace](../../examples/agent_chat/runtime/workspace.ml). [Native walkthrough](../../scripts/test_agent_chat_presentation.py). |
| OCH-33 skeleton/shimmer/spinner | `Loading`, `View.loading` | Implemented real query skeleton/shimmer and query/fixture/stream spinners through [Query_loading](../../examples/agent_chat/runtime/query_loading.ml). [Full/Reduce walkthrough](../../scripts/test_agent_chat_presentation.py) covers semantics, completion, hide/remount and obsolete-query cancellation. Aggregate idle/traffic measurements remain pending. |
| OCH-33 shortcut/status bar | `Presentation.shortcut_label`, `status_bar` | Feedback demonstrates real rating arrow keys with shortcut labels and accepted rating in a status bar. [Review_feedback](../../examples/agent_chat/runtime/review_feedback.ml), [walkthrough](../../scripts/test_agent_chat_feedback.py). Result selection now uses a status bar/marker; query and streaming loading follow their existing state. [Presentation walkthrough](../../scripts/test_agent_chat_presentation.py). |
| OCH-33 attachment/message/bubble/tool-result cards | `Presentation` card constructors | The [artifact tour](../../examples/agent_chat/runtime/artifact_tour.ml) uses public attachment cards linking to real destinations. Existing user/assistant/artifact rows now compose public message, bubble and tool-result cards through [Chat_message](../../examples/agent_chat/runtime/chat_message.ml), preserving document ownership. [Native walkthrough](../../scripts/test_agent_chat_presentation.py) checks streaming and actual code/diff expand/collapse/exact Unicode copy; [screenshots/evidence](../evidence/agent-chat-m5.md). |
| OCH-33 form field/help/error | `Form.field`, accessibility field associations | Implemented numeric, single/range-slider and OTP labels/help; incomplete/invalid numeric drafts set associated field errors. [Settings](../../examples/agent_chat/runtime/settings.ml), [native walkthrough](../../scripts/test_agent_chat_settings.py). |
| OCH-33 rating | `Rating`, `View.rating` | Implemented window-local run-usefulness rating: native keyboard/AX requests reduce against the latest accepted state, Clear restores unrated, route/inspector remount retains value. [Review_feedback](../../examples/agent_chat/runtime/review_feedback.ml), [walkthrough](../../scripts/test_agent_chat_feedback.py), [evidence](../evidence/agent-chat-m5.md). |
| OCH-34 numeric and stepper | `Number_input`, `Number_input.Step_controls` | Implemented real simulated stream chunk-size settings (4–128 bytes): native partial/invalid drafts, Return commit, Escape restore and side/stacked/keyboard-only steppers. [Settings](../../examples/agent_chat/runtime/settings.ml), [generation model/tests](../../test/agent_chat_showcase/settings_test.ml), [native walkthrough](../../scripts/test_agent_chat_settings.py). Bytes are explicitly not LLM tokens. |
| OCH-34 single/range sliders | `Slider` | Implemented actual simulated stream interval (10–200 ms) and inclusive result-score interval; Apply filters the complete result query. Native preview/commit/cancel are visible, accepted values survive reopening. [Settings](../../examples/agent_chat/runtime/settings.ml), [Results](../../examples/agent_chat/runtime/results.ml), [native walkthrough](../../scripts/test_agent_chat_settings.py). |
| OCH-34 OTP | `Otp_input` | Implemented Connection demo: native six-digit editor, normalized clipboard paste, partial-code retention, clear/reset and explicitly simulated completion. [Settings](../../examples/agent_chat/runtime/settings.ml), [native walkthrough](../../scripts/test_agent_chat_settings.py). No service or authentication claim. |
| OCH-35 calendar | `Calendar` | Implemented October 2026 review filter: native single/range selection, partial drafts, disabled weekdays/dates, bounded endpoints and explicit Apply. [Schedule_settings](../../examples/agent_chat/runtime/schedule_settings.ml), [native walkthrough](../../scripts/test_agent_chat_dates_colors.py), [evidence](../evidence/agent-chat-m5.md). |
| OCH-35 popup date picker | `Date_picker` and Bonsai presenter | Implemented simulated follow-up with confirm/cancel/Escape, restored trigger focus and window-owned accepted civil date. Page deactivation cancels popup drafts. [Schedule_settings](../../examples/agent_chat/runtime/schedule_settings.ml), [walkthrough](../../scripts/test_agent_chat_dates_colors.py). No scheduling service or timestamps. |
| OCH-36 swatches/channels/picker | `Color_value`, `Color_input`, `Color_picker` and Bonsai presenters | Implemented diagram connector annotation: palette/hex/HSLA editing, isolated preview, Apply/cancel/reset and invalid drafts. Concrete RGBA and alpha persist across themes; Empty follows the theme accent. [Annotation_settings](../../examples/agent_chat/runtime/annotation_settings.ml), [walkthrough](../../scripts/test_agent_chat_dates_colors.py), [scene tests](../../test/agent_chat_showcase/diagram_test.ml). |
| OCH-37 collapsible/accordion | `Disclosure`, public panel/accordion constructors | Implemented private review-note disclosure (`Retain`) and single/multiple review-guidance accordion (`Unmount`). Collapse keeps native draft/focus behavior; route remount uses the observed note as initial text. Collapsed guidance content is not built. This makes no claim of deactivating a Bonsai task. [Review_feedback](../../examples/agent_chat/runtime/review_feedback.ml), [walkthrough](../../scripts/test_agent_chat_feedback.py). |
| OCH-37 navigation stack/breadcrumbs | `Navigation_stack`, `Navigation.breadcrumbs`; [Inspector](../../examples/agent_chat/runtime/inspector.ml) | Implemented overview → run → stage, back/forward, next-stage replacement and current breadcrumbs. History is bounded, inactive native pages unmount while window data persists. [Actual walkthrough](../../scripts/test_agent_chat_diagram.py), [evidence](../evidence/agent-chat-m5.md). Other navigation families remain below. |
| OCH-37 sidebar | `Sidebar`; [Artifact_sidebar](../../examples/agent_chat/runtime/artifact_sidebar.ml), [Inspector](../../examples/agent_chat/runtime/inspector.ml) | Implemented grouped artifact/context destinations sharing inspector routes and history. Independent branch expansion, SVG icon mode, retained offcanvas inertness/restoration, keyboard/pointer navigation and themes pass in the [native walkthrough](../../scripts/test_agent_chat_sidebar.py). [Evidence/screenshots](../evidence/agent-chat-m5.md). |
| OCH-37 pagination | `Pagination`, `Navigation.pagination` | Implemented scheduled review pages (six records/page): date filtering preserves or clamps the page, with first/previous/next/last navigation. [Schedule_settings](../../examples/agent_chat/runtime/schedule_settings.ml), [walkthrough](../../scripts/test_agent_chat_dates_colors.py). The transcript/table remain virtualized. |
| OCH-37 sheet/drawer | `Sheet`, `View.sheet` | Implemented modal Settings sheet with page changes, close/reopen and native dismissal. [Settings](../../examples/agent_chat/runtime/settings.ml), [walkthrough](../../scripts/test_agent_chat_settings.py). Resize/narrow-layout and additional focus acceptance remain pending. |
| OCH-37 alert dialog | `Alert_dialog`, `View.alert_dialog` | Implemented nested confirmation for resetting generation preferences; cancel preserves values, confirm resets stream settings while preserving score filters. [Settings](../../examples/agent_chat/runtime/settings.ml), [walkthrough](../../scripts/test_agent_chat_settings.py). |
| OCH-37 hover card | `Hover_card`, `View.hover_card` | Implemented contributor preview: keyboard focus or delayed pointer hover opens it, Escape restores trigger focus, and its public Link opens Sources. Inactive feedback closes accepted preview state. [Review_feedback](../../examples/agent_chat/runtime/review_feedback.ml), [native walkthrough](../../scripts/test_agent_chat_feedback.py), [evidence](../evidence/agent-chat-m5.md). |
| OCH-37 carousel | `Carousel`, `View.carousel` | Implemented four-stop workspace tour with public attachment cards and real Sources/Results/Diagram/Feedback navigation. Manual selection, native keyboard/current semantics, opt-in four-second timing, focus/hover pauses, inspector hide/remount and reduced-motion policy pass locally. [Artifact_tour](../../examples/agent_chat/runtime/artifact_tour.ml), [native walkthrough](../../scripts/test_agent_chat_tour.py), [evidence](../evidence/agent-chat-m5.md). |
| OCH-38 managed tree | `Gpuio_bonsai.Tree`, Eio tree loading; [Sources](../../examples/agent_chat/runtime/sources.ml), [Source_data](../../examples/agent_chat/runtime/source_data.ml) | Implemented lazy sample explorer: native selection/range/typeahead/reveal, first-load failure/retry, collapse cancellation, drag/context move proposals with explicit approval, empty/reset and optional 100,000-node fixture. No real files are changed. [AppKit walkthrough](../../scripts/test_agent_chat_sources.py), [model tests](../../test/agent_chat_showcase/source_test.ml), [evidence](../evidence/agent-chat-m5.md). Combined streaming/tree/table/canvas workload remains pending. |
| OCH-39 read-only table | `Gpuio_bonsai.Table`, `Gpuio_eio.Table_paging`; [Results](../../examples/agent_chat/runtime/results.ml), [Result_data](../../examples/agent_chat/runtime/result_data.ml), [Result_actions](../../examples/agent_chat/runtime/result_actions.ml) | Implemented read-only run findings: native selection/activation/context/reveal/copy, resize/reorder/pinning/reset, full-query sorting/filtering, paging/failure/retry/cancelled obsolete query, empty/reset and opt-in 100,000 rows. [AppKit walkthrough](../../scripts/test_agent_chat_results.py), [model tests](../../test/agent_chat_showcase/result_test.ml), [local evidence/screenshots](../evidence/agent-chat-m5.md). Combined workload and hosted gates remain pending. |

## Validation and completion evidence

Preserve the existing [agent-chat walkthrough](../../scripts/test_agent_chat.py)
and [M4 evidence](../evidence/agent-workspace-m4.md): streaming, composer draft and
IME behavior, managed transcript/history, Markdown/code/diff, commands, tabs,
independent windows, picker flows and cleanup. These are regression requirements,
not candidates for replacement with a simpler showcase application.

Add deterministic success/loading/empty/invalid/failed/disabled/cancelled paths
where applicable. Exercise actual macOS keyboard, pointer, focus and accessibility
through the integrated application, including nested overlays, theme switching,
split resizing and reduced motion. Capture and inspect actual light/dark and
narrow/wide screenshots, iterate on typography/spacing/composition, then update
the demo's versioned screenshots and usage documentation.

Measure a simultaneous run with streaming and responsive composer input while
the large tree/table fixtures, canvas and native extension are active. Record
workload sizes, input latency, retained resources/queues and bridge traffic;
show that native animation does not create per-frame OCaml work. Repeated view
mount/unmount and window close must release owned resources and leave no test
processes or windows behind.

Only fill acceptance links after those exact flows pass. Required hosted macOS
and Linux build/unit gates and merge remain necessary. Full Linux GUI validation
stays in OCH-17; this plan adds no Windows support, dynamic plugins, hot reload,
real scheduling service or milestone-6 desktop services. OCH-29 remains the
separate broader graphics/independent-extension acceptance application.
