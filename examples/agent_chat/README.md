# Agent workspace

A runnable OCaml/Bonsai/Core/Eio application using the public GPUIO APIs. It has a
conversation sidebar and search, retained tabs, a resizable sidebar, a paged
transcript, native composers, Markdown/code/diff tool cards, text attachments,
commands, light/dark appearance and independent windows sharing conversations.
The deterministic local backend needs no credentials or network connection.

![GPUIO Studio in dark mode](../../docs/images/studio-dark.png)

[Light appearance](../../docs/images/studio-light.png) uses the same semantic palette and layout.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/agent_chat/main.exe -j 2
_build/default/examples/agent_chat/main.exe
```

Enter sends; Shift-Enter inserts a newline. Send captures the current native
editor contents. Editing while acceptance is pending preserves the newer draft.
Open Demo controls to choose Normal stream, Slow stream or Simulate error before sending; Cancel keeps
partial output and Retry response resets that response. Load older prepends saved
history without moving your reading anchor. Latest returns to the growing tail.
Closing/reopening a conversation tab preserves its draft and reading position.
New window opens the selected conversation with a separate draft and viewport.
Closing a window with drafts asks for a decision; other windows remain usable.

Primary-Shift-P opens the command palette, Primary-N opens another window,
Primary-Shift-[ / ] changes tabs, and Primary-W closes a tab. Primary means
Command on macOS and Control on Linux. The Workspace menu exposes the same
window-scoped commands. Split dividers support pointer resizing and native
keyboard/accessibility actions.

The paperclip (Attach text) opens a native picker, then reads the selected path through Eio in
the originating window's scope. UTF-8 text files up to 64 KiB become transcript
artifacts. `--directory /absolute/path` supplies an optional starting-folder hint. Binary/NUL-containing input and larger files report an error. Markdown
links report a navigation intent in the status area; the demo does not launch a
browser or execute tool output.

The example deliberately bounds retained state: three seeded conversations,
four live windows, three retained panels per window, 64 responses and eight text
attachments per conversation. Each conversation starts with 40 of 200 saved
messages. Conversation history is synthetic and session state is in memory;
closing the application discards it. A real application supplies persistence,
provider integration, credentials and orchestration through its own Eio services.
This example is not a provider client or a storage layer.

## Implementation map

- `model/fake_backend`: pure chunk/delay/failure configuration and fixtures.
- `runtime/icons` and `runtime/palette`: original SVG assets and semantic light/dark colors.
- `runtime/conversation`: shared conversation scopes, paging, response documents,
  bounded attachments, cancellation and retry.
- `runtime/workspace`: one window's tabs, editors, list views, commands and close
  decision; composed entirely through public view/controller APIs.
- `main.ml`: Eio capabilities, window creation and integration acceptance runner.

The [ownership design](../../docs/design/agent-workspace.md) explains why hiding a
row, closing a tab and closing a window have different effects on streaming.

## Validation

```sh
_build/default/examples/agent_chat/main.exe --self-test
python3 scripts/test_agent_chat.py
```

The first opens real native windows and drives public controllers: exact submit,
concurrent streams, protected drafts, cancellation/retry/errors, history anchoring,
retained tabs, independent windows, attachments and pending-send cancellation.
It does not simulate external OS keystrokes. The second is a macOS-only external
accessibility/keyboard test targeting only its child application's PID. It needs
Accessibility permission and reaps its child on success or failure. It exercises
actual native UI activation, Return, the picker, tab/window/theme/palette controls
and OS close decisions. `--native-test` only lengthens fake send acceptance to
make the newer-draft race observable and prints a marker after `App.run` returns.
Linux GUI execution remains informational under OCH-17; Linux build/unit checks
remain required. See [milestone evidence](../../docs/evidence/agent-workspace-m4.md).

## Milestone 5 integration in progress

**Explore workspace** opens the review inspector beside the conversation. Click
its checkpoint card or focus it and press Space/Enter, choose a step of 1 or 5,
and reset the count. Closing/reopening keeps this window's observed progress and
conversation draft while unmounting native inspector content. Progress is in
memory and ends with the window.

The app now links the separate example counter package through its generated
native backend. `runtime/review` consumes only that package's public typed API;
`runtime/inspector` owns visibility. The reset status waits for a native command
acknowledgement rather than treating submission as completion.

Choose **Workspace → Explore run diagram** (or **Diagram**) to inspect a simulated
three-stage run. Select or drag a stage, use Shift+arrows to move it, Alt+arrows to
pan, and +/- or Ctrl+scroll to zoom. Enter opens stage details; Next stage replaces
the current detail visit. Back/Forward and breadcrumbs navigate the same bounded
history. Reset view returns to the default 100% viewport. The scrollable stage
list provides descriptions, coordinates and actions without requiring the graphic.
No actual source files are read or modified by this simulation.

`runtime/run_diagram` holds pure stage geometry; `runtime/diagram` uses the public
scoped canvas API. One scene is registered lazily per window, retained while the
inspector is closed and released with the window scope. Native views unmount on
page changes; diagram positions, viewport and review progress remain window-local.
Returning to the same current destination is a no-op. History keeps at most 32
visits and shows at most four breadcrumbs; at the limit a new visit starts again
from Workspace without discarding application state.

Choose **Workspace → Explore sources** for the in-memory source explorer.
Project sources load lazily; Research notes deliberately fail their first load
so Retry has a reproducible purpose. Select a row, type a filename prefix, use
Shift+arrows for range selection, or reveal the main source. Enter reads its
sample note. Drag a source or choose Actions → Archive source to propose a move;
the confirmation must be accepted before the hierarchy changes. Cancel leaves
the sample unchanged. These are synthetic sources, not files in your checkout.

**Load 100,000 sources** constructs a large fixture in an Eio worker domain; normal
startup contains only three lazy collections. Reveal last source jumps to the end.
Empty workspace shows a restore action, and Sample sources resets the demo.
The window owns its data loader and move approval; changing pages or evicting a
virtual row does not discard the source data. Collapsing a branch cancels its
unfinished child load, and window closure cancels its data scope. Native source
views follow the same app palette in both themes.

Run `python3 scripts/test_agent_chat_review.py` after building for the actual
macOS pointer/keyboard/properties/commands/events and lifetime walkthrough. Set
`GPUIO_SCREENSHOT_DIR` to capture the dark/light views. See the
[partial M5 evidence](../../docs/evidence/agent-chat-m5.md) and
[complete remaining coverage plan](../../docs/design/agent-chat-m5-showcase.md).
Run `python3 scripts/test_agent_chat_diagram.py` for native canvas gestures,
keyboard pan/zoom, route replacement, breadcrumbs, state preservation and theme
switching. It also supports `GPUIO_SCREENSHOT_DIR`. The full milestone showcase
is not yet complete.

`python3 scripts/test_agent_chat_sources.py` validates the source explorer through
actual macOS input, including failure/retry, native drag and approval, collapse
cancellation, large-fixture reveal and bounded accessibility rows. It supports
the same screenshot output variable and always reaps the owned app.

Choose **Workspace → Explore results** for structured sample findings. The initial
query loads 24 rows, with 24 more available. Sort icons reorder the complete query;
resize header boundaries, drag column headers, pin/unpin result IDs, and reset
column preferences. Selection and column preferences survive leaving the page.
Enter, Shift+F10 or a right click opens finding details; Reveal finding selects its
full Unicode summary, which can be copied with the native Copy command.

Score filters include a deliberate empty state with a restore action. **Fail next
query** fails once and needs Retry results. **Slow query** makes cancellation
observable: choosing another query retires the old producer and its delivery.
**Load 100,000 results** opts into a full local dataset built in an Eio worker
domain, keeping startup small. Reveal last result reaches its final record while
the native table mounts at most 24 rows / 96 cells. This is a read-only table;
findings are simulated, never external tool output or files from your checkout.

Large fixture construction (sources and results) allows one active producer and
one newest pending replacement per window/fixture. Reset retires stale delivery;
an already-running pure CPU calculation drains before starting its replacement,
so repeated controls cannot accumulate worker domains. Closing the window cancels
the owning Eio scope. Complete-query reordering preserves surviving row identity;
removal and later reinsertion deliberately start a new membership lifetime.

`python3 scripts/test_agent_chat_results.py` exercises the integrated table on
macOS and supports `GPUIO_SCREENSHOT_DIR`. Its owned process has a total deadline
and is closed/reaped on success and failure. Full M5 acceptance still requires
the other planned families and the simultaneous workload in the coverage matrix.

Choose **Settings** in the top bar (or Workspace commands → Workspace settings)
for window-local generation preferences and an explicitly simulated connection.
Stream chunk size controls actual bytes per simulated response chunk; Return
commits, Escape restores, and side/stacked/keyboard-only steppers share one editor.
The stream interval slider changes delays for future sends. Running streams keep
their captured configuration. Existing demo presets remain explicit overrides.

The score-range slider selects inclusive percentages; **Apply score interval**
filters the complete results query. Open Workspace → Results to inspect it.
Reset generation preferences uses a nested confirmation and preserves result
filters. Closing or changing pages discards uncommitted numeric drafts; accepted
preferences and canonical partial OTP codes survive within this window.

**Connection demo** accepts any six digits, including normalized `123-456` paste.
Completion is purely local: no account, credential, network request or service
connection is created. Clear removes the sample code. The settings walkthrough is
`python3 scripts/test_agent_chat_settings.py`, with the same screenshot variable
and guaranteed owned-process cleanup.

**Dates & reviews** uses a fixed October 2026 civil-date fixture. Select a single
weekday or an interval and Apply to filter the paginated sample reviews. Weekends
and October 20 are unavailable endpoints. A partial interval cannot be applied;
**All review dates** restores the full list. **Choose follow-up date** opens a
separate confirm/cancel picker. Its saved date is a local simulation: nothing is
scheduled or sent, and no timezone conversion occurs.

**Annotation color** previews palette swatches, hex values and HSLA channels.
**Apply annotation** changes the actual run diagram connector strokes; Cancel
keeps the accepted color. Alpha is preserved across light/dark themes.
**Use theme accent** clears the override. Accepted dates/colors belong to this
window; leaving the page cancels an open picker draft. Channel displays use up to
two decimals without rounding the stored model; typed drafts keep their spelling.
Run `python3 scripts/test_agent_chat_dates_colors.py` for the integrated native
walkthrough. Remaining M5 families stay listed in the coverage matrix.
