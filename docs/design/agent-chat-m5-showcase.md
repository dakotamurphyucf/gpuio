# Milestone 5 agent-chat showcase

Status: implementation plan for OCH-46, reconciled with the live OCH-23–26 and
OCH-33–39 scopes on 2026-09-26. **The review inspector is implemented; the remaining
flows and combined acceptance below are pending.** See [current evidence](../evidence/agent-chat-m5.md).
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
| OCH-24 retained canvas | `Canvas`, `Canvas_scene`, `View.canvas` | Run diagram: select, drag and pan/zoom using accepted native policies; selected-object details and an accessible text alternative describe the same scene. |
| OCH-25 springs | `Animation.Spring` and typed animation specs | Inspector/detail reveal with interrupted retargeting; reduced motion reaches the correct state immediately. |
| OCH-25 sequences and synchronized repetition | `Animation` sequence/repeat/group APIs | Ordered artifact reveal and synchronized active-run indicators; completion, hiding and teardown stop native work. No per-frame OCaml updates. |
| OCH-26 container breakpoints | `Container_query`, `View.container_query` | Inspector and conversation adapt to their assigned split widths in independent windows; hidden alternatives are inert and preserve only their declared state. |
| OCH-33 avatar/fallback | `Avatar`, `View.avatar` | Participant/contributor identity, including a deterministic unavailable-image fallback. |
| OCH-33 badge/tag/marker/label | `Presentation` | Run status, removable result filters and labelled artifact metadata; status/filter changes affect displayed results. |
| OCH-33 link/separator | `Presentation` | In-app source/artifact navigation and clear section boundaries; no network dependency. |
| OCH-33 group box/settings groups/description list | `Presentation` | Generation settings and structured run details using shared themed compositions. |
| OCH-33 empty state/alert/banner | `Presentation` | Empty filters, deterministic tool failure/retry and clearly labelled simulation notices. |
| OCH-33 skeleton/shimmer/spinner | `Loading`, `View.loading` | Loading sources/artifacts with bounded native activity, static reduced-motion alternatives and no wakeups after hiding or completion. |
| OCH-33 shortcut/status bar | `Presentation.shortcut_label`, `status_bar` | Discoverable inspector commands and actual run/loading/selection status. |
| OCH-33 attachment/message/bubble/tool-result cards | `Presentation` card constructors | Existing conversation and artifact cards adopt public helpers while retaining Markdown/code/diff behavior and meaningful actions. |
| OCH-33 form field/help/error | `Form.field`, accessibility field associations | Settings labels, constraints and validation errors refer to their actual controls. |
| OCH-33 rating | `Rating`, `View.rating` | Keyboard-accessible local response feedback, explicitly stored in this demo's in-memory state. |
| OCH-34 numeric and stepper | `Number_input`, `Number_input.Step_controls` | Bounded output-token/count settings with editable invalid/partial drafts, commit/cancel and increment/decrement. Stepper is a presentation of number input, not a separate module. |
| OCH-34 single/range sliders | `Slider` | Generation temperature and result-score interval; filtering uses the accepted interval, and final/cancel behavior is visible. |
| OCH-34 OTP | `Otp_input` | Labelled simulated connection flow with paste, navigation, clear/reset and completion; no authentication claim or external service. |
| OCH-35 calendar | `Calendar` | Inline civil-date/range run filter with disabled/bounded dates and an empty/partial state. |
| OCH-35 popup date picker | `Date_picker` and Bonsai presenter | Simulated scheduled follow-up settings with confirm/cancel and restored focus; date-only values never become timezone-dependent timestamps. |
| OCH-36 swatches/channels/picker | `Color_value`, `Color_input`, `Color_picker` and Bonsai presenters | Diagram annotation color and alpha with preview, commit, cancel and reset; malformed channel drafts are visible and theme tokens remain separate from selected colors. |
| OCH-37 collapsible/accordion | `Disclosure`, public panel/accordion constructors | Tool details plus single/multiple settings sections; demonstrate retained draft versus unmounted lazy content deliberately. |
| OCH-37 navigation stack/breadcrumbs | `Navigation_stack`, `Navigation.breadcrumbs` | Artifact overview → run → item, with back/forward/replacement and a current breadcrumb. |
| OCH-37 sidebar | `Sidebar` | Grouped artifact/source destinations with independent expansion and collapse modes. |
| OCH-37 pagination | `Pagination`, `Navigation.pagination` | Bounded artifact gallery/result summaries; changing filters clamps the current page. Do not replace the virtual transcript/table with pagination. |
| OCH-37 sheet/drawer | `Sheet`, `View.sheet` | Settings/inspection surface with resize-safe placement, dismissal and focus restoration. |
| OCH-37 alert dialog | `Alert_dialog`, `View.alert_dialog` | Confirm clearing local review/settings data; cancel preserves it and nested-modal focus stays correct. |
| OCH-37 hover card | `Hover_card`, `View.hover_card` | Contributor/source preview reachable by pointer and keyboard, with purposeful navigation. |
| OCH-37 carousel | `Carousel`, `View.carousel` | Attachment/artifact preview with manual navigation and opt-in tour auto-advance honoring focus, hidden state and reduced motion. |
| OCH-38 managed tree | `Gpuio_bonsai.Tree`, Eio tree loading | Lazy in-memory workspace/source explorer, loading/failure/retry, selection/reveal/typeahead/context actions and explicit approval for proposed sample moves. No mutation of real files. |
| OCH-39 read-only table | `Gpuio_bonsai.Table`, `Gpuio_eio.Table_paging` | Run results with stable selection, resize/reorder/pinning, sorting, copy/context actions and cancelled obsolete queries. An explicit large-data action loads 100,000 rows; normal startup stays small. |

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
