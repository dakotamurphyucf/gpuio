# Implementation status

Current checkpoint, 2026-09-27: milestone 5 is merged in
[PR #13](https://github.com/dakotamurphyucf/gpuio/pull/13) at `936fb7d` after
[required macOS/Linux CI](https://github.com/dakotamurphyucf/gpuio/actions/runs/36312697654)
passed on `473407c`, including all 71 macOS GUI stages. All twelve milestone
tickets are Done. Milestone 6's OCH-27 now has public Core/Eio link routing,
bounded readiness/backpressure and packaged macOS cold/warm OS delivery with
native window close/reopen checks. Represented-document and edited metadata pass
native/public macOS tests; Linux reports typed unsupported outcomes. Linux
launch forwarding passes local private D-Bus/codec tests through `App.run_desktop`;
typed packaging and real private bus arbitration now pass locally; real Linux GUI
invocation and consolidated hosted gates remain pending. Portal file services pass local private
D-Bus/worker tests, with Linux build/GUI validation still outstanding. macOS file open/reveal and explicit scheme
registration pass local public OS checks. See the
[milestone 6 plan](milestone-6.md) for current scope, evidence and next steps.
OCH-28's typed Core/Eio notifications and owned macOS adapter now pass public
packaged Notification Center actions, replacement/dismissal, readiness,
closed-window routing and service cleanup locally. Linux notification transport and its bounded native worker now pass local
private-bus action/replacement/owner-loss tests, with required Linux build and
desktop presentation validation pending; see
[notification evidence](evidence/os-notifications-och28.md).
OCH-40's typed chart data model now covers all seven families and mixed Cartesian
layers, with bounded paired codecs and local expect tests including 100,000
points, independent binary fixtures and exhaustive small-graph cycle checks. A pinned Sankey algorithm extraction compiles/tests against the
unchanged GPUI revision. The bounded native resource store and scoped Eio scheduler
now pass publication/cancellation/retention tests and a connected windowless macOS
application check with 100,000 points through the public Eio API. Native chart
interaction now passes native and public macOS input checks. Explicit line/area envelopes, bar sum/mean
and candle OHLC policies now have paired types and a tested native reduction kernel
that preserves gaps and aggregate source ranges. Validated native plotting options
and retained logical geometry now cover all seven families, with passing local
extreme/degenerate, 100k exact/sampled and crowded Sankey tests. The prepared native
painter now passes actual hidden-window GPU checks for every family, mixed layers,
gradients/corners, clipping and hollow/filled candles. The Core/Bonsai chart view
bridge, scoped event fencing and native tree validation are implemented and locally
tested. Resource-backed mounted views now pass hidden GPU lifecycle checks and
the public seven-family Chart Studio self-test. Native axis/family labels and
scrollable legends now render from the displayed snapshot; dense legend scroll
persists across updates and clears on reset. Typed semantic-selection events,
exact-source provenance and a bounded worker-prepared hit index are implemented;
native hover/drag, plotted-mark keyboard input, tooltips and semantic callbacks now
pass locally. A built-in read-only original-data table now pages through all source
values independently of rendering/sampling, with bounded native rows and keyboard/AX
navigation. Bounded series identifiers, themed data controls and mixed/horizontal
examples now pass local checks, including actual dense-legend wheel scrolling
and offset retention/reset. Two hidden managed-list windows now pass repeated
frame-budget/GPU checks, independent scrolling and fourteen shared publications
with observer disposal and idle source release. A public 80-publication/150-update
workload now records 10k/100k sampled and exact rendering, coalescing, CPU/RSS,
queue bytes and retention on Apple M1 Max. Broader-example integration and
consolidated hosted gates remain. See
[chart contracts and remaining work](design/charts.md).
The detailed entries below record implementation checkpoints chronologically;
their earlier pending-gate statements are superseded by this delivery record.

Updated 2026-09-26. Milestones 01 and 02 are merged, including native text editing,
controls/interactions and declarative animations. [PR #10](https://github.com/dakotamurphyucf/gpuio/pull/10)
merged at `17e863279bff25253edf47f449c04cd9aee5e867` after the required macOS and
Linux checks passed. Milestone 03 / OCH-13 implements keyed collections, paging and managed virtual
lists in [PR #11](https://github.com/dakotamurphyucf/gpuio/pull/11). See the [managed-list design](design/managed-lists.md)
and [local acceptance evidence](evidence/managed-lists-och13.md). The managed
component, paging, native interactions and full-history retention tests pass
locally and in hosted validation at `2c2063b`. See PR #11 for the final checked
head and merge. CI run 36056171245 also passes X11 list checks; Wayland stops
at the existing combobox clipboard failure before reaching them. Final review
adds a second complete OCaml 100,000-row traversal, also passing locally.

Milestone04 is implemented in [PR #12](https://github.com/dakotamurphyucf/gpuio/pull/12): revisioned
streaming documents and native Markdown/code/diff; independent windows, retained
tabs and split panes; and a polished agent-chat reference application. The app's
public integration and external macOS AX/keyboard/picker scenarios pass locally.
See the [M4 evidence ledger](evidence/agent-workspace-m4.md),
[ownership design](design/agent-workspace.md), and [runnable demo](../examples/agent_chat/README.md).
The full consolidated local build, suites, native regressions, Clippy and format
checks pass. The evidence ledger records hosted results; PR #12 records the final
checked head and merge status. Full Linux GUI acceptance remains OCH-17.

OCH-46's combined macOS workload now passes with 100k source nodes, 100k table
rows, canvas and native extension mounted in four windows during streaming.
Native keyboard input, bounded accessible rows/cells, painted animation without
additional OCaml transactions, and repeated window/canvas cleanup are measured.
Read-only runtime diagnostics distinguish serialized traffic and owned resources
from clock polling and total/native memory. Full/Reduce responsive checks also
cover settings-sheet resize, saved values and nested Escape/focus restoration.
See the [combined evidence](evidence/agent-chat-m5.md#combined-streaming-large-artifacts-and-cleanup).
Consolidated local Dune/Rust suites, native checks, all 15 chat walkthroughs and
a fresh staged extension consumer now pass. [PR #13](https://github.com/dakotamurphyucf/gpuio/pull/13)
records required hosted macOS/Linux results, the checked head and merge state.
The [milestone handoff](milestone-5.md) maps all delivered families and ownership
contracts to current source and evidence.

OCH-37 now has compiled and locally tested Core models for bounded navigation
history, single/multiple disclosure and pagination. Tests cover route replacement,
back/forward/branching, disabled and stale requests, collection/page-count shrink,
128-entry navigation, 4,096-item disclosure and 10,430 bounded pagination partitions.
Core/Bonsai panel/disclosure/accordion bindings and initial native macOS checks now
pass keyboard/expanded accessibility state, nested focus restoration, retained and
unmounted editors, marked-text isolation and hidden focus-scope cleanup. A small
vendored patch to the unchanged accesskit_macos 0.26.3 exposes expanded state;
both native backend build paths use it. Public breadcrumb/pagination compositions
now pass bounded-model/reconciliation tests and native AppKit current descriptions,
keyboard/AX actions and focus retention. The Navigation Lab also verifies retained
Unicode drafts, independent lazy Bonsai lifecycle and Eio data-scope cleanup.
The initial sidebar adds grouped/nested destinations, independent expansion,
icon/offcanvas modes, scoped icons, context commands and current-link semantics.
Local public macOS AX checks and screenshots cover its collapse modes; native
regressions also cover custom disclosure headers and retained hidden popup scopes.
Native sidebar width transitions now pass public macOS geometry, interruption and
reduced-motion checks. Retained offcanvas content now slides out on either side
while inert: native input/AX access stops immediately, editors survive, and nested
animations/popup scopes suspend. GPU and public screenshot evidence verifies paint
continues during exit. Navigation now has tested bounded native transition state
and a mounted Core/Bonsai presenter, including reversal from painted positions,
retained native controls, destination focus, outgoing GPU paint and immediate
removal. Native nested/modal, IME, pointer/keyboard exit gating and a full 128-page
workload with resize also pass, alongside the public example. Four retained editors
fit within unchanged editor quotas; history bounds do not exempt native resources.
Four-edge sheets and alert-dialog adapters now share the existing modal focus and
asynchronous dismissal infrastructure. Native macOS checks cover edge geometry,
clamping/resize, late hover styles, editor identity, focus restoration and nested
alert backdrop blocking. The public Navigation Lab includes drawer/confirmation
flows; its latest validation is recorded in the evidence ledger.
Interactive hover cards now expose a separate nonmodal Dialog role while reusing
native tooltip timing, retained content and placement. Local native tests cover
Tab/pointer/IME/Escape, accepted controlled close, anchor restoration and timer
cancellation; the Navigation Lab includes a contributor preview. Existing tooltips
retain their separate help semantics and grace clock.
Carousel now has a tested Core selection model, paired envelopes, Core/Bonsai
constructors, bounded default pagination and native admission/request dispatch.
Mounted horizontal/vertical presentation reuses retained pages; local macOS checks
cover GPU transition geometry, retained editors and focus preservation/handoff.
Native keyboard and auto-advance scheduling now pass local tests for child-editor
key isolation, pause/resume, clipping/window activation, one pending proposal,
no idle frames and teardown. Native wheel bursts now pass axis/cancellation,
momentum fencing after accepted selection, missing-end fallback, nested scrolling,
reduced-motion input and disposal checks. Native pointer/GPU checks now cover
axis locking, two-page preview, capture/rebinding, snap/accepted retargeting,
in-flight grabs, child-control priority and lifecycle/foreign-capture cancellation.
The public Navigation Lab now verifies native requests through Eio/Bonsai, AX/current
metadata, native auto-advance and explicit unmount leases while Bonsai/data remain
alive. Native marked-text and nested-popup checks now pass focus handoff, hidden
input rejection, IME-first Escape, editor key isolation, popup focus/hover pause
outside the carousel bounds and full scope/timer disposal. Shared overlays now
register their visible panel bounds with the existing focus manager. Local OCH-37
component acceptance is complete; navigation bit `274877906944` is advertised
(current aggregate `2199023255551`). Consolidated hosted checks and merge remain,
followed by final ticket completion. The chat showcase stays in OCH-46.
See [navigation design](design/navigation-components.md) and
[foundation evidence](evidence/navigation-components-och37.md).

OCH-38 now has a pure Core `Tree` collection with stable typed IDs, validated flat
forest topology, revisioned replacement, parent/ancestor/sibling metadata and
O(log n) payload updates sharing topology. Expect tests exercise malformed graphs,
100,000-node traversal/reorder, depth/metadata limits and distinct incarnation/
child revisions for future lazy-load admission. `Tree_state` now adds separate
incarnation-checked expansion/selection preferences, cached visible order, logical
cursor repair, single/multiple/range selection and pure tree keyboard reduction.
Tests cover hidden/disabled/reordered/reincarnated nodes, 100,000 selections and
application-payload collection. Core/Eio lazy loading now provides generation-
checked requests, 64 queued branches, four reusable workers, atomic child pages,
explicit retry and bounded error detail retention. Local runtime tests cover
cancellation without concurrency overshoot, queued-result reset, inbox backpressure,
shutdown and preservation of unrelated tasks. `Tree_rows` now projects item and
lazy-boundary records into keyed list data, with compact generation/incarnation-
checked identity, point invalidation and no historical key registry. Core tests
cover 100,000-node updates, 200,000 logical item/boundary rows, depth 128,
collapse/reopen and old-payload collection. The Bonsai tree-row primitive now
mounts only viewport/pinned rows, checks source-instance/reset identity, preserves
coalesced invalidation and retires old controller effects. Eio controls drive
capacity-limited visible demand, explicit retry and collapse cancellation; local
combined runtime tests distinguish view unmount from application data lifetime.
Tree metadata now reaches the native managed-list root and one focus-owning item
per row. Paired codec/Core/Bonsai tests and actual macOS AppKit checks cover
hierarchy, selection, expansion, disabled state, updates/removal and teardown.
The pinned macOS accessibility adapter adds reproducible disclosure getters.
A pure `Tree_interaction` reducer now checks source/node identity, preserves ordered
relative requests, separates activation, opens ancestors for logical reveal and
returns application-approved move proposals with approval-time revalidation.
Opt-in native keyboard/pointer requests now travel through monotonic list identity
to current Core/Bonsai handlers. Local macOS checks cover ordered arrows/modifiers,
AppKit focus/select, child editor/IME isolation and pointer priority. Explicit
reveal now hands focus to a stable row after asynchronous mounting, with bounded
pending state and cancellation on retirement, blur, deactivation or scrolling
away. Native tests cover a one-row budget and actual window activation changes.
Unicode typeahead now supports canonical accents, case folding, repeated-prefix
cycling and current-label search, with an event-driven native expiry clock and
bounded Core prefix. Codec, reducer, queued Bonsai and actual native key-dispatch
tests pass; a 100,000-node Core benchmark is recorded separately from native
workload acceptance. Per-row AppKit selection and expansion/disclosure setters now
carry explicit desired states, preserving ordered requests before rerender. The
public Bonsai widget now integrates preferences, ordered native/controller requests,
default row presentation and deferred reveal/focus. Its Eio filesystem example
passes actual directory loading, native focus retention, selection and stale-command
retirement after reset on macOS. Opt-in native dragging now emits typed move
proposals through current row identities, with native lifecycle cancellation and
Core/Bonsai endpoint validation. Local GPUI tests cover all placements, cancellation,
preview cleanup and embedded-editor isolation. The public editable outline passes
actual AppKit drag, confirmation and context-menu moves. Native GPU checks cover
actual focus/blur and all drop indicators above opaque rows, including changed
foreground and hover exit. Full traversal and revisit now pass separately through
Bonsai and native GPUI at 100,000 rows/depth 128, with 256 transient rows and weak
probes proving model/resource release. Native source reorder/window isolation,
deactivation/close, and public deep reveal, resize, loading/retry/cancellation and
window-scope cleanup now pass. Paint-time focus schedules one follow-up redraw to
publish its pin while OCaml is idle. Local OCH-38 component acceptance is complete;
managed-tree bit `549755813888` is advertised (aggregate `2199023255551`).
Consolidated hosted gates and merge remain before ticket closure. See the
[managed-tree design](design/managed-trees.md).

OCH-39 now has a tested Core column schema with stable keys, finite widths,
resize clamping, left pinning and keyed multi-level header groups. Tests cover
4,160 moves at the 64-column limit, invalid schemas and user restrictions. An
isolated styled DataTable candidate compiles against the unchanged GPUI pin after
a one-line macro crate-name fallback. Its native macOS probe renders at most
80 distinct body cells across four sampled positions in a 100,000-row/64-column
table. This is candidate evidence, not production integration or full-history
acceptance. Core data/paging and a scoped Eio adapter now pass 100,000-row order
and paged workloads, retired-row/query checks, saturated inbox delivery and
100 rapid resets while cancellation cleanup holds both worker slots. Producer
concurrency stays at two; old results are discarded and unrelated scoped work
survives explicit close. The selected native adapter now lives in `rust/table`,
with upstream provenance, existing base helpers and per-instance appearance.
Its macOS test reproduces the four virtualization samples, preserves selection
through row/column reorder, clears removed selections without event echoes, and
verifies native entity release on close. Native pointer tests now exercise keyed
double/context/sort/resize/reorder events and suppress obsolete frame input and
drags after schema refresh. Keyed pixel anchors survive native row/column reorder
and prepend; unchanged schemas preserve resize/reorder gestures through row
arrivals. The bridge, public widget, full paging/query reconciliation and native
keyboard/accessibility/cache/lifecycle acceptance remain.
Paired bounded table payload codecs and the pure Core `Table.Config` now pass
independent byte fixtures and invalid-input/budget tests. Transaction/event
envelopes now feed native tree admission, cell/schema byte accounting, ordered
command admission and live session input validation. Query resets retire viewport
handlers, and dirty cell edits revalidate table ownership. The retained native
host now renders real cell Views, publishes bounded viewport demand, executes
commands, shares focus retention and captures input routes at event creation.
A local background-window test passes a sparse 100,000-row source, row-height
anchor changes, single/empty data and entity release. The Core managed table View
now constructs bounded keyed cells, generates accepted schema revisions, routes
typed input through current query/schema/policy checks and emits ordered commands.
The public Bonsai presenter and Eio paging controls now connect bounded cell
lifetimes, membership-aware commands, query resets and current selection. Local
tests traverse all 100,000 rows twice and release retired cell payloads. The public
Table Lab passes native keyed scrolling/anchors, streaming, query retirement,
failure/retry and window cleanup. Native pointer/key dispatch and OS clipboard
checks now pass exact Unicode/quoted TSV, unavailable-selection preservation,
Tab exit, toolbar Copy, child-editor priority and hidden/disabled gating.
The public table now applies its style to one native root while separate
source identity preserves reset semantics. GPU checks pass alpha/gradient surfaces,
border/padding/corner clipping, inherited text, state precedence, pinned-column
paint and readable selected cells/rows. The styled Table Lab retains selection
and anchors through light/dark changes. Actual macOS accessibility now passes
logical counts/indices, Unicode cell values, selected-descendant focus, ordered
selection setters, separate sorting, selection-mode restrictions and retired
hidden/disabled objects. The vendored Cocoa adapter has a reproducible table
metadata/selection patch. The native table now passes two complete 100,000-row
traversals with at most 128 active rows/512 cells, actual horizontal sweeps and
zero retired text payloads retained at batch checks. This exposed a pinned Taffy
measurement-context retirement issue; a reproduced, documented two-line patch
at the same version fixes it in both backend paths. Unmount/window release and
intentional failure cleanup pass. The public example now also preserves anchors
and selection through accepted resize/reorder during a pending Eio page and
sorts while obsolete producer cleanup is held; late results do not alter the
new query. Shared controls/editor/list/tree/table regressions, Rust workspace
tests and strict Clippy pass. AppKit keyboard and embedded-editor composition now
pass through targeted OS events and the native text-input client. The public
example also passes pointer/keyboard inspection, guarded context actions, reveal,
focus restoration and native window closure. Selectable text now exposes its
content label to macOS accessibility. The [local acceptance audit](evidence/data-tables-och39-audit.md)
maps the live requirements to code and native evidence. Managed tables advertise
bit 40 (`1099511627776`), with shared mask `2199023255551`. Showcase integration,
hosted macOS/Linux gates and merge remain pending.
See [table design](design/data-tables.md) and
[evidence](evidence/data-tables-och39.md).

OCH-46 is now in progress. The chat's Explore workspace inspector integrates the
separate native counter package through its generated backend, with real property,
event and acknowledged-command flows. Local macOS pointer/keyboard, hide/reopen,
draft preservation and theme checks pass; both existing M4 chat acceptance suites
remain green. The native package's duplicate Space activation was fixed. Actual
light/dark screenshots were inspected and the controls refined. The inspector
also includes a lazily registered run diagram and bounded artifact history.
Native selection/keyboard movement, pointer drag, pan/zoom/reset, stage activation,
back/forward/replacement/breadcrumbs, preserved window state and themed screenshots
now pass locally. The new expect tests and existing review/M4 regressions remain
green. These are integrated flows, not completion of the full component matrix or simultaneous
workload. See [M5 integration evidence](evidence/agent-chat-m5.md).

The chat's source explorer also passes local macOS acceptance through the public
managed-tree/Eio APIs: lazy failure/retry, keyboard range/typeahead, drag and
context move approval/cancel, reveal, collapse cancellation, empty/reset and the
explicit 100,000-node fixture. The last-source viewport exposes 9 native AX rows
within its 24-row budget. Window-owned data/preferences survive inspector page
changes; fixture construction uses an Eio worker domain. Native theme tokens now
follow the chat palette, and all prior inspector/M4 acceptance suites remain green.
Actual source screenshots and scoped ownership evidence are recorded in the same
ledger.

The results inspector now passes its own local native walkthrough: full-query
sort/filter, paging/failure/retry/cancellation, cell/context/reveal/Unicode copy,
actual column resize/reorder, pin/reset and preferences across page unmounts.
The opt-in 100,000-row fixture exposes 7 AX rows / 32 cells at the last record,
within its 24-row / 96-cell budget. Shared fixture work is bounded to one running
producer and one latest replacement per window/fixture. Actual light/dark captures
led to corrected selection/hover contrast; GPU regressions verify both themes.
A held-pointer row-focus AX crash is fixed in the table adapter and protected by
a native intermediate-frame test. The full component matrix, combined workload,
consolidated hosted gates and merge remain pending.

The first settings pages now pass local macOS integration: a modal sheet with
window-owned stream chunk-size/pacing preferences, inclusive score-range filtering
of the actual results query, and an explicitly simulated six-digit connection.
Native numeric partial/invalid drafts, Return/Escape, three stepper presentations,
slider keyboard/AX edits and pointer preview cancellation, accepted-value remounts,
nested reset confirmation, OTP paste/clear, themes and composer preservation pass.
No native draft is replaced by an observation; closing discards uncommitted numeric
drafts and stale callbacks are fenced by the settings generation. Model tests
verify real fake-backend chunking/delays and score bounds. Existing results and M4
public/AppKit regressions pass. Date/color settings now also implement civil-date
review filtering/pagination, simulated confirmed follow-ups and concrete RGBA
annotations on the actual diagram. Local native keyboard/AX checks cover partial
and disabled dates, focus restoration, color validation/cancellation and retained
values across themes. Remaining presentation/navigation, motion/responsive and
combined-workload acceptance are still pending; see the M5 evidence ledger.

The review workspace also has local run feedback: ordered native rating requests,
retained private-note disclosure, single/multiple guidance accordion, and an
interactive contributor hover card linking to Sources. Local AppKit validation
covers keyboard/AX, pointer hover, focus restoration, note/rating retention across
route and inspector remounts, themes and composer preservation. Contributor initials
are shown initially; a local SVG portrait and actual unavailable-image decode/fallback
fixture can now be selected inside the preview. Existing
review extension and original M4 public/AppKit regressions remain passing.

The workspace tour uses a public native carousel of four attachment cards with
real destination actions. Local normal/reduced-motion runs verify keyboard and
control navigation, current semantics, focus/hover/hidden pauses and native
opt-in auto-advance. Contributor portraits reuse two window-scoped registrations;
actual native decoding failure shows initials, and switching back restores the
SVG image. The demo accepts `--reduced-motion` without changing OS preferences.
Remaining full M5 integration, combined workload and hosted/merge gates stay open.

The chat now has grouped public Sidebar destinations sharing inspector routes and
history, with independent branch expansion, SVG icon collapse and retained
offcanvas hiding/restoration. Local macOS pointer/keyboard/current-link and
light/dark walkthroughs pass; screenshots and source links are in the
[showcase evidence](evidence/agent-chat-m5.md). Public message/bubble/tool-result
cards, removable query tags, loading skeleton/shimmer/spinners, status and error
recovery now pass the Full/Reduce local presentation walkthrough, including exact
Unicode code/diff copy. Stage-context springs, ordered destination reveals and
shared response activity are integrated; local Full/Reduce geometry/interruption,
hidden context and response-input checks pass. The responsive inspector now passes
local Full/Reduce pointer/keyboard resizing, close/reopen geometry, Unicode draft
retention, hidden-alternative accessibility and independent-window checks at
1000–1360-pixel desktop widths. Native split tests also preserve marked IME text
and editor identity when the sibling pane closes. Combined workload and
hosted/merge acceptance stay open. Native documents now expose labelled groups
and dispatch toolbar/Markdown copy accessibility actions directly. The chat
regression verifies exact Unicode copy from a retained offscreen toolbar without
misrouting a click to another visible control, in Full and Reduce modes.

Streaming transcript jitter is also fixed: already installed documents no longer
insert/remove an Updating line for each pending parse, and initial preparation
does not paint a dummy source editor that disappears with the first Markdown
result. A failing native geometry
regression now passes in Flow/Viewport layouts, and the actual chat probe changed
from repeated 29-pixel rebounds to no downward steps while the composer stayed
fixed. See the [streaming evidence](evidence/agent-chat-m5.md#streaming-transcript-geometry-regression).

Milestone 05 is in progress on `milestone-5-ui-extensions`. OCH-23 static native
components pass local acceptance; OCH-24 has validated geometry, a bounded scene
codec, native resource ownership and a tested OCaml/Rust upload bridge. The
pure typed scene API, scoped Eio registration, bounded native geometry/job
preparation and GPUI worker/mesh painting are implemented. Hidden-window macOS
GPU tests pass for shapes, curves, clipping, pan/zoom, resizing and cleanup.
Native interaction state now covers selection, drag previews/cancellation,
position ownership, viewport policies and commands; GPU checks validate its
effective transforms through direct state calls. Bounded native text/raster/SVG
painting now passes GPU clipping, zoom, retired-source and deferred-work checks.
The typed canvas view, owner-aware reconciliation, bounded native tree admission
and revision-checked event bridge now have local regression coverage. Mounted
retained-tree GPU rendering passes publication, hide/show, command retention,
preparation failure/recovery, generation reset and repeated disposal checks.
Native-dispatch input checks now pass focus, drag/selection pixels, keyboard,
pan/zoom, wheel coalescing and cancellation, including actual window deactivation.
Native macOS object accessibility now passes label/selection/focus, separate
activation, transformed bounds, offscreen reveal, disabled/hide/removal checks.
The public OCaml Canvas Lab passes command/update/reset lifecycle checks with
184 and 19,024 items, plus external macOS AX/keyboard interaction checks.
The maximum mounted workload also passes with 20,000 marks, 2,048 interactive
objects, 4,096 accessibility nodes and zero retained accounting after each of
three update/resize/disposal cycles. Local canvas acceptance is complete and the
full canvas capability is advertised; consolidated hosted gates remain pending. See the
[extension](evidence/extensions-och23.md) and [canvas](evidence/canvas-och24.md)
evidence for exact completed scope. Its scope includes OCH-23–26 and OCH-33–39, followed by
[OCH-46](https://linear.app/ochat/issue/OCH-46/showcase-milestone-05-features-in-the-polished-agent-chat-reference):
showcase the completed feature families in the polished agent-chat application.
OCH-46 is part of milestone completion and covers feature mapping, interactive
flows, light/dark and responsive-layout polish, accessibility, reduced motion,
streaming/performance regression checks and updated screenshots/documentation.
Its [component coverage and flow plan](design/agent-chat-m5-showcase.md) is
versioned; its unimplemented rows and combined acceptance remain required work.
OCH-25 now has validated public spring parameters, an independent OCaml/Rust
parameter fixture and a tested analytic native spring trajectory. Typed programs,
a bounded codec and compiled finite sequence timelines also pass local tests.
The retained owner and bounded shared-clock registry now pass deterministic
lifetime, playback, phase and stale-paint tests. Atomic session admission and
compiled-storage quotas, GPUI rendering, and `View.animate_program` with bounded
stage-event batches are wired. Public Bonsai/Eio and expanded native macOS checks pass, including retained-list
and hidden-panel timing, deferred overlays, two-window clock lifetimes, spring
retarget controls and a 1,024-visible-owner workload with input and complete disposal.
The advanced-program capability is now advertised; hosted gates remain pending. See
[advanced animation evidence](evidence/animation-programs-och25.md) and
[design](design/animation-programs.md).
OCH-26 now connects typed container rules through retained views, bounded
selection events and native assigned-size layout. Local macOS checks pass resize
selection, keyed identity, nested/dialog/virtualized queries, hidden motion,
AppKit accessibility and IME isolation, pointer cancellation, fractional/scale
boundaries, observer/config lifetimes and a 256-query workload. The public
Bonsai/Eio example verifies retained editors and lifecycle behavior. Container
queries are advertised and final local capability checks pass. Consolidated
hosted macOS/Linux gates and merge remain pending.
See [container query design](design/container-queries.md) and
[acceptance evidence](evidence/container-queries-och26.md).
OCH-33 now implements bounded semantic metadata, native form associations and
stateless presentation/card helpers with public Core/Bonsai/Eio usage. Local
OCaml/Rust tests and macOS editor/AX/keyboard checks pass, including metadata
updates during IME composition and corrected light/dark card layouts. Avatar
image/fallback now passes actual GPU/AX, source lifetime, SVG resize recovery,
synthetic density and idle checks, with a public example covering ready/failure/
initials. Rating now provides controlled request reduction, native hover/stars,
keyboard and AX slider actions with read-only/disabled and modal/pointer policies.
Local acceptance passes, including 228 constrained-content combinations. The
presentation capability is enabled; consolidated hosted gates and merge remain. Native skeleton/shimmer/
spinner leaves now pass local reduced/static/ancestor-hidden idle, resume,
accessibility and disposal checks; the public example exercises all three.
See [presentation evidence](evidence/presentation-components-och33.md) and
the [Component Studio example](../examples/presentation/README.md).
OCH-34 has begun with validated shared numeric domains, min-anchored stepping,
and draft classification, with independent OCaml/Rust fixtures and boundary tests.
Slider contracts, bounded codec and native interaction state now pass local
checks. Retained slider views, tree admission and observation routing are implemented;
native single/range rendering and initial macOS pointer/keyboard/AX checks now pass.
Correlated commands and the public Bonsai/Eio slider controller/example pass local integration.
Slider foreground/focus styling now passes local GPU pixel checks for both axes,
light/dark palettes, display densities and constrained layouts. The public example
shows single/range and linear/logarithmic modes, all now exercised by external
macOS keyboard/AX automation. Decorated pointer geometry, capture loss,
minimize/restore and independent-window/close lifetimes also pass locally.
A three-cycle, 1,024-owner native workload passes bounded event/coalescing, idle
and owner-disposal checks; debug timing and batching limits are documented.
Number_input Core/wire contracts and bounded native codecs now cover draft/value
separation, historical domains, UTF-8 selection/IME ranges and guarded commands;
independent cross-language fixtures and semantic validation tests pass locally.
The native numeric policy model now validates commit/cancel/step, pending-edit
guards, composition/configuration preservation and revision/fault behavior with
deterministic state tests. Its callbacks now connect to one native InputState.
Retained numeric view descriptions, strict tree admission, owner/revision event
routing and byte-accounted change coalescing now pass local bridge tests.
Mounted numeric editing, basic step buttons, correlated commands and the Eio
controller are implemented. Initial macOS native checks pass keyboard commit/
cancel/step, clipboard, undo/redo, revision guards, retained configuration, marked
text composition and disposal. Native step-button hold-repeat now passes local
Sides/Stacked, cancellation, real window deactivation and idle-task disposal
checks. Actual AppKit numeric/text values, focus/edit/step/button actions in all
three stepper layouts, draft feedback, application metadata and IME/read-only/
disabled/hidden policies now pass locally. The public numeric example now passes
three-layout command/event, draft/history/selection, revision/lease/policy and
remount/close integration checks. Numeric GPU light/dark, density, focus and
constrained-layout checks now pass, with a stacked-button overflow fixed. External
macOS AX/OS keyboard tests also pass through the public example. Retained IME and
history through configuration changes, hidden/modal gates, independent-window
close during a held repeat, and three 256-editor workload/disposal cycles now pass.
OCH-34 local acceptance is complete for sliders, numeric editors/steppers and OTP.
OTP now has public Core/Bonsai/Eio controllers, bounded paired codecs, retained
native editing and correlated commands. Native key/clipboard/NSTextInputClient,
AppKit value/action/masking, public OS keyboard/AX, GPU light/dark/density/preedit,
managed-list pins, hidden/modal/capture cleanup and independent-window lifetimes
pass locally. Three 256-owner workloads verify bounded history/coalescing, idle
behavior and complete disposal. Capability `34359738368` advertises the family
(current aggregate `2199023255551`). Consolidated hosted gates and merge remain pending.
See [OTP contracts](design/otp-inputs.md).
See [numeric design](design/numeric-inputs.md) and
[foundation evidence](evidence/numeric-inputs-och34.md).
OCH-35 now has tested civil-date/month/selection/constraint models, typed locale
configuration, commands/observations, bounded paired codecs and a native calendar
policy owner. Partial ranges, historical selection validity, focus callback
failure and revision exhaustion have model coverage. Gregorian-cycle, daily
reference, malformed-wire and maximum-config checks pass locally. Retained calendar
views, bounded tree admission, seed/history semantics, revision-checked event routes
and atomic completion mailbox admission now pass paired-codec/Core/native tests.
The mounted GPUI calendar now passes initial macOS day/month/year keyboard/pointer,
single/range event ordering, retained historical state, hidden/read-only/disabled
focus, pair-overload and disposal checks. Four initial light/dark GPU readbacks were
visually reviewed. Public correlated controllers now pass local real-window checks
for commands, revision/lease guards, admission limits, policy changes and close
ordering. Hidden focus cleanup now reports actual platform focus. A public popup
picker now separates application values from native drafts, with explicit
Apply/Cancel, revision/session guards and external-value invalidation. Local
macOS AX/OS-input checks cover popup selection, dismissal and focus behavior.
Inline AppKit values/cursor/selection and OS keyboard/Tab navigation also pass,
with native civil endpoints, leap clamping and locale retention checks. Calendar
rendering now passes 192 GPU theme/density/font/constrained-layout cases; long
labels use ellipses and caller font overrides are honored. Native modal/pointer
gates, managed-row pins, independent windows and three 64-owner idle/disposal
cycles also pass locally. Single/range popup mode changes, nested-dialog dismissal/
focus, actual panel bounds and right-edge placement also pass. OCH-35 local
acceptance is complete; capability `68719476736` is advertised (aggregate
`2199023255551`). Consolidated hosted gates and merge remain pending.
See [calendar design](design/calendar.md) and [model evidence](evidence/calendar-och35.md).
OCH-36 now has concrete Core/Rust RGBA/HSLA conversion and bounded hex-draft
models, separate from theme references. The pure native policy now tests
preview/commit/cancel, draft/composition handling, hue memory, guarded Set/Reset,
stale callbacks, configuration history and revision/fault lifetimes. Core control
contracts and bounded standalone Rust/OCaml codecs now pass independent byte
fixtures and malformed/maximum-payload checks. Retained descriptions, tree seed/
history semantics, revision-checked routing and atomic bounded event batches now
pass local bridge tests. Initial mounted channel/palette checks now pass native
keyboard/Tab, pointer preview/commit/Escape, configuration-during-drag, disabled
isolation and disposal. Native Hex/HSLA text fields now pass draft/commit/cancel,
composition, history/configuration preservation, bounded storage and child
disposal checks, including actual AppKit marked-text/insertion delegates. Runtime
controllers and a public Color Studio example now pass real-window correlated
command, revision/lease, policy, request-limit, remount and close checks. Popup
selection now passes public session/policy/Apply/Cancel integration and actual
macOS AX/OS keyboard, dismissal, focus restoration, nested-dialog and clamped
placement checks. OCH-36 local acceptance is complete: GPU checks cover 64
light/dark/density/font/constrained-layout cases and transparent/opaque/empty
swatches; native tests cover composition/managed-row pins, hidden/modal/pointer
gates, independent-window deactivation/close and three 64-owner/320-editor
workload/disposal cycles. A short-height clipping bug was fixed and channel
fields now use available width without rounding native values. Color capability
`137438953472` is advertised (aggregate `2199023255551`). Consolidated hosted
macOS/Linux gates and merge remain pending. See [color design](design/color-inputs.md)
and [foundation evidence](evidence/color-inputs-och36.md).
No milestone-05 completion or hosted acceptance is claimed yet.

The [milestone acceptance ledger](evidence/milestones-01-02.md) maps every ticket
to its implementation and evidence. The first consolidated hosted run passed all
macOS checks and Linux build/unit tests; one Linux-only native-test lint issue was
corrected for final-head verification. Linux GUI remains informational. See
[theme/scale review](evidence/theme-scale-och11.md) and
[animation contracts](design/animations.md). Checkpoints below describe the state
at the time and preserve earlier validation findings.

Repository: `dakotamurphyucf/gpuio`, public, Apache-2.0, default branch `main`.
These settings were selected by the owner on 2026-09-11.

Platform priority updated by the owner on 2026-09-11: macOS is the primary
functional acceptance platform during implementation. Linux builds/unit tests
remain required, but graphical checks are informational and full Linux GUI
validation is deferred to OCH-17. Linux remains an intended platform. Earlier
design documents requiring native GUI acceptance on both OSes before advancing
are superseded by this priority; native GUI coverage must still be reported honestly.

- OCH-18 complete: remote scaffold, standards/design import and fresh-clone checks.
- OCH-19 complete: pinned OCaml/Rust closure, reconstructed
  native Bonsai sources and patches, codec/lifecycle checks.
- OCH-20 complete: isolated bootstrap and contributor tools.
- OCH-21 complete: source-built Dune/Cargo smoke app and
  two-window native identity/lifetime scenario.
- OCH-22 complete: required builds/tests, macOS native checks, informational Linux
  graphical checks, retained evidence and protected main branch.
- OCH-6 setup gate complete, merged in PR #1 at `81f6b581c784d448a8948d4cb55e73af9db4b86c`.
- OCH-7 complete, merged in PR #2 at `e6471b4ec88e6847f950da30576b6d2a6639d930`.
  CI run34650637422 passed on both OSes, including production 50-revision/
  two-window/rollback/panic smoke on macOS, X11 and Wayland.
- OCH-8 complete, merged in PR #3 at `8f7fd9f357a0b8df3e9dfe31c2a7217925c8846d`: typed views/styles/themes, keyed reconciliation, pure Bonsai
  adapter, native button/selection behavior and GPUIX style mapping. Local macOS
  tests and both required CI jobs passed in run 34654290650. X11 passed all GUI
  checks; Wayland passed the typed bridge but failed the new hover-reset test.
  That informational limitation remains tracked in OCH-17.
- OCH-9 public Bonsai/Eio runtime merged in PR #4 at
  `02558d8d393c49e5e159812394dd9061820c39fc`. Both required CI jobs passed in
  run 34740866262, including all macOS runtime/measurement scenarios. Linux GUI
  exposed a default quit-policy difference, fixed in PR #5 at
  `88cc9287db49cd27c0b78a6f19eea17fc4c1069e`. Final run 34741216419 passed both
  required jobs and all OCH-9 scenarios on macOS, X11 and Wayland. X11 passed
  the full GUI suite; the existing Wayland hover-reset issue remains under OCH-17. See [runtime](design/runtime.md) and [measurements](evidence/runtime-och9.md).

## Local evidence

macOS arm64, stock OCaml 5.3.0, Dune 3.24.2, Rust 1.97.1. The separate
`.opam-root/gpuio` was created from the pinned opam repository; the existing Ochat
switch and default toolchain selections were not modified.

- Core/PPX expect tests and native Bonsai lifecycle tests pass, including
  optimized/unoptimized graphs, unchanged views, keyed retention, cleanup and a
  dedicated OCaml domain.
- The OCaml and Rust codec checks independently construct, encode and decode the
  same 96-byte fixture with full byte consumption.
- The actual GPUI window self-test passes: 50 checked commits, 1525 command bytes,
  native input-handler probes, stale event rejection, Rust panic containment,
  balanced 23/23 row activation/deactivation, Eio cancellation and clean shutdown.
- Two actual windows pass distinct identity, independent editor state, stale
  window rejection and continued use of the surviving window after closing the
  first. Both close and return through the FFI.
- First-party Rust passes Clippy with warnings denied. GPUI's transitive `block`
  0.1.6 reports a future-compatibility notice; it does not fail the pinned build.

`docs/evidence/macos-arm64-packages.txt` is the actual isolated package inventory.
Historical research documentation is preserved under `docs/design` and is not
an assertion of current production API functionality.

## Hosted evidence and remaining platform validation

PR run [34646959232](https://github.com/dakotamurphyucf/gpuio/actions/runs/34646959232)
passed on macOS ARM64 and Ubuntu 24.04 x86-64. The informational Linux GUI report
also records X11 and Wayland success: both asserted the intended backend and
passed the 50-commit native/lifecycle/input-handler example and two-window
identity/cleanup scenario. X11 used Xvfb/Openbox; Wayland used nested Weston;
Mesa software Vulkan supplied rendering. This is actual backend window coverage,
distinct from the earlier accidental headless X11 attempt. Both pinned language servers have passed hover and
go-to-definition checks; evidence is in `docs/evidence/*-lsp-navigation.json`.
Main requires PRs and both `foundation (macos-15)` and `foundation (ubuntu-24.04)`
checks with an up-to-date branch. Force pushes and branch deletion are disabled.
Linux GUI outcomes remain informational and do not alter this development gate.
No full OS IME automation, accessibility or production multi-window Bonsai API is
claimed by the bootstrap smoke tests. Those remain in their owning v1 tickets.

## Typed API validation (OCH-8)

The pure API tests cover callback-only refresh, keyed reorder/replacement, invalid
plans, theme changes, style composition/reset and bounded incremental output.
OCaml and Rust independently agree on every expanded style tag in `style-v1.hex`.
Native tests validate malformed styles, rollback and nested memory accounting.
The actual macOS window test passes grid bounds, hover/pressed/focus, Enter/Space,
Tab/Shift-Tab, pointer policy, Unicode select/copy, replacement and inherited reset.
The public OCaml example passes 20 acknowledged native commits and theme changes.
The [typed API contract](design/typed-ui.md) records all GPUIX style mappings and
functional limits. Linux graphical execution remains informational under OCH-17.

## Milestone 02

OCH-10 implements native input/composer ownership, stable Bonsai/Eio controllers,
revisioned commands, native composition and grapheme editing, undo/redo selection,
auto-grow and basic accessibility. [PR #6](https://github.com/dakotamurphyucf/gpuio/pull/6)
and [its evidence report](evidence/native-editor-och10.md) record implementation
and platform validation. Hosted run 34745383026 passed Linux build/tests/lint and
macOS editor/input/accessibility checks. X11 passed the complete GUI suite;
Wayland passed public editor commands but its clipboard-based native test failed
before insertion, tracked in OCH-17. These checks do not claim physical IME
candidate-panel or complete screen-reader coverage.

Current OCH-11 summary: the controls/commands/focus/overlays, pointer capture,
file-dialog bridge and drag/drop behaviors below are implemented and validated
locally. Drag/drop includes actual AppKit handoff/reentry/cancel/unmount and held-
gesture close/shutdown checks. Raster/SVG assets, bounded caches and foreground-tinted icons now pass local
integration checks. Decorative button/command-button icon slots also pass local
checks. Nested transcript/code/composer/popup/modal scrolling and scroll-owner
disposal also pass local native checks. Remaining work includes theme/scale and
native-state audit, basic transitions and aggregate lifetime review. Consolidated macOS/Linux CI and merge
remain. The chronological checkpoints below distinguish earlier partial states
from later validation; they do not all describe the latest remaining scope.

OCH-11 is in progress: controlled checkboxes/switches and disabled buttons merged
in PR #7 (`0ef2c7dc5b71235c090d4dc6373f505db69624e5`); CI 34747606484 passed both
required jobs and control windows on macOS, X11 and Wayland. The existing Wayland
editor clipboard limitation remains under OCH-17. Radio groups in PR #8 pass
both required jobs in CI34748629291, including actual control windows on macOS,
X11 and Wayland. PR #8 merged as `5448842ffd9bff9e249071698a294a3afc3ffb42`.
Select PR #9 merged as `8a9167cf227a290f967452c051f0b4c7dde19461` after both required
jobs in CI34750240273 passed. Its adapter adds native popup navigation/cancellation, current-frame
positioning and virtualized options. Local macOS window/accessibility checks,
4096-option navigation, OCaml/Rust tests, full build/format and Clippy passed.
Choice appearance adds theme-aware popup/option/empty styles, configurable uniform
row geometry and localized empty text while retaining native focus/open state.
Local tests validate these changes; general overlay integration remains OCH-11 work. [Native controls](design/native-controls.md) records these families
and the remaining ticket scope. OCH-12's declarative animation configuration and
timing core and view/bridge/native integration now pass local tests; application
motion policy and platform preference detection remain pending.
Combobox is implemented on the local OCH-11 branch with native editor ownership,
query filtering, exact selection snapshots, the editable accessibility role and
shared popup appearance/virtualization. Local native control tests pass including
macOS marked/committed text. The public controller smoke passes conditional
replacement, stale revisions, undo and unmount; existing two-window editor commands
also pass after sharing the controller implementation. OCaml/Rust tests and Clippy
pass locally. Full build/format validation is recorded with the local change.
Per owner instruction, remaining OCH-11 work stays local until the complete ticket
is ready for a consolidated CI pass. No Combobox hosted acceptance is claimed.
The broader component catalog is planned in OCH-33–45; vendoring GPUI Base does
not expose all of those widgets through the OCaml API.

Focus scopes are also implemented locally for OCH-11: native Tab trapping, nested
entry/restoration, hidden/disabled traversal, empty-root fallback, command and
accessibility gating, and bounded cleanup pass actual macOS control-window tests.
The other remaining OCH-11 families are still in progress.
No hosted acceptance is claimed for this local scope implementation.

Dialog/Popover surfaces are implemented locally with application-controlled
content lifetime, typed dismissal, native stacking/placement and accessibility
semantics. Local macOS tests pass nested dialogs, restoration, choice-popup
interaction beyond panel bounds, marked-text Escape and moving anchors. The
public Bonsai/Eio overlay example passes native mount, editor commands, modal
focus denial, stale unmount and close. OCaml/Rust tests, independent fixtures,
full build/format and Clippy pass locally. No hosted acceptance is claimed;
tooltips/menus/commands and the rest of OCH-11 remain in progress.

Anchored placement is implemented locally: popovers accept preferred side,
start/center/end alignment and signed offset, with current-frame edge flipping
and viewport clamping. Local native checks retain focus while changing placement
and moving the anchor; positioning/validation unit tests and independent protocol
fixtures pass. The extension appends a new operation without changing earlier
overlay records.

Tooltips are implemented locally with managed or application-controlled visibility,
retained arbitrary content, delayed hover, shared grace timing and keyboard
opening/dismissal. Hidden content preserves native editor identity while denying
focus and deactivating nested traps. Local macOS tests pass hover cancellation,
interactive content, tooltip/popover hit routing, accessibility exposure and
bounded timer/subscription disposal. The public Bonsai/Eio example passes retained
editor commands, controlled visibility, stale unmount and shutdown. Independent
protocol fixtures, OCaml/Rust tests, full build/format and Clippy pass locally.
No hosted or full Linux GUI acceptance is claimed for this local checkpoint;
menus/commands, feedback, pointer/desktop interactions and assets remain OCH-11 work.

Shared command registries, command buttons and single-chord shortcuts are
implemented locally. Native macOS tests pass scoped dispatch, native editing targets,
keyboard/IME priority and two-window isolation, including closing one window and
continuing in the other. Independent OCaml/Rust protocol fixtures pass. The public
Bonsai/Eio example, full OCaml build/tests/format, Rust workspace tests and
Clippy pass locally. Menu adapters are being validated locally as described below;
the command palette and other OCH-11 requirements remain In Progress.


Menus are implemented on the local OCH-11 branch: immutable command-reference
models, dropdown/context/in-window/platform presentations, virtualized cascading
popups and active-window macOS menu ownership. Targeted macOS native tests pass
actual NSMenu and accessibility activation, right-click Copy/focus restoration,
1000-entry wheel/keyboard navigation, popover integration, focused command scopes,
hidden/stale actions and menu restoration after closing a second window. The
activation test found and fixed menu ownership refresh when returning to an
unchanged surviving window. Hidden triggers now close detached popup state and
release focus. The combined native controls suite, public Bonsai/Eio example,
Rust workspace tests and Clippy pass locally; the [menu evidence report](evidence/native-menus-och11.md)
records coverage and limitations. No hosted or Linux GUI acceptance is claimed.


The command palette is implemented on the local OCH-11 branch: ordered command
references, native query/composition/history, virtualized results, shared command
execution, modal focus and accessible activation. Local macOS tests pass a
1000-command list, current-query/current-generation routing, native document Copy,
hidden/nested-modal restoration and query disposal. Visibility-driven dismissal
now runs before paint can discard focus ancestry. Full OCaml build/tests/format,
Rust workspace tests, Clippy and the public Bonsai/Eio example pass locally. The
[palette evidence report](evidence/native-palette-och11.md) records the checks and
an unresolved intermittent tooltip-hover failure seen in an earlier combined run;
the subsequent combined controls run passed. OCH-11 remains In Progress, with no
hosted or Linux GUI acceptance claimed for this checkpoint.


Progress indicators are implemented locally for OCH-11 with validated fractions,
explicit indeterminate state, native animation, percentage accessibility and the
existing root-style/theme API. Actual macOS tests pass painted dimensions/colors,
noninteractive focus, animation without OCaml commits, and hidden/determinate/
unmount cleanup. The public Bonsai/Eio example, combined controls suite, full
OCaml build/tests/format, Rust workspace tests and Clippy pass locally. The
[progress evidence report](evidence/native-progress-och11.md) records the scope;
in-app notifications are described below and OCH-12 still owns general motion
and reduced-motion integration. No hosted or Linux GUI acceptance is claimed.


In-app notifications are implemented on the local OCH-11 branch with keyed
terminal sessions, bounded stacks, explicit overflow, native active-time deadlines
and hover/focus/hidden/modal pause. Local macOS tests pass close/accessibility/
keyboard actions, native editor Escape priority, ordinary action content, expiry
without OCaml commits and unmount cancellation. The public Bonsai/Eio example
passes native dismissal delivery and keyed removal. Independent protocol fixtures,
OCaml expect tests, Rust workspace tests and the combined native controls suite
pass locally. The [notification evidence report](evidence/native-toasts-och11.md)
records exact coverage and remaining validation. No hosted or Linux GUI acceptance
is claimed. OCH-11 still includes pointer capture/drag-drop/file dialogs, assets/
images/SVG/cache and theme-scale integration, remaining state/transition work,
scrolling/lifetime checks, documentation and consolidated CI/merge.

Captured pointer regions are implemented locally for OCH-11. Real macOS native
tests pass out-of-bounds movement, redraw/reposition retention, cancellation,
modal gating, nested ownership, native child-control precedence and pressed
styling. The combined controls suite passes after final release-order review;
Clippy and the full Dune build/tests/format also pass. Independent protocol/Core
tests cover validation, callback lifetimes and motion coalescing. The public
Bonsai/Eio resize example passes its lifecycle self-test. See the
[pointer evidence report](evidence/native-pointer-och11.md) for precise coverage.
Pointer capture remains distinct from drag/drop and file dialogs, which are still
pending alongside assets/images/SVG/cache, theme-scale integration, remaining
state/transitions, scrolling/lifetime checks and consolidated CI/merge. No hosted
or Linux GUI acceptance is claimed for this checkpoint.

File-dialog implementation has started with pure OCaml/Rust path and open/save
configuration models. Focused Core expect tests, Rust protocol tests and Clippy
pass, including exact non-UTF-8 path bytes, filename validation and selection
limits. These constructors do not present dialogs. The
[file-dialog design](design/file-dialogs.md) records the
contracts, pinned-source findings and remaining acceptance work.

The macOS Rust file-panel adapter now passes native sheet presentation, file and
directory selection, exact save-path return without file creation, Busy,
cancellation and owner disposal checks. Full Clippy, Rust workspace tests and
Dune build/tests/format pass with its direct macOS dependencies. The
[file-panel evidence](evidence/native-file-dialogs-och11.md) describes the actual
AX-based test and its permission requirement. The bridge checkpoint below adds Runtime/Eio/Bonsai integration and application
close cancellation. Capability reporting and Linux portal support remain pending;
this is not a completed file-dialog feature or OCH-11 ticket.


The OCH-11 file-dialog bridge now connects the OCaml configuration models to
window-owned macOS panels through correlated Bonsai/Eio effects. Local native
ownership tests and public close/shutdown tests pass; an end-to-end test selects
the LICENSE file through real AppKit controls and reads it explicitly with Eio.
Independent fixtures cover exact raw path bytes; result decoding and mailbox
accounting enforce count/size bounds. See the updated
[file-dialog evidence](evidence/native-file-dialogs-och11.md). Capability queries
and the Linux portal backend remain pending (non-macOS currently returns
Unsupported), so file dialogs and OCH-11 are not complete. No hosted CI or Linux
GUI acceptance is claimed for this checkpoint.

The Linux file-dialog protocol layer is now implemented in the new `gpuio-portal`
workspace crate. Fourteen local D-Bus socket-peer tests pass for request/reply
races, cancellation/cleanup, service identity/loss and bounded URI results. It
reuses existing locked dependency versions. The crate is not yet connected to
the native runtime: X11/Wayland parenting, cleanup barriers, capabilities and Linux
validation remain pending. See the [portal design](design/linux-file-portal.md).
No actual Linux portal GUI or completed OCH-11 support is claimed.

The next local checkpoint connects the portal worker to X11 native requests.
Window-close/shutdown cleanup now waits for background workers, including a
response already being delivered. Quit cleanup runs before GPUI clears windows;
ordinary lifecycle cleanup stays asynchronous. The shared ownership adapter's
tests, full Rust/OCaml checks, actual macOS picker suite and public Bonsai/Eio
close/selection/read regressions pass. See the updated
[portal evidence](evidence/linux-file-portal-och11.md). Wayland exports, public
capabilities and Linux build validation remain pending. OCH-11 stays In Progress.

Wayland file-dialog parenting is now implemented locally with one shared guest
registry per application display and separately owned surface exports. It uses
GPUI's existing socket reader, bounded pending-export polling, cancellation and
the native cleanup barrier. Full workspace Clippy/Rust and Dune checks pass on
macOS; three new system-libwayland protocol tests compile but are explicitly
ignored here and await Linux execution. Public capabilities and consolidated
Linux/macOS CI remain pending. No Linux GUI or complete OCH-11 acceptance is
claimed; see the [Wayland checkpoint evidence](evidence/linux-file-portal-och11.md).

Public file-dialog capabilities are now implemented locally: a per-window typed
snapshot reports single/multiple selection by mode and save support, with the
same Not_ready/Busy/Closed lifecycle as pickers and no picker presentation.
Local macOS native/public tests, independent OCaml/Rust fixtures, portal version/
no-presentation tests, full build/format and Clippy pass. Existing real selection
and Eio-read regressions pass after sharing the correlated query path. See the
[file-dialog capability evidence](evidence/native-file-dialogs-och11.md). Linux
build/unit verification (including three ignored-on-macOS Wayland tests), remaining
OCH-11 feature families, consolidated CI and merge are still required.

OCH-11 drag/drop now has validated OCaml/Rust payload, source and target models,
plus bounded bin_prot codecs and independent byte fixtures. Text, raw Unix paths
and opaque custom data retain distinct validation rules; desktop-file offering
requires explicit directory metadata and native target acceptance uses an exact
format allowlist. Full local workspace Clippy/Rust and Dune build/tests/format
pass. These are data/configuration tests, with no native drag/drop interaction
claimed yet. View/event/native ownership integration is next; see the
[drag/drop design and remaining acceptance](design/drag-and-drop.md).

The next OCH-11 drag/drop checkpoint integrates source/target views through
reconciliation, protocol, native trees and Bonsai/Eio event routing. Local macOS
native window-dispatch tests pass for nested acceptance, immutable gesture
snapshots, cancellation/removal, raw incoming files, size limits and release of
source/hover state. The public example's lifecycle test, independent operation/
event fixtures, queue/ownership tests, full builds/format/Clippy and existing
native pointer regressions pass. The bridge advertises drag/drop bit 1048576
(required mask 2097151). See [integration evidence](evidence/drag-drop-och11.md).
Actual OS file export/reentry, multi-window/focus/active-close checks and public
gesture callback testing remain; this is not completed drag/drop or OCH-11
acceptance. No hosted CI or Linux GUI acceptance is claimed.

Actual AppKit mouse dragging now passes through the public Bonsai/Eio example:
matching gesture identity/payload, accepted hover, result update, painted frame
and clean shutdown. Expanded native checks pass focus-trap cancellation and
second-window activation/recovery with late-release suppression. No production
runtime patch was needed for the system-event test driver. Native Clippy, full
Dune checks and the ordinary lifecycle example pass. See the updated
[drag/drop evidence](evidence/drag-drop-och11.md). OS file export/reentry, live-close/
shutdown and remaining OCH-11 families still require work; no hosted CI is claimed.

Actual macOS file-session checks now pass through the public Bonsai/Eio API:
second-window Desktop delivery with a distinct gesture ID and unknown metadata,
source-window reentry restoring original identity/metadata, OS Escape without a
drop, and source unmount suppressing late callbacks while the immutable OS offer
remains receivable. The temporary source file remains unchanged. These are real
AppKit sessions between child windows, not Finder/external-copy acknowledgement
or Linux GUI coverage. See [drag/drop evidence](evidence/drag-drop-och11.md).
Live-window close/shutdown while dragging and the aggregate lifetime review remain,
as do the other OCH-11 families and consolidated CI/merge.

Held-gesture close/shutdown validation now passes locally for both internal drags
and OS-owned file sessions. AX confirms physical source-window removal; a surviving
window paints after an explicit mouse-release handshake. App shutdown returns
cleanly with no callbacks to disposed sources. The drag-specific ownership review
found no reference cycle and records bounded snapshots/hover state separately from
OS payload lifetime. Native Clippy, full Dune checks and transfer/unmount regressions
pass. See [drag/drop evidence](evidence/drag-drop-och11.md). Remaining OCH-11 feature
families and consolidated platform gates are unchanged; nothing has been pushed.

Asset work has started with immutable OCaml/Rust source descriptors for the nine
pinned GPUI format families. Constructors preserve opaque encoded bytes, enforce
nonempty/16-MiB bounds and report format/length rather than dumping contents.
Core expect tests, targeted Rust tests, protocol Clippy and full Dune build/tests/
format pass. The [asset design](design/assets.md) records the required chunked
transport under the existing 1-MiB envelope and the native ownership/cache plan.
Registration, decoding and image/icon views are not implemented by this checkpoint;
no asset capability is advertised yet.

The native application session now owns a bounded encoded asset registry: ordered
chunk staging, complete-data publication, generational IDs, retirement and terminal
shutdown. Existing readers keep retired data valid and charged until they release
it; retired handles cannot create new uses. Five registry tests and the session
lifecycle test pass, along with the Rust workspace, native Clippy and full Dune
checks. See [asset evidence](evidence/assets-och11.md). This registry is not yet
connected to FFI upload commands or the OCaml runtime, and no image/icon rendering
or new capability is claimed. Those integrations are the next OCH-11 work.

Encoded assets now cross the FFI using bounded correlated Begin/Append/Finish/
Release messages and reserved responses. Independent OCaml/Rust fixtures,
mailbox-pressure tests, full Rust/Clippy and Dune checks pass locally. A windowless
public Eio runtime example uploads >2 MiB and verifies release, stale IDs, invalid
uploads, quota recovery and shutdown. The new capability 2097152 (aggregate
4194303) advertises encoded registration only. Scoped public ownership, decoding,
image/icon views and cache cleanup remain; see [asset evidence](evidence/assets-och11.md).

Scoped encoded registration now uses `Gpuio_eio.Asset.register app ~scope source`.
The adapter bounds queued source bytes/live metadata, suppresses cancelled user
completions while accounting for late allocation replies, and reserves one request
lane for upload/cleanup independent of raw traffic. The windowless native example
passes public registration and scope retirement under saturated raw request lanes,
with subsequent full-quota allocation proving reclamation. Deterministic scope tests
exercise every upload cancellation boundary. Decoding and pure image/icon views are
still pending; this is encoded ownership, not rendered-image acceptance.

The native in-memory raster decoder now covers PNG/JPEG/WebP/GIF/BMP/TIFF/ICO/PNM,
GPUI BGRA ordering, static EXIF orientation, GIF delays and complete-result failure
on malformed frames. It checks dimensions and retained pixel/frame bounds. This
helper is not yet scheduled from the host or exposed in views; SVG, aggregate
worker/cache ownership and actual rendered-image acceptance remain pending.
See [asset design](design/assets.md) for strict-output versus best-effort decoder
allocation limits and [pixel-test evidence](evidence/assets-och11.md).

The decoded-cache/work-ticket controller now reserves result output before native
work dispatch, bounds live/queued/running/retired state, shares source decodes and
keeps evicted pixels charged through their last reader. Worker/handle identities
reject late or foreign results; mounted-owner disposal directly cancels work.
Controller tests include an actual background-thread decode. The host does not
yet schedule these tickets or perform per-window atlas evictions; SVG and image
views are still pending. See [asset ownership design](design/assets.md).

The production native host now initializes the image scheduler, launches admitted
decodes on GPUI's background executor, refreshes windows, accounts for per-window
image uploads and drains workers/atlas cleanup during shutdown. A local macOS test
with focus disabled passes exact two-window GPU readback and verifies eviction by
forcing a same-ID diagnostic reupload with different pixels. It also passes close,
replacement and two-outstanding-job shutdown checks. The optional native-image-tests
feature/CI target adds test-only readback support; no hosted run or Linux GPU result
is claimed. Public OCaml image/icon views and SVG remain pending; see the
[asset evidence](evidence/assets-och11.md).

Declarative raster image views now work through the public scoped Asset/Bonsai/Eio
path, with immutable application-specific handles, fit/description configuration,
loading/ready/failure observations and native mounted leases. Pure owner/protocol/
reconciliation tests, native tree validation, full Dune/Rust workspace checks and
feature-enabled Clippy pass locally. A background macOS production-view test passes
exact GPU pixels, retirement/restyle/replacement/disposal and AXImage label checks;
the public example separately passes actual FFI event integration. See
[asset design](design/assets.md) and [asset evidence](evidence/assets-och11.md).
SVG/icons and the remaining OCH-11 families are still pending. CI definitions are
updated, but hosted/Linux gates and merge remain deferred until local scope is done.


SVG/icon rendering now passes local native GPU and public OCaml tests. SVG views
preserve color; icons tint the alpha mask with inherited foreground. Native resize
and hover select new size/density/fit/tint variants without an OCaml transaction,
including after registration retirement. The background production-view test passes
actual color/resize/tint pixels and immediate weak-binding cleanup; it uses synthetic
GPUI hover dispatch and does not claim physical monitor-scale changes. Decoder/cache
and cross-language Icon fixtures, full Rust workspace and Dune checks pass. Public
raster/SVG/icon example modes all pass lifecycle and FFI state integration. See
[SVG evidence](evidence/assets-och11.md#svgicon-integration--local-macos-continuation).
Remaining image clipping/composition and the other OCH-11 acceptance/gates remain.


Image corner propagation now passes an actual native regression: asymmetric raster
corners and changing icon hover radii clip the pixels while preserving center color,
image lifetime and AXImage semantics. See [clipping evidence](evidence/assets-och11.md#native-image-corner-clipping).
Icon/control composition and the remaining OCH-11 acceptance/gates are still pending.


Decorative leading/trailing button icons and labelled icon-only buttons now compose
with existing native activation/focus/accessibility. Core identity tests, native atomic
slot validation, actual GPU/AXButton and synthetic GPUI input/command-label checks pass.
Public raster/SVG/icon example modes pass with the new button compositions. See
[button icon evidence](evidence/button-icons-och11.md). OCH-11 remains in progress;
remaining theme/state/transitions, scrolling/lifetimes and consolidated gates remain
at that checkpoint.

Nested container scrolling now passes local native acceptance for transcript,
horizontal code, composer, Select popup and modal shielding. Same-node offsets
survive updates, wheel input leaves the tree revision unchanged, and removed
scroll owners dispose immediately. Native image/button and pointer regressions pass;
see [scrolling evidence](evidence/scrolling-och11.md). OCH-11 still needs the remaining
theme/state audit, basic transitions shared with OCH-12, aggregate lifetime review
and consolidated macOS/Linux gates and merge.

Native command routes now share immutable registry entries instead of cloning
label/shortcut payloads per button/menu/palette route. A 1,024-route lifetime test
checks sharing, stale-generation rejection and final-owner disposal; the full
native controls and image/button suites pass locally. See
[command lifetime evidence](evidence/command-lifetimes-och11.md). This closes the
identified command-payload duplication concern; remaining OCH-11 scope and hosted
gates are still pending.

OCH-12 now has validated OCaml animation configuration and a deterministic Rust
timing core. Tests cover delayed starts, paint-confirmed completion, interruption,
repetition, hidden/reduced-motion state and prepared-frame invalidation. The numeric
configuration has an independent OCaml/Rust binary fixture. No animation capability
is advertised: View/reconciliation/transport, GPUI scheduling, platform motion
preferences and actual native acceptance still need implementation. See
[animation design and current evidence](design/animations.md).

The OCH-12 view/bridge/rendering pipeline now passes local native and public checks.
`View.animate` retains node/run identity, delivers typed endpoints, and applies
Rust-computed values to GPUI. Actual native tests cover sidebar geometry without
inner reflow, interruption, native repetition, whole-window idle/hidden/reduced
behavior and delayed-task disposal. The public Bonsai/Eio example passes endpoints,
theme change and shutdown. Platform preference detection, application policy,
final capability advertisement and consolidated gates/merge remain pending; see
[animation design](design/animations.md).

Shared motion preferences now work through `App.run ~motion` and `App.set_motion`.
macOS uses a live NSWorkspace observer; Linux has an event-driven XDG Settings
adapter with bounded calls and documented unavailable-setting fallback. The native
macOS animation suite passes policy changes and a real notification/disposal check;
the public example passes immediate settling of a long animation under Reduce.
Portal protocol tests pass using a private mock connection on macOS. Final
capability/acceptance, hosted macOS/Linux gates and merge remain pending; see
[animation policy](design/animations.md#application-motion-preferences).

Consolidated local milestone 02 acceptance (2026-09-24): full Rust workspace,
Dune `@all @runtest @fmt`, and all-target feature-enabled Clippy pass. Native
animation, controls, progress, image/scale, drag/drop and AppKit file-dialog suites
pass. Public animation, drag/drop and file-dialog lifecycle examples pass with the
final capability mask. The animation test now activates its window: controlled
activation proved that a fully occluded background window was waiting for its
first frame. This change affects test reliability, not production window policy.
Hosted macOS/Linux validation and merge remain pending.
