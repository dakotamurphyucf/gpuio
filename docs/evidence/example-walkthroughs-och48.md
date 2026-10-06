# Example walkthrough coverage — OCH-48

2026-10-06, source base `f73a8f9`, macOS 14.5 arm64. This checkpoint starts the
explicit documentation inventory; it does not complete OCH-48 or milestone 07.

The [inventory](../../examples/coverage.md) covers **417 OCaml/Rust source files
in 260 groups**, with each `.ml`/`.mli` implementation/interface pair owned
together and Rust sources classified separately. All groups remain visible,
including historical bootstrap, generated registration, extension-author,
benchmark, diagnostic and test support. Existing README presence is not treated
as a completed review. Build metadata and generated-output handling are explained
in the [review guide](../../examples/coverage-guide.md).

Four groups have been reviewed against their actual source and OCH-48's content
requirements:

- [Getting started](../../examples/getting_started/README.md): integer model,
  pure view versus Bonsai state, event/effect/update trace, startup/closure and
  a concrete decrement adaptation.
- [Embedded palette](../../examples/gallery/embedded_palette_preview.md): stable
  commands, persistent native query, model actions, native editor target,
  hide versus unmount and focus-command limitations.
- [External palette](../../examples/gallery/external_palette_preview.md): mock
  search, serial/query identity, Eio cancellation, accepted-view staging followed
  by guarded result publication, errors and bounded-result adaptation.
- [Preview scope](../../examples/gallery/preview_scope.md): activation acquisition,
  partial-failure cleanup, late-result suppression and application/page ownership.

The other **256 groups remain pending review**, including existing companions
that may already satisfy much of the checklist. The source index makes these
omissions reviewable; it is not a count of missing implementations.

`scripts/audit_example_docs.py` passes source ownership, path and generated-table
checks. It deliberately allows explicitly pending rows. It does not infer prose
quality from links or mark reviews automatically. A new required foundation step
runs this same check on macOS and Linux; hosted execution of that step is pending.
Contributor and agent instructions now require companion maintenance.

Local checks pass:

```sh
python3 scripts/audit_example_docs.py --write
python3 scripts/audit_example_docs.py
python3 -m py_compile scripts/audit_example_docs.py
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
GPUIO_JOBS=2 ./scripts/gpuio build examples/getting_started/main.exe
git diff --check
```

Relative file links in changed Markdown were checked for existing destinations.
This does not validate external URLs or every heading anchor. The gallery and
starter documentation use their existing launchers; no GUI test was repeated for
this prose/inventory-only change. Previous [starter/gallery behavior evidence](example-readability-och17.md)
and the scoped [embedded](palette-embedded-och41.md) /
[external](palette-external-results-och41.md) palette checks retain their original
source/platform limits. No new GUI, IME, VoiceOver, performance or Linux desktop
acceptance is claimed.

## Gallery application structure follow-up

Starting from `a370318`, five adjacent guides now explain six more source groups:
[`main`/`Application`](../../examples/gallery/application.md),
[`Component`](../../examples/gallery/component.md),
[`Shell`](../../examples/gallery/shell.md),
[`Pages`](../../examples/gallery/pages.md) and
[`Palette`](../../examples/gallery/palette.md). The application guide owns both
entry-point dispatch and startup; neither source part is omitted. The other
guides link their interfaces and supporting model sources without marking those
models' separate coverage rows complete.

The review covers initial/per-window/shared state, explicit Eio services, the
four-window demo bound and cleanup, native observations, theme synchronization,
pure layout/effect inputs, native-handshake capability gating, stable page keys,
branch deactivation and the local Presentation/Controls/Text editing examples.
It distinguishes Bonsai model retention from native resource retirement and
documents mock operations and discarded command results. Each guide gives a
concrete adaptation. README reading maps link all five companions.

Coverage is now **10 reviewed groups and 250 pending** out of the same 260;
the 417-file source inventory is unchanged. The earlier four-group checkpoint
above remains historical.

The source/table audit, gallery build and changed-Markdown file-link checks pass.
These documented non-GUI modes were also executed successfully:

```sh
./scripts/gpuio exec _build/default/examples/gallery/main.exe --check-catalogs
./scripts/gpuio exec _build/default/examples/gallery/main.exe --print-info-plist
```

The former reports `GALLERY_CATALOGS_PASS counter=1 document_profile=1`;
the latter's output passes `plutil -lint`. No native window was opened for this
documentation-only follow-up. The ongoing PR run `37447717604` covers `548bcde`,
not these newer docs/CI changes, so it cannot certify their new audit step.

## Theme loading and stale-result ownership follow-up

Starting from `8ba99e2`, the adjacent
[theme preview](../../examples/gallery/theme_preview.md),
[appearance model](../../examples/gallery/model/appearance.md),
[selection identity](../../examples/gallery/model/theme_selection.md),
[profile decoder](../../examples/gallery/model/theme_profile.md) and
[file adapter/test](../../examples/gallery/files/theme_file.md) guides explain
six additional source groups. The adapter guide explicitly owns its associated
expect-test source and Dune fixture setup; the interface files are linked too.

The review follows the actual one-request-at-a-time card, picker/read selection
fences, scope cancellation, retained draft and last-good palette. It explains
process-local token identity, the distinction between selected preference and
effective appearance, the 16 KiB/UTF-8/name/parsed-depth constraints, all ten
concrete color fields, bounded Eio reading and cancellation propagation. The
profile guide explicitly places the depth check after S-expression parsing.
These are application example contracts, not an upstream theme-format claim.

Coverage is now **16 reviewed groups and 244 pending**, with all 417 source files
still mapped. README and related guide links expose the complete theme path.
Inventory/table and changed-Markdown file-link checks pass, as does
`git diff --check`. The documented command also succeeds using existing Dune
build/test caching:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @examples/gallery/files/test/runtest
```

No implementation or tests changed. Earlier actual parser/filesystem/native
results retain the scope recorded in [theme-file evidence](gallery-theme-files-och41.md).
No GUI window was opened, and no additional IME, accessibility, performance or
Linux desktop acceptance is claimed by this documentation review.

## Parallel beginner walkthrough review

Starting from `1d623f7`, the owner authorized GPT-6.1 Sol agents to write separate
example guides in parallel. Three agents read their assigned implementation and
public interfaces, kept independent ticket notes and edited only their scoped
documentation. The primary agent reviewed their delivered guides and maintained
the shared source inventory. No agent ran competing builds or GUI tests.

Eighteen additional source groups are reviewed:

- Chart Studio's [application](../../examples/charts/main.md) and
  [index adapter](../../examples/charts/gallery.md), plus six pure sample pairs:
  [catalog](../../examples/charts/samples/gpuio_chart_samples.md),
  [categorical](../../examples/charts/samples/categorical.md),
  [stacked](../../examples/charts/samples/stacked.md),
  [ordinal](../../examples/charts/samples/ordinal_colors.md),
  [inspection](../../examples/charts/samples/inspection.md) and
  [Sankey presentation](../../examples/charts/samples/sankey_presentation.md).
- The [controls](../../examples/controls/main.md),
  [text input](../../examples/text_input/main.md),
  [combobox](../../examples/combobox/main.md) and
  [menus](../../examples/menus/main.md) applications.
- Agent Chat's [CLI](../../examples/agent_chat/main.md),
  [application](../../examples/agent_chat/application.md),
  [fake backend and its separate test group](../../examples/agent_chat/model/fake_backend.md),
  [message composition](../../examples/agent_chat/runtime/chat_message.md) and
  [motion](../../examples/agent_chat/runtime/chat_motion.md).

The guides teach concrete function/type/API names, state construction and
reactive syntax, effect execution, native ownership and representative event
traces. They distinguish synchronous Option syntax from Bonsai syntax, editor
seeds from guarded live commands, native query from application selection, and
conversation-scoped streams from mounted row views. Diagnostic flags and expert
hooks are identified rather than recommended as ordinary application structure.
Each companion is discoverable from its local README and provides an adaptation.

Coverage is **34 reviewed groups and 226 pending**, still covering all 417 source
files. This is documentation review, not a percentage of feature implementation.
Source/table audit, changed-Markdown local file links and `git diff --check` pass.
Agents checked documented commands against source/Dune/driver options; the
primary agent's full `dune build -j2 @all @runtest @fmt` also passes at this source
checkpoint. Native runs performed for the separate rich-label implementation are
recorded in [their own evidence](chart-node-labels-och41.md), not generalized to
these prose changes. Existing recorded platform limits remain in force.

## Scoped page/controller/conversation batch

Starting from `c22a61c`, the same three authorized documentation agents reviewed
eight further source groups: the gallery's [canvas](../../examples/gallery/canvas_page.md),
[assets](../../examples/gallery/assets_page.md),
[documents](../../examples/gallery/documents_page.md) and
[charts](../../examples/gallery/charts_page.md) pages; the positioned-menu
[launcher](../../examples/menu_controller/main.md),
[single-window component](../../examples/menu_controller/component.md) and
[multiwindow component](../../examples/menu_controller/multiwindow.md); and Agent
Chat's [conversation owner](../../examples/agent_chat/runtime/conversation.md).
Nested Phase/Message modules are explained in the conversation guide; separate
files are not invented. The primary agent reviewed the prose and updated the
shared inventory and README discovery links.

Coverage is now **42 reviewed groups and 218 pending**, still 417 source files.
The guides trace actual source publication, native commands and observations,
Bonsai state/effects, activation disposal, stream acceptance/cancellation and
resource ownership. They expose a confusing existing example detail: the Code
tab's append/reset controls currently mutate Markdown. The guide describes that
limitation without inferring design intent; it needs a separate example repair.
No such code repair is claimed by this documentation batch. The subsequent
[Code-tab repair](document-code-controls-och41.md) has separate local behavior evidence.

Relative local file links, source/table audit and whitespace checks pass. Agents
checked command names/flags against code/Dune/drivers and ran no builds or GUI.
The prior full repository build covers these unchanged implementations; current
Sankey ribbon-color work is separate and not certified by this prose review.
No new platform/input/performance acceptance is claimed.

## Pure models, bounded jobs and progress follow-up

Starting from `392dcc9`, seven more source groups are reviewed:
[Canvas_study](../../examples/gallery/model/canvas_study.md),
[Diff_state](../../examples/gallery/model/diff_state.md),
[Image_samples](../../examples/gallery/image_samples.md),
[Progress](../../examples/progress/main.md), and Agent Chat's
[Fixture_job](../../examples/agent_chat/runtime/fixture_job.md),
[Query_loading](../../examples/agent_chat/runtime/query_loading.md) and
[Result_actions](../../examples/agent_chat/runtime/result_actions.md).
The guides link their paired interfaces and concrete callers. Parent-page and
README links make the supporting modules discoverable.

These explain whole-scene validation, absolute transforms, exact managed/controlled
diff guards (without claiming general revision validation), binary PNM generation,
progress state/effects versus native motion, newest-pending CPU jobs, query versus
fixture loading, and row-membership/query/session guards around table actions.
Existing supporting test files are linked for their actual cases, without counting
them as newly reviewed source groups or claiming new test execution.

Coverage is **49 reviewed and 211 pending**, with 417 sources still mapped. The
structural audit, local Markdown paths and whitespace checks pass. Command/flag
review uses the actual source and manifests. This documentation batch changes no
implementation and runs no GUI; platform acceptance remains separately evidenced.

## Native drafts, confirmation and paged chat data

Starting from `9dc6328`, eleven further groups are reviewed: the numeric
[slider](../../examples/numeric/main.md), [number input](../../examples/numeric/number.md)
and [OTP](../../examples/numeric/otp.md); inline/popup
[calendar](../../examples/calendar/README.md) and
[color](../../examples/color_input/README.md) controls; and Agent Chat's
[Sources](../../examples/agent_chat/runtime/sources.md),
[Source_data](../../examples/agent_chat/runtime/source_data.md),
[Results](../../examples/agent_chat/runtime/results.md) and
[Result_data](../../examples/agent_chat/runtime/result_data.md).

The three authorized GPT-6.1 Sol agents read the implementations and interfaces
where present; the primary agent reviewed the guides, checked representative
source paths and updated the shared inventory. The guides distinguish native
editing from confirmed application state, keyed graph deactivation from removing
a view, command/session/revision guards, scoped lazy loading, complete-query
sorting before pagination and application approval of synthetic tree moves.
They name the actual helpers and public APIs and explain reactive/effect syntax.

Coverage is **60 reviewed groups and 200 pending**, still mapping 417 sources.
Local Markdown path checks, the source/table audit and whitespace checks pass.
Documented commands/flags were checked against source and build declarations.
This documentation review adds no GUI, accessibility or platform acceptance;
separate in-progress Sankey implementation checks are not evidence for these guides.

## Navigation, placement guards and accepted settings

Starting from `2afb197`, twelve more source groups are reviewed: navigation's
[entry point](../../examples/navigation/main.md),
[carousel](../../examples/navigation/carousel_lab.md) and
[sidebar icon loader](../../examples/navigation/sidebar_icons.md); gallery models
[Editor_visit](../../examples/gallery/model/editor_visit.md),
[Extension_state](../../examples/gallery/model/extension_state.md),
[Feedback_state](../../examples/gallery/model/feedback_state.md),
[Message_follow](../../examples/gallery/model/message_follow.md) and
[Message_stream](../../examples/gallery/model/message_stream.md); and Agent Chat's
[schedule data](../../examples/agent_chat/runtime/schedule_data.md),
[schedule UI](../../examples/agent_chat/runtime/schedule_settings.md),
[generation settings](../../examples/agent_chat/runtime/generation_settings.md) and
[annotation settings](../../examples/agent_chat/runtime/annotation_settings.md).

The same authorized agents read their complete scoped sources/interfaces and
relevant callers; primary review checked the prose and representative implementation
contracts. Local READMEs link each companion. The guides trace native retention,
Bonsai graph lifetime and scoped I/O separately; revision/generation/session guards;
message identity and follow overlays; draft versus applied dates/colors; and
accepted generation parameters captured by a later simulated send. Pure helpers
are not described as creating reactive graphs or doing I/O.

Coverage is **72 reviewed / 188 pending**, still 417 sources in 260 groups.
Relative file-link checks, the structural audit and whitespace checks pass.
Source/manifest/driver review checks command spelling and flags; no new build or
GUI run is claimed for this documentation batch. Existing tests mentioned by
these guides retain their separate scope and evidence.

Two implementation limits remain explicit: navigation's combined
`--carousel --self-test` mode omits editor placements required by the full-lab
self-test (a likely timeout, not reproduced here), and Extension_state.Depart
cannot advance an already exhausted Int64 generation. The ordinary navigation
self-test command is documented. Neither issue was repaired by this prose batch.

## Presentation composition, settings models and review controls

Starting from `72f4026`, fourteen more groups are reviewed: presentation's
[main](../../examples/presentation/main.md),
[avatar assets](../../examples/presentation/avatar_assets.md),
[avatar mode](../../examples/presentation/avatar_mode.md),
[content fixture](../../examples/presentation/content_cases.md) and
[rating reducer](../../examples/presentation/rating_action.md); gallery models
[Numeric_state](../../examples/gallery/model/numeric_state.md),
[Page](../../examples/gallery/model/page.md),
[Picker_state](../../examples/gallery/model/picker_state.md),
[Selection_state](../../examples/gallery/model/selection_state.md) and
[Settings_state](../../examples/gallery/model/settings_state.md); Agent Chat's
[Review](../../examples/agent_chat/runtime/review.md),
[Review_feedback](../../examples/agent_chat/runtime/review_feedback.md),
[Score_range](../../examples/agent_chat/runtime/score_range.md) and
[Responsive](../../examples/agent_chat/runtime/responsive.md).

The three authorized GPT-6.1 Sol agents read scoped sources/interfaces and
relevant callers. Primary review checked the guides and representative code/API
contracts. Explanations distinguish native retained editor state, application
intent and observations, committed versus draft settings, pure reducers, actual
Bonsai syntax/effects and native container-query selection. Extension consumption
is distinguished from extension authoring. READMEs link every companion.

Coverage is **86 reviewed / 174 pending**, still 417 sources in 260 groups.
Relative file links, the structural inventory audit and whitespace checks pass.
Command/flag review uses source/build/driver declarations. No fresh native or
platform acceptance is claimed by this prose batch. The separate document-page
repair has its own validation.

Presentation's combined `--content-check --self-test` similarly omits observations
required by its ordinary self-test; the guide documents separate commands rather
than claiming that combination passed. Settings_state's unlocked demo Custom
counter uses unsaturated int addition; its theoretical overflow limit is explicit.
Neither issue was reproduced or repaired in this documentation batch.

## Animation, canvas, presentation primitives and workspace assets

Starting from `efc1711`, seventeen further groups have adjacent reviewed guides:
animation/main, animation_program/main, container_query/main, canvas/main and
canvas/plot; gallery alert, aspect, attachment, avatar, badge, label and separator
previews; Agent Chat artifact_sidebar, artifact_tour, contributor_portrait, icons
and palette. Each companion is linked from its owning README and the
[coverage inventory](../../examples/coverage.md).

The three owner-authorized GPT-6.1 Sol agents read the scoped implementations,
interfaces and relevant callers. Primary review checked the guides against
implementation contracts. The explanations distinguish native animation clocks
from Bonsai effects, pure scene construction from scoped publication, carousel
requests from accepted selection, asset registration from asynchronous decoding,
and application state from native widget ownership. Exact commands and small
adaptations are included.

Coverage is **103 reviewed / 157 pending**, still 417 sources in 260 groups.
Relative file-link checks, the structural inventory audit and whitespace checks
pass. Commands were reviewed against source/build declarations; no build or GUI
run was added by this prose batch. Existing acceptance evidence remains separate.
The animation example's diagnostic event history and canvas selection's fixed-data
assumption are documented. The separator checkbox is independent demo state,
and its size bound is not described as an explicit clipping operation.

## Desktop input, command policies and complete workspace composition

Starting from `a9f7994`, sixteen further groups have reviewed adjacent guides:
commands, documents, drag_drop, drag_drop_desktop and file_dialogs entry points;
gallery binding, button appearance, rich button, checkable navigation, command
tooltip and content hint previews; Agent Chat diagram, run_diagram, inspector,
settings and workspace. Owning READMEs link each companion. A small spacing
correction also improves the existing animation diagnostic explanation.

The same three authorized GPT-6.1 Sol agents read their complete scoped source/
interfaces, callers and relevant contracts. Primary review checked all guides and
representative implementations. Gallery explanations were expanded during review
to define reactive state, latest-model reducers and deferred effects, with concrete
native-event → effect → model → view → native-update traces. The workspace guide
separates shared conversations from retained per-window composers/lists, native
submission/conditional clear, attachment scopes and deferred close decisions.

Coverage is **119 reviewed / 141 pending**, still 417 sources in 260 groups.
Primary validation passes 346 relative file targets across 23 guide/README files,
the structural inventory audit and whitespace checks. Agents also checked scoped
links/anchors. Build/run and harness commands were reviewed against declarations
and drivers; no new build, native window or platform acceptance run was performed.

Desktop drag and picker-read diagnostics explicitly require their real macOS
drivers. File picker selection is distinct from filesystem authority or save
writing. The simple file reader has no latest-request guard for overlapping reads;
the drag fixture's two self-test flags start independent tasks and are documented
separately. Diagram local scene admission is distinguished from later native
rejection, which this helper does not poll/retry. These limits were documented,
not repaired or newly reproduced by the prose review.

## Notifications, composition and Signal Studio ownership

Starting from `bb24014`, sixteen further source groups are reviewed: images,
notification, overlays, toasts and tooltips entry points; gallery description,
disclosure, empty state, form, group and link previews; Signal Studio workspace
model and its expect tests, application identity, documents and UI-thread adapter.
The model and tests share a guide that explains both sources. READMEs link every
companion. The notification README's obsolete Linux Unsupported statement was
corrected against the current freedesktop contract without claiming desktop
qualification.

The three authorized GPT-6.1 Sol agents read their full sources/interfaces and
relevant callers/contracts. Primary review checked the guides and representative
source contracts. Explanations cover reactive syntax/actions, native resource
retention, hidden versus unmounted content, simulated actions, filesystem
capabilities, submitted-save snapshots and epoch-fenced replies.

Coverage is **135 reviewed / 125 pending**, still 417 sources in 260 groups.
Primary checks pass 269 relative file targets across 22 guide/README files,
structural inventory and whitespace. Commands were checked against source/build/
driver declarations. No new example build, GUI or platform acceptance is claimed
from this batch; native geometry tests performed separately have their own scope.

Signal Studio's `Ui_thread.perform` requires active-scope and caller cancellation/
timeout ownership: inactive-scope enqueue alone cannot resolve its waiting promise.
Actual application/check callers provide the documented scope and timeout. Document
reset fences acceptance but does not cancel an in-progress file write. These are
source-level limitations and semantics, not newly reproduced or repaired bugs.

## Pointer, managed collections and Signal Studio composition

Starting from `9b4c5fd`, sixteen further source groups have reviewed companions:
pointer, virtual list and window lifecycle entry points; table main and event
actions; gallery formatting, horizontal lists, pagination, progress, spinner and
text shimmer; Signal Studio main, component, UI, document-file adapter and its
expect tests. The file adapter guide owns both implementation and test groups.
Owning READMEs link all companions.

The three authorized GPT-6.1 Sol agents read complete scoped sources/interfaces
and relevant callers/contracts. Primary review checked every delivered guide and
representative implementations. The guides trace concrete native events, effects,
Bonsai state changes and view derivation. They distinguish durable conversation
state from transient rows, captured setter values from latest-model reducers,
editor configuration from guarded replacement, and native ownership from file
capabilities and deferred close decisions.

Coverage is **151 reviewed / 109 pending**, still 417 sources in 260 groups.
Primary checks pass 267 relative file targets across 21 guides/READMEs, the
structural inventory audit and whitespace. Commands were checked against source,
Dune and driver declarations; this batch adds no native/platform acceptance.
The separately running chart implementation tests have their own scope.

Pointer's self-test does not inject drags; window lifecycle is scripted even in
its default mode. Table's direct-effect checks differ from the physical AppKit
harness. Shimmer's experimental status remains explicit. Document-file tests
cover actual rename failure and symlink replacement, but not cancellation or
crash durability. These limits remain visible to new readers.

## Tree approval, guarded editing and remaining Signal Studio runtime

Starting from `3d21215`, twenty further source groups are reviewed: all eight tree
modules; gallery slider, workflow stepper, numeric draft, password, multiline and
prepared-filter previews; Signal Studio application, checks, notification controller
and tests, and both generated backend registration sources. Nineteen companions
cover these groups; notification tests share the complete controller guide.
Owning READMEs expose the reading order.

The three authorized GPT-6.1 Sol agents read full scoped sources/interfaces and
relevant contracts/callers. Primary review checked all delivered guides and
representative source implementations. Editing guides were expanded to introduce
actual aliases, graph ownership and reactive syntax before their concrete event
traces. Generated registration is explicitly maintainer infrastructure, separate
from ordinary OCaml view construction. Tree guides distinguish immutable hierarchy
approval, transient row lifetime, native proposals and scoped filesystem loading.

Coverage is **171 reviewed / 89 pending**, still 417 sources in 260 groups.
Primary checks pass 322 relative file targets across 22 guides/READMEs, structural
inventory and whitespace. Agents additionally checked their scoped links/anchors.
Commands were reviewed against source/build/harness declarations. This prose batch
adds no newly executed native, permission, IME or platform qualification.

The guides preserve material limits: tree offset paging is not a directory
snapshot; Support.perform needs caller-owned cancellation/deadlines; slider Reset
is intentionally unconditional; notice epochs do not undo native commands;
notification handlers depend on serial delivery; a receipt is not proof of a
visible banner. These are documented contracts/limitations, not claims of newly
reproduced or fixed bugs.

## Standalone, retained-layout and desktop walkthroughs

Source base `f170dd3`: 18 further groups reviewed, for **189 reviewed / 71 pending**
of 260 groups (417 sources). Three authorized GPT-6.1 Sol agents wrote the guides;
primary review checked the prose and representative source/interface contracts.

- Standalone `view_api` entry point, shared views and compiled Bonsai sample;
  palette, desktop and finite chart-stream entry points.
- Gallery split/group, carousel track, scrollbar, structural table and retained
  tab content previews.
- Desktop page/session, settings preview, Eio settings writer and its expect tests,
  and the packaged counter's extensions page.

Each guide explains actual aliases, reactive syntax, event/effect flow, native
ownership, launch prerequisites and a concrete adaptation. Owning READMEs link the
guides. The settings writer guide also owns its independent test source row.

Review found that `view_api`'s `native_selection=true` self-test marker is not
supported by a generated selection action/assertion. Its walkthrough explicitly
limits the self-test to bridge/frame updates; correcting that legacy diagnostic
marker remains an OCH-17 evidence task. The compiled `Bonsai_component` is not
mounted by this entry point. Neither compilation nor those markers establishes
interactive Bonsai, selection, IME or platform acceptance.

Structural inventory, local Markdown file links and whitespace checks pass.
No executable command or GUI test was repeated for this documentation batch;
concurrent chart validation is recorded separately. OCH-48 remains incomplete.

## Upload diagnostics, pickers and selection compositions

Source base `66dad21`: another 18 groups bring coverage to **207 reviewed / 53
pending** (417 sources, 260 groups). Three authorized agents contributed separate
six-group batches; primary review read every guide and checked representative
source/interface contracts. The earlier layout guides also received typography
and code-identifier formatting improvements.

New coverage includes asset/canvas/chart upload diagnostics, foundation main/native/
wire, grouped picker and edge cases, indicator appearance, keyboard/marker/tag
presentation, message composition/managed chat, popup placement, selectable-list
search, toolbar selection and window read-only text selection. Owning READMEs link
each companion. Guides distinguish pure views, Bonsai state/effects, native retained
resources and Eio scopes, with concrete traces and adaptations.

The diagnostic guides explicitly explain that encoded publication is not decoding
or rendering, the foundation protocol is private/historical, its smoke wrapper
sets the fixture working directory, and its early SELF_TEST_PASS marker does not
replace normal exit and final cancellation assertions. No new graphical acceptance
is inferred from these examples or this review.

The structural inventory, changed Markdown local file links and whitespace checks
pass. Documentation-only commands were checked against source/build declarations;
no new example GUI executions were run for this batch. OCH-48 remains open.

The follow-up diagnostic correction changes `view_api`'s marker to
`native_selection=not_exercised`, matching the walkthrough and actual self-test
scope. This changes reporting only; no new selection test or GUI acceptance is
claimed. Consumers of the existing `TYPED_VIEW_PASS` prefix remain unchanged.

## Page composition and runtime diagnostics

Source base `5acfb5f`: 18 further groups bring coverage to **225 reviewed / 35
pending** of 260 groups (417 sources). The three authorized agents wrote six
guides each; primary review read every guide and checked representative source/
interface/native contracts. All owning READMEs link their component guides.

This batch covers bridge/runtime main and both benches, Agent Chat self-test and
metrics, and gallery Collections, Feedback, Journeys, Navigation, Overlays,
Pickers, Highlight, Input, Motion, Numeric, Observations and Responsive pages.
The guides explain actual independent models/controllers and composition, native
versus application-owned behavior, event/effect traces, runnable entry points and
adaptations. Runtime benchmarks explicitly distinguish simulated acknowledgement,
CPU/allocation counters and frame notifications from physical input/presentation
or whole-process memory. Diagnostic success markers are qualified by their actual
assertions and terminal cleanup. The chat README's workload delay is corrected to
the application's current five-second configuration.

Review also corrected two stale Highlight/View interface comments: native text/
document highlighting is implemented, as the existing host paths and recorded
native evidence demonstrate. No separate highlight capability is advertised; this
comment correction adds no capability, runtime behavior or platform acceptance.

Structural inventory, local file links in changed example Markdown, whitespace,
and ocamlformat checks on the two comment-only interfaces pass. No GUI execution
was repeated for this prose batch. OCH-48 remains open, with extension-author,
performance/resource and remaining gallery source groups visible as pending.

## Extension packages, final gallery pages and workload ownership

Source base `8441661`: another 18 source groups reviewed, for **243 reviewed / 17
pending** of 260 groups (417 sources). Three authorized agents read full scoped
sources/interfaces/callers/contracts; primary review read all guides and inspected
representative package/viewport/cache interfaces and implementations.

New guides cover the counter/document profile author packages and profile expect
test, extension consumer and generated backend, remaining four gallery components,
paged-table driver/cache, streaming driver/model and shared lifecycle workload.
Each distinguishes ordinary OCaml consumers from native extension authors or
qualification instrumentation, and defines actual measurement/ownership limits.

A discovery audit found 53 already-reviewed guides reachable from their own README
but not the main examples index without using the inventory. The index now links
all existing example families by purpose. The structural audit now requires prose
link paths from both root and owning README, excluding generated coverage/code
blocks. Replaying the previous index finds those 53 omissions; the current index
passes. This strengthens discoverability without pretending to check prose quality.

Inventory/discovery, local Markdown file links and whitespace pass. Both example
package schema fingerprints were compared with OCaml and Rust declarations. No
new builds or graphical runs were performed for the prose-only changes; existing
scoped evidence and build declarations retain their limits. OCH-48 stays open.


## Completed example documentation acceptance

Source base `d80e7b3`, 2026-10-06: the final 17 groups bring the inventory to
**260 reviewed / 0 pending**, covering all **417 tracked OCaml/Rust source files**.
Three owner-authorized GPT-6.1 Sol agents drafted disjoint source walkthroughs;
the primary agent read every final guide and reconciled review corrections before
marking its inventory row reviewed. Earlier sections preserve the batch history.

The final batch covers resource-audit OCaml/native/Metal ownership and registration,
frame/document/idle/lifecycle workloads, presentation-specific generated backends,
and the performance probe's codecs, native phases and bounded Metal observations.
These guides distinguish ordinary OCaml application usage from extension authorship
and qualification instrumentation. Review corrected a generator command to use a
fresh output directory, the lifecycle budget's baseline to the first of the final
ten checkpoints, and compilation versus runtime schema-validation claims.

Acceptance review:

- Each inventoried source has explicit ownership and a reviewed adjacent companion
  (interfaces and tightly related test/support modules share explanations where
  appropriate). No historical/generated/native diagnostic category is hidden.
- The main examples index and each owning README provide prose-link reading paths
  to every guide. The structural audit enforces both without counting the coverage
  table as navigation. Newcomers have the small counter as a starting point.
- Guides explain actual named functions/types, Bonsai state/effects/reactive syntax,
  GPUIO views/controllers and Eio/native lifetimes where used, with concrete traces
  and adaptations. Pure/synchronous examples explicitly avoid invented layers.
- Launch/build commands were checked against repository wrappers, Dune/Cargo targets
  and collectors. Prior starter/gallery executions and the recent full isolated
  `dune build @all @runtest @fmt -j 2` remain scoped evidence; see
  [the source validation checkpoint](sankey-label-gallery.md). No benchmark budgets
  or new graphical acceptance are inferred from reviewing commands.
- Contributor/agent guidance requires adjacent-guide maintenance. All 260 reviews
  were explicit content reviews; structural success alone cannot approve prose.

Final local checks pass: `python3 scripts/audit_example_docs.py --write`,
`python3 scripts/audit_example_docs.py`, `python3 -m py_compile
scripts/audit_example_docs.py`, and `git diff --check`. The separate Markdown scans
resolved **3,307 local file targets** and **41 local heading anchors**, with no
missing destinations. External URLs were not comprehensively checked. No new
builds, GUI tests or VoiceOver sessions ran for this final prose batch.

OCH-48 documentation acceptance is complete. OCH-41, OCH-17 and milestone 07 remain
open. The separate Metal plan now reflects the owner's October 5 VoiceOver
authorization; this correction is not accessibility test evidence. Hosted run
37460415879 covers older PR head `75ce53d`: Linux passed, the macOS foundation job
failed Signal Studio responsive/input, Agent Workspace Diagram and both Metal
presentation checks; fresh extracted-app checks were still running at this
checkpoint. Their failures and remaining release gates are not waived by docs.
