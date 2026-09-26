# Milestone 5 chat integration evidence

Status: **in progress**. This ledger records accepted integrated flows, not
milestone completion. The entire [component coverage matrix](../design/agent-chat-m5-showcase.md)
remains required. Separate library examples do not establish chat integration.

## Review inspector and packaged native component

**Explore workspace** in the conversation toolbar, Workspace menu or command
palette opens the artifact inspector. Its first implemented flow is window-local
review progress. The conversation remains mounted beside it; closing the inspector
uses `Content_policy.Unmount` for native children while retaining the Bonsai model.
It does not replace the existing transcript, composer, tabs or streaming runtime.

Source and ownership:

- [Inspector](../../examples/agent_chat/runtime/inspector.ml) owns window visibility;
  [Review](../../examples/agent_chat/runtime/review.ml) owns the count, step, pending
  command and observation status through ordinary Bonsai state.
- The executable explicitly selects `gpuio_counter_backend`, the generated backend
  from [the independent consumer](../../examples/extension_consumer/backend/dune).
  The runtime imports the separate package's public `Gpuio_example_counter` API.
  No private bridge endpoint or synchronous OCaml render callback is involved.
- Native pointer and keyboard activation emit bounded typed events. Step choices
  update properties. Reset sends a sequenced native command; **Review reset** is
  displayed only after its matching acknowledgement. The acknowledged command is
  removed, preventing it from replaying when the native component remounts.
- Values remain within the package's 0–100 count and 1–10 step limits. The current
  UI offers steps 1 and 5. This is explicitly local window review progress, not a
  conversation job or persisted account setting.

The actual macOS test found duplicate Space activation in the independent sample:
GPUI's `on_click` already synthesizes a keyboard click on Enter/Space release, while
the sample also incremented in `on_key_down`. The package now uses the click path
once, distinguishes `ClickEvent::Mouse` for pointer guards, and retains its explicit
accessibility action. The public chat test protects the exact one-key/one-increment
behavior and exercises the real package rather than a replacement fixture.

Local macOS commands **PASS**:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/agent_chat/main.exe @fmt
GPUIO_JOBS=2 python3 scripts/test_agent_chat_review.py
_build/default/examples/agent_chat/main.exe --self-test
python3 scripts/test_agent_chat.py
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j2 \
  -p gpuio-example-counter --all-targets -- -D warnings
```

The new script checks pointer increment, a single OS Space increment, step changes,
two acknowledged resets, hide/reopen before and after reset, disappearance of hidden
native accessibility objects, preserved composer draft, theme switching and native
close/discard. It addresses keyboard input only to its child PID, verifies pointer
ownership, has a total 120-second deadline and closes/reaps its process.

Both existing M4 acceptance suites remain passing with the generated backend.
The public run records 3,951 ms, 1,623 runtime turns, 237 clock ticks, 55 commits,
35 rendered acknowledgements and 37 completed jobs for its established workload.
Those are regression diagnostics, **not** the required simultaneous M5 workload or
a release performance benchmark. The external regression also passes native
search/Send/Return, retry, retained tabs, independent windows, picker/Eio attachment,
themes, command palette and OS close deny/allow.

## Visual iteration

Actual 1180×820 logical-pixel application windows were captured and inspected in
both themes. The first pass exposed mismatched default blue controls and low
contrast. The accepted iteration uses explicit palette surfaces/borders/text,
an intrinsic review badge and a secondary reset control. Capture waits briefly
for the compositor after AX observes a theme update; an immediate capture could
miss the native card caption in the transitional frame. No image editing or
mockup was used.

- [Dark review inspector](../images/studio-review-dark.png)
- [Light review inspector](../images/studio-review-light.png)

Reproduce the screenshots by setting `GPUIO_SCREENSHOT_DIR=/absolute/output/path`
when running `scripts/test_agent_chat_review.py`. The original M4 screenshots
remain the base conversation reference.

## Run diagram and artifact history

The inspector now includes Workspace → Run → Stage navigation using public
`Navigation_stack` and `Navigation.breadcrumbs`. Back/forward, current-location
semantics and next-stage replacement use the same history. Visits are bounded to
32; new navigation at the bound starts from Workspace, while replacement keeps
the current path. At most four breadcrumbs are rendered, including a disabled
ellipsis for omitted visits. Selecting the already-current destination is a no-op.

[Run_diagram](../../examples/agent_chat/runtime/run_diagram.ml) owns a pure,
three-stage simulated run with validated translations and five scene items/five
resources. [Diagram](../../examples/agent_chat/runtime/diagram.ml) lazily registers
one scene through `Gpuio_eio.Canvas`, using the exact window's scope. The scene
persists across inactive/unmounted pages and inspector closure; window cancellation
releases it. No image assets, network or filesystem access are required. Position,
selection and viewport observations arrive asynchronously; Rust performs native
hit testing, intermediate dragging, pan/zoom and paint without synchronous OCaml
callbacks. Completed movement republishes connector geometry with increasing
resource generations. Text/coordinate descriptions and ordinary selection/detail
buttons provide an accessible alternative to the diagram.

Local macOS commands **PASS** after this integration:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/agent_chat/main.exe \
  @test/agent_chat_showcase/runtest @fmt
python3 scripts/test_agent_chat_diagram.py
python3 scripts/test_agent_chat_review.py
_build/default/examples/agent_chat/main.exe --self-test
python3 scripts/test_agent_chat.py
```

The diagram script exercises native accessible selection, Shift+Right movement,
an owner-checked 20×10-pixel mouse drag, Alt+Right pan (observed through changed
native screen geometry), + zoom, Reset view, Enter activation, stage replacement,
back/forward, an AX breadcrumb, preserved review count/composer draft/moved stage,
unmount/reopen, themes and OS close/discard. Its total deadline is 120 seconds and
all exit paths reap the owned application. Tests wait for native AX eligibility
to settle around scene publication before beginning another gesture: publication
can cancel a gesture under the existing canvas contract. The initial integration
also uncovered a demo reset error: restoring `initial_viewport` means native
`Reset_viewport` would return to that restored value. The UI now explicitly sends
`Set_viewport Viewport.default`, so Reset view always returns to 100% at origin.

Expect tests cover movement isolation, both themed scene constructions, stable
stage IDs and rejection of nontranslation/out-of-margin transforms. Existing
review and both M4 acceptance suites remain green. The latest M4 diagnostic run
records 3,941 ms, 1,641 turns, 236 clock ticks, 58 commits, 37 rendered acknowledgements
and 37 completed jobs; this remains a regression check, not the combined M5 benchmark.

Actual screenshots were inspected in both themes. Explicit breadcrumb palette
styles replace the mismatched defaults; compact history prevents a growing path
from consuming the inspector. These are captures from the running application:

- [Dark run diagram](../images/studio-diagram-dark.png)
- [Light run diagram](../images/studio-diagram-light.png)

Set `GPUIO_SCREENSHOT_DIR` when running the diagram script to reproduce them.
The hosted macOS workflow includes the new walkthrough, but this checkpoint has
not yet run through the consolidated hosted gates or merge.

## Remaining acceptance

These flows cover the OCH-23 package integration, OCH-24 diagram, the OCH-37
navigation stack/breadcrumbs and a subset of OCH-33's
presentation compositions. They do not complete the other component families,
responsive/resizable inspector, motion, source tree/results
and large fixtures, settings/input/date/color/OTP flows, other navigation families/tour, or the
combined streaming/input/retention/idle-traffic workload. Those all remain required
by OCH-46, along with narrow/wide visual acceptance, the full coverage map, hosted
macOS/Linux gates and merge. Full Linux GUI acceptance remains OCH-17.
