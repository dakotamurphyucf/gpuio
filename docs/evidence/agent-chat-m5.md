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
retry. Project loading uses a deterministic 300 ms scoped Eio delay; Research notes
uses two seconds so its loading/cancel state is observable through ordinary
native interaction. Neither performs real file I/O.
The large fixture contains exactly 100,000 nodes: three roots and 99,997 leaves.
Construction runs through `Eio.Domain_manager.run` in a scoped task; adopting the
result resets the widget generation. Resetting the sample retires pending build
delivery and clears its approval dialog. The window-scoped fixture helper allows
one running calculation and at most one newest pending replacement. An already
running pure calculation drains before its replacement starts, preventing rapid
resets from accumulating worker domains. Parent scope cancellation still cancels
the Eio task. The widget has fixed 34-pixel rows and a 24-row
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

## Structured run results

**Workspace → Explore results** integrates the public Bonsai table and Eio pager.
[Result_data](../../examples/agent_chat/runtime/result_data.ml) owns immutable
queries, deterministic Unicode findings and global sorting/filtering before
paging. [Results](../../examples/agent_chat/runtime/results.ml) owns one window's
pager, accepted columns, selected view and pending fixture work. The initial
sample loads 24 of 48 records; the explicit large fixture has 100,000 records.
A query replacement builds the full ordered result set, preserving membership
for surviving IDs. A fresh paged sample intentionally starts a new lineage.
Removed/reintroduced IDs cannot revive old row references.

[Result_actions](../../examples/agent_chat/runtime/result_actions.ml) retains a
row reference, query generation and unique dialog identity, rather than payloads
or old data snapshots. Details and delayed reveal recheck the current generation
and membership. Native selection, context targets, horizontal/vertical reveal,
clipboard and column gestures travel through ordinary asynchronous public APIs.
The score filter affects the results; empty output has a restore action. Failure
and slow-query controls provide reproducible retry/cancellation demonstrations,
without real tools, files, services or credentials.

The shared [Fixture_job](../../examples/agent_chat/runtime/fixture_job.ml) helper
bounds CPU fixture work to one running producer and one latest pending request.
Both sources and results use it. A reset retires stale delivery; an already-running
pure calculation finishes before the latest replacement starts. Parent scope
cancellation still cancels its Eio task. A deterministic mocked-Eio test submits
100 replacements while the first producer is held: only the first and latest
start, only the latest is delivered, and peak concurrency is one. Other expect
tests cover whole-query page order, score filtering, exact Unicode, stable row
membership through reorder, retired removal/reinsertion, invalid cursors/sorts
and deterministic ties across 100,000 records.

Local macOS commands **PASS** for the results integration:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/agent_chat/main.exe \
  @test/agent_chat_showcase/runtest @fmt
GPUIO_JOBS=2 python3 scripts/test_table_host.py
python3 scripts/test_agent_chat_results.py
```

The native chat walkthrough validates actual pointer cell selection, OS arrows,
Return/Shift+F10, context targets independent of selection, dismissal/restored
focus, reveal, exact joined-emoji/Japanese Command-C, grouped columns, actual
header-boundary resizing and drag reordering preserved across unmount/remount,
full-query ascending/descending sort, pin/reset, paging, failure/retry, replacing
a two-second pending query with empty results, filters, themes, preserved composer
draft and window close/discard. Large-data last-row reveal observed **7 AX rows
and 32 cells**, within the 24-row/96-cell limits. This is mounted accessibility
evidence, not the still-required combined retention/latency workload. The script
has a 180-second deadline and closes/reaps its owned process on all handled exits.

Integration exposed two native presentation/input defects, now covered by the
retained-host regression. Explicit foreground colors now supply translucent hover
and sort-button feedback, so a light surface does not inherit dark hover paint.
The demo supplies its Selected background through the public style API. GPU tests
check hovered and selected pixels across light/dark updates. Holding the pointer
on an already-selected row also revealed direct row focus being reported as its
own active descendant; the table adapter now emits composite descendant focus only
while the table container owns focus. The actual AX test draws the held-down frame,
checks direct row focus, then releases and restores ordinary cell focus. No GPUI
fork change was needed. Existing keyboard, child editor/IME, clipboard, styling,
AX and teardown tests pass in the same native run.

Actual rendered captures were inspected after the contrast fix:

- [Dark results inspector](../images/studio-results-dark.png)
- [Light results inspector](../images/studio-results-light.png)

Set `GPUIO_SCREENSHOT_DIR` in the results walkthrough to reproduce them. The hosted
macOS workflow includes this walkthrough; consolidated hosted gates and merge are
still pending.

### Source regression and managed demand

Running the source walkthrough after integrating results exposed a shared native
list defect. GPUI skips renderer callbacks for measured overscan rows. Treating
those absent callbacks as eviction demand repeatedly removed and recreated the
same rows at a stationary viewport. A diagnostic run captured 380 last-source
viewport reports, dominated by two alternating requested sets; this was real
bridge/reconciliation work, not merely an accessibility-test timeout.

The host now keeps the previous bounded demand at the same source, anchor, visible
range and viewport dimensions. Scroll, configuration, source and size changes
invalidate it, and the configured active-row budget still applies. The full source
AppKit walkthrough passes with this correction, including cancellation/retry,
100,000-node reveal, reset, themes and cleanup. Its temporary diagnostic trace had
five reports at the corresponding last-source viewport, with normal focus changes
instead of alternating overscan eviction. These differently completed walkthroughs
are diagnostic evidence, not a controlled performance benchmark. Temporary trace
logging has been removed; combined-workload performance acceptance remains open.

The production-host `native_list` regression also passes on macOS. Its new
closed-loop fixture admits exactly each requested active set, evicts the rest,
and requires six successive unchanged native frames. Cached leading/trailing
overscan converges; scroll, zero-overscan configuration, resizing and complete
source replacement retire old demand while respecting the 24-row limit. Existing
focus/editor/selection checks and the full 100,000-row traversal and revisit pass
in the same run, including resource release after unmount and window close.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native \
  --features native-tests --test native_list
```

The final executable without diagnostic tracing passes all six local integration
regressions: source explorer, native review extension, diagram, results, M4 public
self-test and M4 AppKit. Full Dune `@runtest`, `@fmt`, and strict Clippy for the
native host/table adapter also pass. The M4 self-test recorded 3,916 ms, 1,634 turns,
235 clock ticks, 46 commits, 27 rendered acknowledgements and 37 completed jobs.
These remain regression-workload diagnostics, not combined M5 performance evidence.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 \
  examples/agent_chat/main.exe @runtest @fmt
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j2 \
  -p gpuio-native -p gpuio-table-adapter --all-targets \
  --features native-image-tests -- -D warnings
python3 scripts/test_agent_chat_sources.py
python3 scripts/test_agent_chat_review.py
python3 scripts/test_agent_chat_diagram.py
python3 scripts/test_agent_chat_results.py
_build/default/examples/agent_chat/main.exe --self-test
python3 scripts/test_agent_chat.py
```

## Generation settings and simulated connection

The top-bar **Settings** action and Workspace commands open a modal settings sheet.
[Settings](../../examples/agent_chat/runtime/settings.ml) owns one window's accepted
preferences and uses public native control descriptions and presentation/form
helpers. Rust owns editor drafts, selection, composition and slider gestures.
`initial` seeds each mount from accepted values; observations never replace text.
Closing or changing pages discards unfinished numeric drafts. A settings generation
rejects events from a retired page. Canonical partial OTP codes remain local to the
window; their completion is explicitly a simulation, not authentication.

[Generation_settings](../../examples/agent_chat/runtime/generation_settings.ml)
validates 4–128-byte chunks and 10–200 ms intervals in 10 ms steps. Accepted changes
configure actual future fake-backend sends; active streams keep their captured
configuration. Existing demo presets remain explicit overrides. The settings sheet
supports side, stacked and keyboard-only steppers. Native Return commits and Escape
restores drafts; invalid/partial text supplies associated form help/errors.

The range slider selects inclusive integer percentages through abstract
[Score_range](../../examples/agent_chat/runtime/score_range.ml). Apply replaces the
complete results filter while preserving source size, sort and surviving membership;
large queries use the existing bounded fixture worker. Reset generation preferences
opens a nested alert dialog. Cancellation preserves preferences, and confirmation
resets chunk size/pacing without resetting score filters or active streams.

Local commands pass:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/agent_chat/main.exe \
  @test/agent_chat_showcase/runtest @fmt
python3 scripts/test_agent_chat_settings.py
python3 scripts/test_agent_chat_results.py
_build/default/examples/agent_chat/main.exe --self-test
python3 scripts/test_agent_chat.py
```

The settings AppKit walkthrough exercises numeric partial/rejected drafts, actual
Return/Escape and arrow stepping, native stepper buttons, both slider thumbs,
keyboard/AX slider changes, a real held-pointer preview cancelled with Escape,
query filtering observed in the results page (80–90% gives six sample findings),
retained accepted values, discarded
unfinished drafts, nested reset cancellation/confirmation, native normalized
clipboard paste into the OTP control, clearing, partial-code page retention,
light/dark themes, intact composer content and OS close/discard. All owned processes
are closed/reaped, with a 150-second walkthrough deadline. The first rapid page-switch
test required an observation barrier before clicking the newly enabled destination;
the final test waits for the actual numeric field rather than relying on timing.

Model expect tests verify actual fixture chunk sizes, exact reconstructed Unicode
response bytes, pacing values, rejected settings, inclusive score endpoints and
full-range 100,000-row cardinality. The results and both original M4 acceptance
flows remain passing. These checks do not replace combined M5 workload acceptance.

Actual rendered screenshots were inspected, and numeric padding/banner styling
were refined. Captures are reproducible with `GPUIO_SCREENSHOT_DIR`:

- [Generation, dark](../images/studio-settings-generation-dark.png)
- [Generation, light](../images/studio-settings-generation-light.png)
- [Connection demo, dark](../images/studio-settings-connection-dark.png)
- [Connection demo, light](../images/studio-settings-connection-light.png)

The macOS workflow includes the new walkthrough; hosted execution is still pending.
The following checkpoint adds calendar/color settings. Additional presentation/
navigation families, resize/narrow and reduced-motion acceptance remain open.

## Civil-date reviews and diagram annotation settings

The Settings sheet now includes **Dates & reviews** and **Annotation color**.
[Schedule_data](../../examples/agent_chat/runtime/schedule_data.ml) defines an
explicit October 2026 fixture, with weekday endpoints, October 20 unavailable,
and endpoint-only range validation. Empty selection means all 21 sample reviews;
a partial range cannot be applied. Applying a filter preserves or clamps the
public pagination model (six reviews/page). The list uses public description-list
composition. The independent follow-up picker saves a civil date only: no clock,
timezone conversion, real scheduling, service or network request is involved.

[Schedule_settings](../../examples/agent_chat/runtime/schedule_settings.ml) and
[Annotation_settings](../../examples/agent_chat/runtime/annotation_settings.ml)
activate the public Eio/Bonsai presenters only on their settings page. Accepted
values belong to the window; deactivation cancels popup drafts. Outer settings
generation guards fence retired callbacks. Color preview is local until Apply;
confirmed RGBA colors drive the actual retained canvas connector strokes. Empty
uses the current theme accent. Scene registration also republishes if an annotation
or theme changes while native registration is pending.

The new [AppKit walkthrough](../../scripts/test_agent_chat_dates_colors.py) covers
single/range/partial/disabled/out-of-month dates, native Right/Return selection,
explicit filtering, page clamping, date save/cancel/Escape and trigger-focus
restoration. Color checks cover native palette activation, hex/alpha, malformed
drafts disabling Apply, channel changes, cancel, concrete RGBA retention across
themes, theme-accent reset, preserved composer content and window close with a
nested picker open. Its child has a 180-second deadline and is always reaped.
The test initially requested AXButton for a palette swatch; the correct native
role is AXRadioButton. This was a test correction, not a widget defect.

The color-editor display was refined after screenshot review: canonical channel
values show at most two decimal places. This changes presentation only; full
model precision and successfully entered raw text are retained. Native tests prove
that focus/Return without editing leaves exact alpha unchanged, and cancelling an
invalid hue draft restores the exact model hue while displaying the shorter text.
The complete native color suite includes 64 GPU layout cases and three 64-owner
cleanup/workload cycles. All eight non-GUI color integration tests also pass.

Pure expect tests cover fixture cardinality, endpoint validation, exact filtered
dates, and the endpoint-only rule. Scene tests independently inspect both connector
paints to prove selected RGBA including alpha is identical across themes, while
Empty resolves to the respective palette. Existing settings and diagram AppKit
regressions pass. One diagram launch timed out before its initial window appeared;
a separate run passed all gestures/history/theme/cleanup checks. No cause is claimed
for that isolated startup timeout.

Commands:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native \
  --features native-image-tests --test native_color_input --test color_input
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/agent_chat/main.exe \
  @test/agent_chat_showcase/runtest @fmt
python3 scripts/test_agent_chat_dates_colors.py
python3 scripts/test_agent_chat_settings.py
python3 scripts/test_agent_chat_diagram.py
```

Actual rendered screenshots were inspected in light/dark themes, including actual
red-alpha diagram connectors and the inline calendar scrolled into view. The
walkthrough sends an OS wheel event only after checking the target point belongs
to its child window. A process-targeted wheel did not scroll; normal event posting
at that verified point does. This correction affects the test only.

- [Inline calendar, dark](../images/studio-settings-review-calendar-dark.png)
- [Date picker, dark](../images/studio-settings-date-picker-dark.png) / [light](../images/studio-settings-date-picker-light.png)
- [Color picker, dark](../images/studio-settings-color-picker-dark.png) / [light](../images/studio-settings-color-picker-light.png)
- [Annotated diagram, dark](../images/studio-diagram-annotation-dark.png) / [light](../images/studio-diagram-annotation-light.png)

Both original chat public/AppKit flows pass on the final executable, as do Rust
format and strict all-target native Clippy with `native-image-tests`. Captures are
reproducible with `GPUIO_SCREENSHOT_DIR`. The macOS workflow includes the new walkthrough; hosted
execution and Linux build/unit validation remain pending for this branch.

## Local run feedback and contributor navigation

[Review_feedback](../../examples/agent_chat/runtime/review_feedback.ml) adds a
separate Feedback route reached through **Review feedback** in Review, Workspace
or a run stage. A single application reducer owns the accepted rating, observed
note, disclosure/accordion expansion and contributor preview. Native rating
requests use `Rating.Config.apply_request` against the latest model. Arrow keys,
End, accessibility SetValue and Clear have real effects; hover preview stays native.
The public marker/status-bar/shortcut/label/group-box/separator helpers show the
accepted state and its actual keyboard controls.

The private-note disclosure uses `Retain`: collapse preserves its live native
editor and restores trigger focus. Observed note text supplies only the initial
value on route/inspector remount; it never commands a replacement into the current
editor. Feedback is explicitly in-memory until window close, with no filesystem
or external feedback service. The guidance accordion supports single/multiple
expansion and native header keyboard navigation. Its pure content uses `Unmount`
and is not built for collapsed sections; this is not a claim of Bonsai task
activation or cancellation.

The contributor hover card opens on keyboard focus or pointer hover. Its public
link navigates to the actual Sources route; Escape closes it and restores trigger
focus. Leaving Feedback clears its accepted preview state. The avatar has an
explicit GP initials fallback and meaningful accessible name. The next checkpoint adds actual portrait decoding and unavailable-image fallback.

Local commands:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/agent_chat/main.exe @fmt
python3 scripts/test_agent_chat_feedback.py
python3 scripts/test_agent_chat_review.py
_build/default/examples/agent_chat/main.exe --self-test
python3 scripts/test_agent_chat.py
```

The native feedback walkthrough covers rating keyboard/AX/clear, Unicode note
collapse/focus/remount retention, accordion arrow navigation and expansion
semantics, contributor keyboard/pointer access and actual source navigation,
light/dark themes, unchanged composer content and child cleanup. It has a
180-second deadline and always reaps its process. The review-extension and both
original M4 acceptance flows also pass after adding this route. No Rust or shared
library changes were needed.

The test scrolls guidance headers into view before real keyboard navigation;
AX activation alone can target content outside the painted viewport. It also
moves both pointer and focus to Back before returning from Sources: AXPress does
not transfer focus, and a fresh hover/focus is supposed to reopen a hover card.

Actual rendered screenshots were inspected:

- [Private note, dark](../images/studio-feedback-note-dark.png) / [light](../images/studio-feedback-note-light.png)
- [Expanded guidance](../images/studio-feedback-guidance-dark.png)
- [Contributor preview, dark](../images/studio-feedback-contributor-dark.png) / [light](../images/studio-feedback-contributor-light.png)

Capture with `GPUIO_SCREENSHOT_DIR`. The new walkthrough is included in the macOS CI step; hosted execution remains
pending. Narrow/resizable and reduced-motion checks remain part of the full
showcase acceptance.

## Workspace carousel and contributor portrait fixtures

[Artifact_tour](../../examples/agent_chat/runtime/artifact_tour.ml) adds
**Workspace → Take workspace tour**, a four-item public carousel of Sources,
Results, Diagram and Feedback attachment cards. Each action navigates to the
corresponding working inspector route. A bounded application model owns selection;
native transitions and the optional four-second deadline never call OCaml per frame.
Manual navigation is the default. Start/Pause explicitly changes the native policy.
Leaving the route invalidates outstanding automatic proposals; returning preserves
selection and starts a fresh eligible interval rather than catching up hidden time.

[Contributor_portrait](../../examples/agent_chat/runtime/contributor_portrait.ml)
extends the feedback hover card with **Local portrait** and **Unavailable portrait**.
The first request registers a valid local SVG and deliberately malformed PNM in the
window's Eio scope. Successful publication is distinct from decoding: the malformed
source reaches the native decoder, emits Failed, and the actual avatar renders GP
initials. Restoring SVG emits Ready and renders the image. The two handles are cached
across source/route/inspector toggles; a pending registration is not duplicated.
If the second registration fails, the first is explicitly released before retry.
Closing the window retires both through the scope. No filesystem/network source is
acquired, and no demo-only bridge or fabricated image state is used.

The demo now supports `--reduced-motion` and `--full-motion` as mutually exclusive
native policy overrides. Omitting both follows System. They leave OS preferences
unchanged. The [native tour walkthrough](../../scripts/test_agent_chat_tour.py)
launches one child for each explicit policy, serially, with 180-second deadlines
and guaranteed process cleanup.

Local commands:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/agent_chat/main.exe \
  @test/agent_chat_showcase/runtest @fmt
python3 scripts/test_agent_chat_tour.py
python3 scripts/test_agent_chat_feedback.py
_build/default/examples/agent_chat/main.exe --self-test
python3 scripts/test_agent_chat.py
```

The tour checks native Right/Home/End and numbered/current-item semantics, manual
controls and pointer swipe, destination navigation, 4.7-second focus and hover
pauses, eligible auto-advance under Full and non-advance under Reduce, hidden
inspector/remount without catch-up, Pause, theme switching, preserved composer,
real SVG/invalid-image fallback/restoration and window cleanup. The test's initial
Results-heading assertion was corrected to the actual existing label. Reopening
a hover card after Escape requires a fresh focus entry, so the test moves focus
to Back before returning to its trigger.

Actual rendered captures were inspected after refining card borders/corners:

- [Workspace tour, dark](../images/studio-tour-dark.png) / [light](../images/studio-tour-light.png)
- [SVG portrait, dark](../images/studio-contributor-portrait-dark.png) / [light](../images/studio-contributor-portrait-light.png)
- [Decode-failure initials, dark](../images/studio-contributor-fallback-dark.png) / [light](../images/studio-contributor-fallback-light.png)

The final full-motion scenario passes. In the subsequent reduced-motion scenario,
macOS accessibility stopped exposing the window at inspector close while the child
was still running; cleanup reaped it. A separate reduced-motion run passed the
complete scenario. This isolated window-query failure is retained as evidence,
without claiming a diagnosed product cause or silently retrying inside the test.
Earlier combined normal/reduced runs also passed before the final pointer/all-link
coverage and explicit Full override were added. The hosted step still runs both
policies; hosted checks and broader workload acceptance remain pending.

The feedback regression also explicitly moves the pointer out and back after
Escape, matching the native hover-card suppression contract, and verifies that
the preview is absent while Sources is active. It places focus/pointer outside
the card again after route return before checking closure. With this setup, the
existing feedback walkthrough passes including rating, retained notes and hover
without stealing focus. The original chat public self-test and AppKit acceptance
also pass on the final executable. These are test changes; native hover behavior is unchanged.

This checkpoint demonstrates attachment cards and carousel behavior in the chat;
it does not substitute for remaining transcript/message/bubble/tool-result
composition, springs/sequences, responsive layouts or simultaneous-workload
resource/traffic measurement. Native image policy is covered here by observable
Ready/Failed and real rendered captures, not by inferring decode success from
asset registration.

## Grouped workspace destinations

[Artifact_sidebar](../../examples/agent_chat/runtime/artifact_sidebar.ml) composes
public `Sidebar.view` and `Sidebar.toggle` beneath the conversation list. Its
ARTIFACTS and CONTEXT groups navigate the existing inspector through
[Inspector.navigation](../../examples/agent_chat/runtime/inspector.ml). The current
route supplies selection, including Back/Forward changes and stage-to-diagram
mapping. Expansion and collapse preferences belong to each window's Bonsai model;
changing them never navigates or creates another page/task owner.

Run diagram and Source collection have independently expandable children. The
external toggle offers SVG icon collapse or offcanvas hiding. Existing scoped app
icons are reused. Retained hidden children become inaccessible/inert immediately;
restoring the rail restores both expansion preferences. Closing the inspector
leaves navigation available, and selecting a destination opens its actual page.
Native width transitions use the Sidebar implementation; this is not yet evidence
for the separate spring/sequence integration requirement.

The final local macOS walkthrough passes actual pointer activation, focused OS
Return activation, current-link semantics after history navigation, independent
expansion, hidden descendant accessibility, icon/offcanvas restoration, inspector
reopening, light/dark themes, composer preservation and window/process cleanup.
Build, showcase expect tests and formatting pass. The original chat public
self-test and full M4 AppKit walkthrough also pass on the final executable:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/agent_chat/main.exe @fmt @test/agent_chat_showcase/runtest
GPUIO_SCREENSHOT_DIR=scratch/sidebar-images python3 scripts/test_agent_chat_sidebar.py
_build/default/examples/agent_chat/main.exe --self-test
python3 scripts/test_agent_chat.py
```

The script requires an existing screenshot directory when that variable is set.
The first rendered inspection showed that the old promotional card crowded the
navigation. It is replaced by the same compact Demo controls action, and sidebar
spacing now accommodates both expanded groups at the default window size.
Actual final captures: [dark](../images/studio-sidebar-dark.png),
[light](../images/studio-sidebar-light.png), [icons](../images/studio-sidebar-icons.png),
[offcanvas](../images/studio-sidebar-hidden.png). These show the navigation section
collapsing; the conversation rail remains independently resizable. Narrow-window
and container-query acceptance remain in the responsive-layout work.

## Transcript and query presentation

[Chat_message](../../examples/agent_chat/runtime/chat_message.ml) wraps the existing
owned documents in public `Presentation.message`, `bubble` and `tool_result` cards.
It introduces no document/task registration. Code/diff collapse, copy, Markdown
rendering and conversation scope stay in their existing owners.
[Query_loading](../../examples/agent_chat/runtime/query_loading.ml) supplies native
skeleton, shimmer and spinner views for real pending result queries, fixture
construction and response generation. There is no OCaml animation timer.

Results now show a removable active-filter tag, simulation banner, selection
status bar, empty recovery and query failure/retry alert. Removing a tag updates
the full query, not just loaded rows. Streaming failures use an alert beside the
existing Retry response control. The explicit Slow query fixture takes five
seconds so its loading/cancellation states can be inspected; normal queries stay
at 300 ms.

The local macOS presentation walkthrough passes in explicit Full and Reduce modes:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/agent_chat/main.exe @fmt @test/agent_chat_showcase/runtest
GPUIO_SCREENSHOT_DIR=scratch/presentation-images python3 scripts/test_agent_chat_presentation.py
python3 scripts/test_agent_chat_results.py
```

It checks actual visible code/diff Expand/Collapse and exact Unicode source copy,
keyboard removal of a score filter, all three indeterminate loading roles with no
numeric progress, completion while hidden, remount, obsolete-query cancellation,
query failure/retry, streaming/cancel/error/retry, themes and child cleanup.
The loading check traverses the inspector once to observe the three transient
nodes together. Earlier sequential whole-window searches missed this state;
a live capture/AX dump confirmed the indicators existed. This was not diagnosed
as a production deadlock. Clipboard subprocesses explicitly use UTF-8.

Follow-up fixes the offscreen document accessibility defect exposed here. Native
document toolbar controls now handle Click directly through their retained owner,
sharing their operation with pointer/keyboard activation. Markdown copy-code and
copy-table actions also handle accessibility directly. A labelled Group exposes
each document's existing config label, making source identity discoverable.
Previously GPUI's fallback synthesized a click at a retained offscreen button's
unclipped center, which could hit an unrelated window control.

The final presentation regression expands both artifacts, verifies the code Copy
source button is outside the transcript viewport, replaces the clipboard with a
sentinel, invokes AXPress and checks exact Unicode source plus absence of an
unexpected inspector. Full and Reduce pass, including Markdown code/table copy. The final combined
run completed Full; its following Reduce run lost AX window discovery during the
unrelated query-cancellation step while the child remained alive. Cleanup reaped
it, and a standalone Reduce run passed the complete final scenario. This recurring
AX discovery symptom is not diagnosed here as a product failure. The earlier
viewport-only document test workaround has been removed. Native document and original public regressions pass; native feature-enabled
Clippy passes. The M4 AppKit regression also passes after resolving the retained status/composer
before its one-second acceptance fixture. The direct status observation and edit
took 30 ms in the passing run; earlier whole-window searches missed the transient
state or edited after acceptance. The application fixture delay is unchanged. This is direct command dispatch,
not automatic scrolling/focus of an offscreen document.

Actual captures were inspected: [loading](../images/studio-results-loading.png),
[filter/light](../images/studio-results-filter-light.png),
[empty](../images/studio-results-empty.png),
[stream/light](../images/studio-stream-presentation-light.png),
[error/light](../images/studio-stream-error-light.png),
[code](../images/studio-artifact-code.png) and
[diff](../images/studio-artifact-diff.png).
Existing public chat/M4 AppKit acceptance also passes through this presentation
change. Aggregate animation idle/bridge metrics and simultaneous-workload evidence
remain required; static captures do not establish those properties.

## Native motion in the workspace

[Chat_motion](../../examples/agent_chat/runtime/chat_motion.ml) uses public typed
`Animation.Program` values. The stage detail page has a window-owned Show/Hide
stage context action, reduced against the latest Bonsai state. A single spring
retargets its height between 0 and 112 logical pixels. The nested panel unmounts
its native context immediately on close; the remaining space contracts. This is
not exit-presence orchestration, and it creates no application task.

Workspace overview destinations use two-stage opacity reveals with short ordered
delays. Busy response header/composer indicators use the same named native clock
for their conversation, including when it appears in another window. Idle/error/
cancelled responses mount no repeat members. Native hidden/reduced-motion policy
applies. None of these wrappers installs a per-frame OCaml closure.

The local macOS [motion walkthrough](../../scripts/test_agent_chat_motion.py)
passes Full and Reduce. It measures the Next stage button's actual AX position:
opening settles exactly 112 pixels lower; Full recorded 17 intermediate samples,
while Reduce recorded zero. Reversing after roughly 80 ms (35.5 pixels into the
Full reveal in this run) returns to the original baseline without accumulated
offsets. It checks hidden context absence, keyboard opening, updated next-stage
content, close/remount, themes, active response input/cancellation and child cleanup.
Build, showcase expects and formatting pass. The original public chat, M4 AppKit
and diagram regression walkthroughs pass on the same executable.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/agent_chat/main.exe @test/agent_chat_showcase/runtest @fmt
GPUIO_SCREENSHOT_DIR=scratch/motion-images python3 scripts/test_agent_chat_motion.py
_build/default/examples/agent_chat/main.exe --self-test
python3 scripts/test_agent_chat.py
python3 scripts/test_agent_chat_diagram.py
```

Actual inspected captures: [stage context/dark](../images/studio-stage-context-dark.png),
[stage context/light, reduced motion](../images/studio-stage-context-light.png),
[workspace overview](../images/studio-workspace-overview.png).
The native timeline/group tests are documented in [animation programs](../design/animation-programs.md).
These chat screenshots and AX checks do not measure shared phase at every frame
or aggregate idle/bridge traffic; the combined workload acceptance still must
record those ownership/performance limits.

## Streaming transcript geometry regression

The owner reported a second jitter case while Markdown/code streamed, after the
small-scroll row-retention fix. The native document renderer inserted a flow
`Updating…` line above already installed content on every pending parse, then
removed it on completion. Paginated source metadata was also gated on readiness.
These transient layout changes were unrelated to real new lines.

A failing native geometry regression demonstrated the installed body moving from
Y=24 to Y=56 solely when preparation became pending. The fixed renderer retains
installed content/metadata. The initial notice occupies the existing toolbar,
and no dummy source editor is painted before the first prepared result.
The test now checks pending/ready geometry in both Flow and Viewport layouts for
code, Markdown, diff and first/last pages of a large source fallback. It holds the
pending state deterministically so worker speed cannot hide the layout defect;
existing tests separately exercise real worker/source updates and lifetime cleanup.

An actual slow-stream AppKit probe measured the preceding artifact repeatedly
moving down/up by 29 logical pixels (16 downward 29-pixel steps in 80 samples),
while the composer stayed at Y=813.5. The same probe after the fix recorded 81
samples with **zero downward steps**; messages moved upward only as content grew,
and the composer remained fixed. The stricter permanent test then caught a
separate initial 91-pixel rebound: an empty dummy source editor disappeared when
the first empty Markdown result arrived. Moving the notice into the toolbar and
omitting that dummy body fixed it. The final run recorded **122 samples over 3.50
seconds, nine upward growth steps, zero downward steps**, and a stable composer.
These measurements apply to the deterministic fixture/default window, not every
font, width or arbitrary Markdown edit.

The permanent [streaming-layout walkthrough](../../scripts/test_agent_chat_streaming_layout.py)
checks real streaming through the public chat application, multiple growth steps,
no downward bounce of the preceding card, composer stability and window/process
cleanup. Native document, native Clippy, app build/format/showcase expects and
original public/M4 chat regressions pass locally on macOS. The M4 check preserved
the newer composer draft with the concurrent edit issued 15 ms after Send:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native --features native-tests --test native_document
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j2 -p gpuio-native --features native-tests --all-targets -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/agent_chat/main.exe @fmt @test/agent_chat_showcase/runtest
python3 scripts/test_agent_chat_streaming_layout.py
_build/default/examples/agent_chat/main.exe --self-test
python3 scripts/test_agent_chat.py
```

This removes a confirmed per-parse oscillation, not normal movement from actual
content growth or explicit document expansion. Required hosted gates remain open.

## Responsive inspector and conversation

The chat now keeps a stable nested `View.split_pane` around the conversation and
inspector. The native divider owns live geometry and accepts pointer, keyboard
and accessibility resizing. Closing the inspector's immediate panel reclaims its
space; reopening restores its native sizes without reparenting the conversation
or recreating its editor. The library composition contract and native marked-text
regression are documented in [workspace ownership](../design/agent-workspace.md).

[Responsive](../../examples/agent_chat/runtime/responsive.ml) builds small,
fixed-height alternatives using public `Container_query`. The conversation
toolbar switches at 720 logical pixels; inspector heading/navigation at 420/400;
sidebar branding at 200. These are the query's actual assigned widths (including
the effect of surrounding padding), not window-size callbacks. No OCaml size
observation drives layout. Editors, documents, source/table models and application
tasks remain outside these alternatives. Compact actions retain accessible names;
conversation selection remains available in the sidebar and commands.

The [AppKit walkthrough](../../scripts/test_agent_chat_responsive.py) passes in
Full and Reduce modes. It exercises an actual pointer drag and 16-pixel keyboard
steps, two inspector close/reopen cycles, exact Unicode draft retention, one
reachable copy of toolbar controls, mutually exclusive inspector headings,
sidebar adaptation, light/dark themes and independent windows. It validates
1000/1180/1360-pixel desktop widths. Composer width changed from 722 to 328.5
pixels after the drag, returned to 722 while closed, and returned to 328.5 on
reopening. Child windows and processes closed and were reaped.

AppKit accessibility element handles can change when their native layout ancestry
changes; the public test reacquires them. This is separate from native editor
identity: the native split test verifies the same editor/focus handle and marked
composition survive while its sibling closes. Public draft assertions alone are
not evidence of IME identity. Below the combined pane minimum sizes, the layout
may clip as documented by `Split_pane.Config`; closing the inspector reclaims room.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/agent_chat/main.exe @fmt @test/agent_chat_showcase/runtest
GPUIO_SCREENSHOT_DIR=scratch/responsive-images python3 scripts/test_agent_chat_responsive.py
_build/default/examples/agent_chat/main.exe --self-test
python3 scripts/test_agent_chat.py
python3 scripts/test_agent_chat_streaming_layout.py
python3 scripts/test_agent_chat_motion.py
```

The app build/format/showcase expects, public self-test, existing M4 AppKit and
streaming geometry regression pass on the integrated layout. The streaming check
records 90 samples over 3.51 seconds, ten growth steps and zero downward jumps.
Motion checks pass in Full/Reduce on this final layout: 112-pixel settled context,
16 intermediate Full positions and none under Reduce, interruption, navigation,
stream/input/cancel and cleanup. Actual inspected captures: [resized inspector,
dark](../images/studio-responsive-inspector-dark.png), [narrow,
light](../images/studio-responsive-narrow-light.png), [wide,
light](../images/studio-responsive-wide-light.png). Hosted gates and the
combined workload/resource/idle-traffic measurements remain pending.

The final responsive walkthrough also opens Settings by keyboard and resizes the
window from 1360 to 1000 and back while the sheet remains open. The sheet and
numeric editor stay within their containing bounds; the saved chunk size remains
64. Escape closes the nested reset confirmation first and restores its trigger;
a second Escape closes Settings and restores its original trigger. The composer
draft survives. Both Full and Reduce runs pass.

## Combined streaming, large artifacts and cleanup

The permanent [combined AppKit walkthrough](../../scripts/test_agent_chat_combined.py)
passes locally on macOS arm64 with the development build. It keeps four windows
mounted: 100,000 source nodes, 100,000 read-only result rows, the interactive run
canvas, and the separately packaged native review component. All share a live
conversation. It types 20 actual keyboard characters during streaming, clicks the
native counter, moves a canvas stage with the keyboard, revisits both large-data
views, cancels the stream, and closes every owned window/process.

The opt-in `--workload-metrics --full-motion` fixture sends seven-byte chunks five
seconds apart (the original measurements below used two seconds). This deliberately leaves measurable inter-chunk gaps; it is not a
maximum-throughput benchmark. One window is foreground at a time; others remain
mounted but can be occluded on the single local display. No claim of four
simultaneously visible, full-rate render surfaces is made.

| Measurement | Local result |
| --- | --- |
| Source accessibility rows at 100,000 nodes | 9, within the 24-row budget |
| Table accessibility rows/cells at 100,000 rows | 7 rows / 32 cells, within 24 / 96 |
| Twenty key-post to AX-value readbacks | Median about 56 ms, p95 about 67 ms, maximum 119 ms |
| Four-window registered canvas | One scene, 16,232 reserved bytes |
| Sampled process RSS | 128.6 MB initially; 519.3 MB mounted; about 534.3 MB streaming |
| After each of three fresh window/canvas/extension cycles | 5 scopes, 2 tasks, 36 cleanups, 1 window, 0 canvases / scene bytes |
| Queues/requests at those cleanup checkpoints | 0 queued jobs, commands, pending requests and asset uploads |
| Application-owned resources after cancellation | 18 assets and 10 documents; shared conversations intentionally survive window close |

Readback latency includes macOS accessibility IPC and is **not input-to-pixel
latency**. The test guards p95 below 500 ms, maximum below 1.5 seconds, and sampled
RSS below 1.5 GiB as broad regression limits, not advertised performance guarantees.
RSS is sampled at checkpoints, not a peak or native-cache accounting measurement.
It reached about 579 MB after the final cycle: allocator/native cache high-water
retention does not disappear immediately when OCaml registrations are released.
Three cycles and stable tracked counters do not prove a flat long-duration RSS
profile. Native widget-specific teardown/cache tests provide separate coverage.

Two actual screenshots of the labelled response spinner have different decoded
pixels while bracketing diagnostics show **zero additional commits, submission
attempts/bytes, accepted messages/bytes or decoded events**. The capture selects
the owned window by PID and title, crops its accessible spinner rectangle, and
renders that region into a fresh bitmap before comparison. Thus changes elsewhere
in the window or PNG metadata cannot satisfy the assertion. The observed interval
was 1,003 ms, with 60 ordinary clock ticks/turns/drains and 60 empty-batch bytes.
This establishes native painted animation without additional OCaml transactions;
it does not mean zero FFI calls or an inactive Bonsai clock.

The test also hides/reopens the source inspector three times, then creates three
new windows. Each new window exercises the native extension, creates a canvas,
hides/reopens it twice, and closes. Scene ownership remains window-scoped across
hiding and returns to zero on closing; scope/task/cleanup and shared registry
counts return to the same baseline after each cycle. The monitor is one of the
reported tasks. [Diagnostic contracts](../design/runtime.md#read-only-runtime-diagnostics)
distinguish OCaml reservations from native/GPU caches and explain queued results
versus live producers.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/agent_chat/main.exe @test/runtime/runtest @fmt
_build/default/examples/agent_chat/main.exe --self-test
python3 scripts/test_agent_chat_combined.py
```

Runtime expect tests and the public self-test pass, including cancellation versus
queued-result accounting and traffic consistency. The walkthrough batches AX
attribute reads because separate per-attribute IPC exhausted the old full-tree
search deadline under native animation. Transcript geometry remains independently
covered by the streaming-layout regression above. Hosted execution and final
consolidated milestone gates remain outstanding.

## Consolidated chat regressions (2026-09-26)

All 15 `scripts/test_agent_chat*.py` walkthroughs pass locally on macOS, including
both motion policies where supported, the combined workload, responsive settings
and strict streamed-document geometry. The consolidated run exposed three test
synchronization problems rather than requiring changed application behavior:

- Results and sources could finish their five-second/two-second loading phases
  before a full accessibility search reached them. The shared helper now uses
  one batched AX attribute read per element, as the combined-workload test already
  did. Search bounds, loading durations, cancellation and status assertions remain
  unchanged. Both complete walkthroughs pass, including obsolete-query rejection.
- The tour sent consecutive focus requests without waiting for the first to take
  effect. Its Escape-dismissed contributor preview could consequently remain
  suppressed. The helper now waits for `AXFocused` readback before returning.
  The full normal/reduced tour passes, including theme changes and portrait
  success/fallback/remount; ordinary feedback and responsive focus checks pass too.

The table-history wrapper also completes both 100,000-row traversals and the
intentional-failure cleanup scenario (521 seconds total). The original full
native UI hover-color assertion passed unchanged on an isolated rerun; the
interactive desktop can deliver pointer events independently of synthetic GPUI
input. The animation scheduling oracle and numeric-fixture shutdown changes are
recorded in their [animation](animation-programs-och25.md) and
[numeric](numeric-inputs-och34.md) evidence. Every scenario counted as passing
completed with an actual successful process exit, not only printed markers.

## Remaining acceptance

The accepted family matrix, responsive inspector and combined workload have local
macOS evidence. Consolidated Dune `@all @runtest @fmt`, Rust workspace tests,
workspace/native/table Clippy, Rustfmt, all native executables and all 15 chat
walkthroughs have passing results. A fresh independent extension consumer built
against staged public libraries with the locked backend, ran its native smoke
scenario and exited successfully; no opam switch was modified.

[PR #13](https://github.com/dakotamurphyucf/gpuio/pull/13) records required hosted
macOS/Linux build/unit gates, any resulting fixes, the final checked head and merge.
The [milestone handoff](../milestone-5.md) maps current contracts and delivery;
full Linux GUI acceptance remains OCH-17.

## Hosted Results pointer fixture follow-up (2026-09-27)

Run `36300619471` stopped the Results column drag because its target failed the
owned-process hit test. Locally, a 900-point window reproduced that exact guard
failure: AX returned a SCORE header extending beyond the clipped window. A fitted
1000×700 fixture also reproduced clipping after resizing the pinned ID column.
The hosted log did not include screen geometry, so it cannot establish its exact
window placement.

The walkthrough now fits only its child window to the physical display, logs
screen/window/header bounds, and reveals SCORE using real native Right-arrow
navigation. It swaps TOOL after SCORE using their visible intersection with the
table/window and pinned-column boundary. This also avoids targeting a leading
insertion gap hidden beneath the pinned column. Ownership hit checks remain in
force on every pointer event; a display smaller than the fixture's 984×730 minimum
fails explicitly. Accepted resize/reorder must still survive page navigation.

The full `python3 scripts/test_agent_chat_results.py` passes locally at the fitted
1000×700 size, including sorting, selection/context/reveal, Unicode clipboard,
paging/filter/failure/retry, obsolete-query cancellation, the 100,000-row budget,
themes, retained draft and clean child exit. No production table behavior changed.
Required hosted acceptance remains recorded on PR #13.
The same corrected walkthrough also passes after resizing to 960×700 before the
column operations. The 900-point diagnostic remains deliberately unaccepted: it
now fails the explicit visible-insertion-gap assertion before sending a drag to
an invisible gap. These checks establish the fixture's minimum viewport, not
arbitrary-width application acceptance.

Run `36303803196` subsequently stopped before the Results drag because its window
fit did not settle. That failure path omitted actual geometry, so the artifact
does not establish whether resize or position was responsible. The fixture now
moves first, waits for the requested size, then positions again and waits for
full display containment. It logs initial, requested and final/failing bounds.
This avoids relying on two back-to-back AX requests having both reached their
final geometry before the fit check.

The complete walkthrough passes locally at 1000×700 and at 1000×688 using a
1024×768 logical fit rectangle on the existing display. The latter is a smaller
fixture boundary, not a physical 1x or hosted-screen reproduction. Native pointer
ownership, insertion-gap visibility, accepted reorder retention and all subsequent
workflow assertions remain required. Hosted placement confirmation is pending.
Each independent chat walkthrough now has its own required CI step so this failure
cannot hide settings, streaming, responsive or combined-workload diagnostics.

## Hosted desktop-space prerequisite (2026-09-27)

Run `36306457769` passes every one of the 22 independent component checks, all
other native/public families and the simultaneous four-window workload. Its eight
failing chat checks are Sources, Results, Settings, Dates/colors, Feedback,
Presentation, Streaming geometry and Responsive layout. Required Linux checks
and X11 pass; Wayland retains the recorded clipboard failure under OCH-17.

The new Results diagnostics establish a concrete runner constraint: the desktop
is 1024×768, initial window bounds are (-78,25,1024,684), and a request for
1000×688 settles at (12,40,1000,684). Responsive's 1180×820 request also settles at
height 684. Tests expecting those larger window geometries cannot succeed on that
desktop. Other failures concern unobserved source/date/slider state, focus, a missing
retained toolbar and absent row geometry; the logs do not prove one cause for all
of them, so they still require validation after the environment correction.

An early CI-only desktop preflight now requests a sufficiently large available
mode for the login session and checks it again in a separate process. It retains
the full component/chat assertions and logs supported modes and usable bounds.
Local compilation/read-only inspection passes at 1728×1117 (usable 1728×1007),
and applying changes outside GitHub Actions is verified to fail before mutation.
No local display settings were changed.

Run `36309465532` confirms the runner offers 1600×1200 and 1920×1080 modes
and successfully applies 1600×1200. The helper then failed because its AppKit
`visibleFrame` retained the original 1024×681 usable area while CoreGraphics
reported the new bounds. The apply process now checks CoreGraphics bounds;
the required separate read-only process checks fresh AppKit usable bounds.
Local read-only compilation and workflow lint pass. Fresh-process hosted
confirmation and the remaining eight walkthrough results are still pending.


## Tail append and capture-window follow-up (2026-09-27)

Run `36309700746` confirms the session-mode preflight at 1600×1200 with 1600×1102
usable points. All component/native families and seven previously failing chat
walkthroughs pass. Required Linux checks pass; X11 succeeds and Wayland retains
the OCH-17 clipboard failure. Two macOS checks remain: Streaming geometry and
the combined animation measurement.

Streaming now reaches the geometry assertion and catches an initial 84-point
backwards step. The preceding card moves from Y775 to Y455 to Y539 while the
composer stays at Y854. The first 320-point movement matches two 160-point
placeholder estimates. A deterministic presenter regression confirms newly
appended descriptions were absent until the next viewport observation. Native
rendering could therefore paint estimates before receiving the real rows.

The Bonsai presenter now prefetches the bounded appended suffix while native
layout reports tail following. It captures the observed final key with that
viewport callback; pins keep priority and paused history stays requested.
Appended rows and order arrive together, with no synchronous OCaml layout call
or larger active-row budget. The new expect regression fails before the change
and passes afterward, including retention across display acceptance. The local
strict streaming check passes with zero downward steps and a stable composer.
Full OCaml build, expect tests and format checks also pass.

The combined check still requires two differing spinner captures bracketed by
identical commit/submission/event counters. A local diagnostic adding one second
to each real owned-window capture reproduced the failure: capture brackets take
3.0–3.5 seconds, crossing the fixture's two-second chunk interval. The opt-in
measurement fixture now spaces chunks five seconds apart; normal demo streaming
is unchanged. Per-attempt capture duration, sample span and traffic deltas are
logged. Pixel-change, input-latency, large-data retention and cleanup assertions
remain intact. Keyboard delivery is paced across a producer interval, and an
explicit response-byte counter must advance during typing; pacing is excluded
from each measured key-to-AX latency. Both the ordinary and slow-capture combined walkthroughs now pass locally,
including four-window cleanup. Both observe response bytes advancing from 0 to 7
during actual typing. The normal animation bracket is 1,504 ms; the slow-capture
bracket is 3,007 ms, with 181 clock ticks and zero commit/submission/event deltas.
The existing native chat walkthrough also passes. The final strict streaming
check observes 116 samples over 3.47 seconds, nine upward growth steps, zero
backwards steps and a stable composer. Required hosted confirmation of this
batch remains pending on PR #13.

## M6 Results regression follow-up (2026-09-28)

Required run `36426087223` passed 99 macOS steps and all required Linux steps,
with failures in the chart visual fixture and Results walkthrough. Results had
accepted the native resize/reorder, then read the TOOL header immediately after
Diagram → Back; the AX table root existed but the visible header lookup was
empty. The existing helper waited for the root, not its descendant header.

The header lookup now reacquires the current table/tree within five seconds.
A persistently missing header still fails, geometry remains strict, and width/
order retention assertions are unchanged. No production table change is inferred
from this trace. The original and revised complete local Results walkthroughs
both pass on macOS 14.5. An additional 20 Diagram → Back cycles retain the resized
column and reordered headers, followed by the full 100,000-row/query/clipboard/
theme/cleanup flow. PR #14 records the subsequent required hosted result.
