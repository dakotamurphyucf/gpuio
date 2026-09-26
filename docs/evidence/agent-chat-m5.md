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

## Managed source explorer

**Workspace → Explore sources** uses the public `Gpuio_bonsai.Tree` and
`Gpuio_eio.Tree_loading` APIs. [Source_data](../../examples/agent_chat/runtime/source_data.ml)
provides pure synthetic data and validated move policy;
[Sources](../../examples/agent_chat/runtime/sources.ml) owns the window-scoped
loader, approval state and view. The loader is created in the window factory,
before graph evaluation. Its data and the managed tree's preferences survive
page changes; native views unmount independently. Row eviction does not cancel
the data scope. Branch collapse cancels unfinished child work; window cancellation
closes the loader and any fixture-building task.

Normal startup has three collections. Project sources load three leaves; Research
notes deliberately fail their first attempt and append two leaves after explicit
retry. Loading uses a deterministic 300 ms scoped Eio delay, never real file I/O.
The large fixture contains exactly 100,000 nodes: three roots and 99,997 leaves.
Construction runs through `Eio.Domain_manager.run` in a scoped task; adopting the
result resets the widget generation. Resetting the sample cancels a pending build
and clears its approval dialog. The widget has fixed 34-pixel rows and a 24-row
active budget. Large-data startup is an explicit action.

Native drag and context actions propose moves. A tokened alert dialog consumes an
approval once and rechecks the latest loader snapshot and tree preferences.
Cancelled, reset-stale and incompletely loaded destinations cannot mutate the
sample. IDs and payloads survive accepted moves. Enter reads a source's sample
note; reveal controls navigate to the main source or the final generated leaf.
An empty workspace uses the public presentation empty-state/restore composition.

The native token theme now follows the chat palette through `Window.set_theme`.
The softer accent surface keeps selected-row text legible in both themes. Actual
captures exposed overly wide action labels; compact row-local Actions menus leave
the source names readable. No image editing or mockups were used:

- [Dark source explorer](../images/studio-sources-dark.png)
- [Light source explorer](../images/studio-sources-light.png)

Local macOS commands **PASS**:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/agent_chat/main.exe \
  @test/agent_chat_showcase/runtest @fmt
python3 scripts/test_agent_chat_sources.py
python3 scripts/test_agent_chat_review.py
python3 scripts/test_agent_chat_diagram.py
_build/default/examples/agent_chat/main.exe --self-test
python3 scripts/test_agent_chat.py
```

The source AppKit walkthrough covers accessible selection, native Enter, typeahead,
Shift+Down range selection, lazy failure/retry, owner-checked native drag/cancel,
context-menu move/approval, reveal, page/inspector state retention, 100,000-node
loading and last-row reveal, empty/reset, collapse during loading, successful
re-expansion, both themes, preserved composer draft and OS close/discard. It
observed **9 native accessibility rows** at the final source in the large fixture,
below the configured budget of 24. The script has a 150-second total deadline and
always closes/reaps its child. Set `GPUIO_SCREENSHOT_DIR` to reproduce the captures.

Expect tests cover fixture size, explicit retry after failure, stable moved-node
identity and rejection of reset-stale/incomplete move destinations. All previous
review, diagram and M4 public/AppKit regressions pass after the theme change. The
latest M4 diagnostic records 3,924 ms, 1,676 turns, 236 clock ticks, 58 commits,
37 rendered acknowledgements and 37 completed jobs. This is still the M4 regression
workload; simultaneous streaming/tree/table/canvas/extension latency and retention
measurements remain required. Hosted execution and merge remain pending.

## Remaining acceptance

These flows cover the OCH-23 package integration, OCH-24 diagram, the OCH-37
navigation stack/breadcrumbs, OCH-38 source explorer and a subset of OCH-33's
presentation compositions. They do not complete the other component families,
responsive/resizable inspector, motion, results table
and its large fixture, settings/input/date/color/OTP flows, other navigation families/tour, or the
combined streaming/input/retention/idle-traffic workload. Those all remain required
by OCH-46, along with narrow/wide visual acceptance, the full coverage map, hosted
macOS/Linux gates and merge. Full Linux GUI acceptance remains OCH-17.
