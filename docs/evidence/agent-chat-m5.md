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

## Remaining acceptance

These flows cover the OCH-23 package integration, OCH-24 diagram, the OCH-37
navigation stack/breadcrumbs, settings sheet/confirmation and feedback
disclosure/accordion/hover card/carousel/sidebar, OCH-34 numeric/OTP,
OCH-35 dates, OCH-36 colors, OCH-37 pagination, OCH-38 source explorer,
OCH-39 results table and a subset of OCH-33's
presentation compositions. They do not complete the other component families,
responsive/resizable inspector, motion, or the
combined streaming/input/retention/idle-traffic workload. Those all remain required
by OCH-46, along with narrow/wide visual acceptance, the full coverage map, hosted
macOS/Linux gates and merge. Full Linux GUI acceptance remains OCH-17.
