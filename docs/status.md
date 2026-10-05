# Implementation status

Current handoff: 2026-10-05. Milestone **07 — Expanded v1 macOS validation and
release** is in progress. **OCH-41 and OCH-17 remain open.** This page separates
current work from historical checkpoints; it does not certify release readiness.

Work is tracked on branch `milestone-07-gallery-release` and draft PR #16.
Latest completed hosted run: [37286788836](https://github.com/dakotamurphyucf/gpuio/actions/runs/37286788836)
passes both foundation jobs and all three extracted apps on a fresh macOS runner.
The actual tested PR merge `689b3fc` has the same tree as branch head `56885cf`.
This covers the repaired audit lock resolution and the package/file-drop changes
at that checkpoint. Later local theme/profile/Metal/appearance/focus work still
requires its own hosted run. [Fresh-package evidence](evidence/package-runtime-och17.md#fresh-macos-receiver-qualification--2026-10-05)
records exact hashes and native coverage; final notice/signing/release approval
remains open. Required Linux checks pass; informational X11/Wayland smoke fails
and remains deferred to OCH-47.
The later hosted run [37297058441](https://github.com/dakotamurphyucf/gpuio/actions/runs/37297058441)
passed Linux but failed three macOS harness checks at tree-equivalent `999e531`;
the dependent fresh-package receiver was skipped. Local corrections now pass a
larger-text native document walkthrough and offline palette/memory artifact
replays. [Failure analysis and evidence](evidence/macos-ci-harness-och17.md)
retain the original failures; the corrected hosted run remains pending.
The later [run 37312985910](https://github.com/dakotamurphyucf/gpuio/actions/runs/37312985910)
passes Linux but fails the new macOS presentation hook: its Apple Paravirtual
GPU reports zero for all 180 presentation timestamps. The fresh-package receiver
is skipped. Independent standalone calibration was suppressed by that failure;
the workflow now preserves both probes' evidence without weakening either gate.
[Investigation](evidence/presentation-startup-investigation-och17.md#hosted-failure-is-distinct).
VoiceOver remains on the owner's explicit hold.
Three full optimized loaded-list, paged-table and growing-document runs pass
their declared budgets; six ordinary/profiled comparisons and three resource
lifecycle runs also pass. Three full streaming/typing runs also pass after an optional profiler repair,
with 1,200 native input samples per run and p99 below 10.5 ms.
Earlier evidence identifies local worktree checkpoints based on `83eb87e`;
their source snapshots must not be confused with that old HEAD alone.
The preceding 2,834-line chronological status is
preserved unchanged in [status-history.md](status-history.md); dated evidence files
retain exact commands, revisions, environments and limitations.

## Scope and platform policy

Follow [the platform policy](platform-release-policy.md): macOS native behavior,
accessibility, performance/resources and distribution remain milestone-07 gates.
Linux compilation, unit/private-bus tests and independent consumers remain required;
X11/Wayland GUI smoke is informational. Full Linux desktop qualification is **OCH-47
in deferred milestone 07b** and does not block this milestone or feature work.

Use the isolated stock OCaml 5.3/Bonsai v0.17/Core/Eio environment and the repository
commands. Do not change unrelated switches. See [CONTRIBUTING](../CONTRIBUTING.md),
[engineering standards](design/engineering-standards.md) and [development](development.md).
Agent notes belong in separate ignored `scratch/agents/<session>/<ticket>.md` files.

## Current implementation and evidence

The public gallery and reference applications exist, with public Core/Bonsai/Eio
APIs and native Rust ownership. Local source reviews and behavior evidence cover
many catalog additions; neither root-module coverage nor compilation proves
whole-family acceptance. Start from [the catalog](catalog/README.md),
[family ledger](catalog/families.json) and [gallery evidence](evidence/gallery-och41.md).

Recent completed local checkpoints:

- Rich-avatar GPU clipping, source/fallback ownership and cleanup now pass, as
  do avatar and Rating walkthroughs from a fresh installed gallery. Representative
  Light/Dark avatar palette pixels are checked on screen.
  [Evidence](evidence/avatar-native-consumer-och41.md).

- Avatar overflow now lays out after overlapping members following a scoped
  intrinsic-sizing repair in pinned Taffy. Engine regressions, 930 native tests,
  5,715 upstream tests and the real macOS gallery walkthrough pass.
  [Evidence](evidence/avatar-layout-och41.md).

- Numeric disabled part styles now admit and render the public Disabled tag;
  24 focused native tests and the physical rating gallery walkthrough pass,
  including 41 geometry/identity cases, themes and native input policies.
  [Evidence](evidence/numeric-disabled-rating-och41.md).

- Managed lifetime allocation now avoids one Bonsai state model/activation action
  per managed entry. Nested lifetime/remount tests and the full OCaml suite pass;
  traced initial table flush falls from 103.687 to 32.003 ms. Fresh untraced
  list/table/document smokes and the first full 10k-list/100k-table presentation
  runs pass, including 60-second idle and cleanup. Remaining repetitions and
  broader full qualification stay open.
  [Implementation and evidence](evidence/managed-lifetime-performance-och17.md).

- The first full optimized document presentation run passes 20 MiB growth,
  traversal/copy/cleanup and timing budgets. The table smoke fails its startup
  bound; temporary frame/task traces locate a gap between frames, outside the
  measured native draw/submission calls. Tracked source and normal executables
  are restored. Repetitions and diagnosis remain open.
  [Evidence](evidence/presentation-startup-investigation-och17.md).

- Three full optimized streaming/typing runs now pass actual paired Metal
  presentation checks: 1,200 OS keys per run while four streams update at 20 Hz,
  worst input-to-presentation p99 33.178 ms, peak RSS 143,589,376 bytes, exact
  content and cleanup. The initial input-free zero in each run remains recorded
  under the declared startup policy. Other presentation workloads and collector
  overhead/resources remain open. [Evidence](evidence/presentation-typing-full-och17.md).

- Presentation workload integration builds four instrumented executables and
  validates native/CPU pairing. Controlled native-only and standalone Metal probes
  reproduce initial zero-time frames after idle, excluding OCaml/bridge dependence.
  A dated startup-transition correction retains every skipped frame and adds a
  hard100 ms first-presentation bound; fresh list/typing smokes pass, including
  all40 native input pairs and zero idle work. Original strict failures remain
  recorded. Optimized repetitions and overhead/resources remain open.
  [Integration](evidence/presentation-workloads-och17.md),
  [idle isolation and revised smoke evidence](evidence/idle-presentation-och17.md). The
  [first full optimized list run](evidence/presentation-list-full-och17.md) completes
  all content/cleanup/idle checks and records26,587 presentations, but fails the
  startup bound at102.723 ms; it is not a passing repetition.

- Default-off presentation collection now includes the maintained Metal hook.
  Two real GPUI windows each pass90 presented frames with complete callback
  accounting and retired sessions. Native932 tests, strict lint, default builds,
  independent gallery consumer and exact reconstruction pass locally. Core
  attribution/clock/lifetime tests also pass. Repeated optimized workloads,
  physical input pairing, overhead/resource checks and new hosted checks remain
  open. [Core evidence](evidence/presentation-core-och17.md),
  [native hook evidence](evidence/metal-presentation-hook-och17.md).

- Typed application clipboard writes and a Bonsai Copy composition now have
  codec/state/clock and real macOS gallery evidence, including current Unicode
  values, native keyboard/AX, timed feedback and original clipboard restoration.
  [Clipboard evidence](evidence/clipboard-och41.md).

- Real macOS Japanese IME now has a passing public gallery sequence: OS candidate
  window, preedit versus committed form state, commit, one-step undo/redo and
  Escape cancellation. Input-source selection/enable states are restored and
  checked after the run. This is one single-line editor/method qualification;
  multiline geometry and other physical release checks remain.
  [IME evidence and recovery](evidence/macos-ime-och17.md).

- Three full optimized 10,000-row loaded-list runs pass predeclared budgets:
  each has over 26,000 draws, p95 below 3.13 ms, p99 below 3.76 ms, peak RSS
  below 185 MB, zero draws during 60 seconds of settled idle, bounded active
  rows and full cleanup. Profiler-overhead comparison, explicitly observed
  focused/unfocused idle and other workloads remain open.
  [Workload evidence](evidence/performance-collector-och17.md).

- An opt-in native histogram collector now has five focused tests, a full 925-test native suite (two existing skips), strict lint and default-feature compilation evidence. It adds no normal-app polling or redraws. Optimized workload measurements remain open. [Collector evidence](evidence/performance-collector-och17.md).

- Structural row-header ranges now work through external macOS AX queries. A selector-availability regression reproduces the prior omission; the scoped adapter repair, exact reconstruction and real table walkthrough pass. [Structural-table evidence](evidence/structural-tables-och41.md).

- Custom choice-picker empty instructions are now exposed to accessibility readers; native open/close/reopen checks and the full physical picker walkthrough pass. [Choice picker evidence](evidence/choice-picker-macos-och41.md).

- The public calendar-content wrapper now passes native tree admission after a physical gallery run exposed an empty style declaration on a structural slot. Shared OCaml/Rust transaction coverage and a passing physical picker rerun are recorded in [calendar content evidence](evidence/calendar-content-och41.md).

- Resumed physical macOS gallery testing: [forms and editor admission](evidence/gallery-editor-admission-och41.md) and [shimmer with bounded AX traversal](evidence/ax-traversal-och41.md) pass locally. Additional physical presentation checks, including tags and managed chat scrolling, pass at this follow-up. The broader gallery run and final release acceptance remain open.

- The [public document-profile macOS walkthrough](evidence/document-profile-macos-och41.md) now passes actual code/table keyboard actions, inline/block plugin navigation, pointer activation, property updates, remount and source release. Arbitrary plugin-owned scrolling and VoiceOver remain separately scoped.
- Static document profiles: checked SDK/catalogs, worker preparation, renderer
  attachment, queued events, a public OCaml/Rust package and independent installed
  consumer. [Renderer evidence](evidence/document-profile-renderers-och41.md),
  [package evidence](evidence/document-profile-package-och41.md).
- Application document defaults: inherited/built-in/explicit policy through both
  Eio runners and later windows. [Defaults evidence](evidence/document-defaults-och41.md).
- Virtual document control focus: offscreen and tall-block reveal, both traversal
  directions, composite exit and stale-request cancellation. Native 876/Base 235,
  lint, gallery, formatting and exact vendor reconstruction passed at that checkpoint.
  [Focus evidence](evidence/document-virtual-focus-och41.md).
- Document ownership: 36 window cycles and 808 block visits, source/profile/callback
  release, zero final parser reservations. Native 877 passed, two existing macOS
  private-bus skips, strict lint and formatting passed. This is TestPlatform
  ownership evidence, not process/GPU memory. [Resource evidence](evidence/document-resources-och17.md).
- Dependency notice collection: 144 OCaml package/root records and 499 text files;
  current native/Signal/gallery Rust graphs have 26 missing-text packages each after source-equivalent Pathfinder notices were collected and all 2,521 copied hashes verified.
  Runtime license retained; asset/system and final distribution review remain.
  [OCaml evidence](evidence/ocaml-notices-och17.md),
  [Rust evidence](evidence/dependency-notices-och17.md).
- Local reference bundle assembly and redirected-output repair:
  [packaging evidence](evidence/release-inputs-och17.md),
  [output adapter](evidence/output-sinks-och17.md). Local ad-hoc signatures and
  metadata exports do not establish clean-machine GUI or distribution acceptance.

These checkpoints do not automatically cover subsequent worktree changes. The
window family now includes the additive Minimize command, custom chrome, public
title-bar composition, guarded native gesture regions, live presentation snapshots
and supported client controls. Automatic client framing now has content-aware
overlay fitting, and metadata-only focused-input lookup now covers eligible native
text controls. Bounded window-wide selection helpers now inspect, clear and end
registered read-only selections without accessing editor values or the clipboard.
Native appearance snapshots now support the gallery’s Follow system option,
independently of explicit application palettes. Default monospace selection now prefers available native candidates once per
application and is shared by rich/source documents. Local checks pass: 904 native
tests with two existing skips, plus nine window-protocol checks. Full OCaml tests/gallery build,
strict native/protocol lint and formatting pass at this checkpoint.
Scoped physical OS results are recorded in the requirement table below; see [minimize evidence](evidence/window-minimize-och41.md),
[region evidence](evidence/window-regions-och41.md),
[presentation evidence](evidence/window-presentation-och41.md),
[frame evidence](evidence/window-frame-och41.md),
[focused-input evidence](evidence/window-input-query-och41.md),
[selection evidence](evidence/window-selection-och41.md),
[appearance evidence](evidence/window-appearance-och41.md),
[font evidence](evidence/default-fonts-och41.md) and
[the source review](catalog/window-review.md).

## Open milestone requirements

| Requirement | Current evidence and next required work |
| --- | --- |
| OCH-41: all required public families have a functional mapping and example | The ledger contains 41 v1 families, one post-v1 docking row and one excluded development-tool row. Review nested functionality, not just root ownership. Custom chrome/move/region integration now has [physical macOS evidence](evidence/window-lifecycle-och41.md): standard/custom minimize/restore, retained draft/selection/undo, native move, fullscreen restore, border resize and default double-click zoom pass; automatic tiling-aware client framing has local checks. Focused-input and bounded window-wide selection helpers have local native/codec/OCaml/gallery coverage. The [physical window-selection walkthrough](evidence/window-selection-och41.md#physical-public-gallery-walkthrough--2026-10-05) now passes exact Unicode read, end/clear, independent windows and retirement on native page unmount, with the clipboard unchanged. The [physical focused-input walkthrough](evidence/window-input-query-och41.md#physical-public-gallery-query--2026-10-05) now passes named controller matching, retained native editing, independent windows and remounts for ordinary/masked/read-only inputs. Geometry, runtime, native-event and diagnostic helpers now have detailed source reviews with API/evidence boundaries; the [nested style/theme review](catalog/style-theme-review.md) now records value/schema differences and the locally implemented file-theme example and native scrollbar snapshot. Default monospace selection has local policy/ownership and native regression evidence. Native appearance observation and the gallery Follow system option now have [local codec/native/OCaml evidence](evidence/window-appearance-och41.md); a [physical two-window macOS walkthrough](evidence/window-appearance-och41.md#physical-macos-appearance--2026-10-05) now passes system Light/Dark, independent overrides, retained editor selection/native typing/undo, and exact OS preference restoration. Automatic scheduling and other appearance/accessibility modes remain outside that check. |
| OCH-41: input/document contracts and lifecycle | Local implementations and tests exist. The [native range-helper repair](evidence/editor-range-geometry-och41.md) fixes reproduced multiline geometry and adds source/layout guards; its [public OCaml query](evidence/editor-range-api-och41.md), codecs/controller and gallery inspector are implemented with local Core/Eio/native coverage. Mounted document profiles now have [local Text/NonText/Opaque semantics evidence](evidence/document-profile-semantics-och41.md) for search paint, select-all copy and AX roles. Declared Text pointer selection and 96-passive-candidate focus traversal now have [local repair/stress evidence](evidence/document-profile-selection-och41.md). Independent plugin viewport clipping/exit/wheel reveal has [local and physical public-example qualification](evidence/document-profile-scroll-och41.md#public-scroll-profile--2026-10-05). The public scroll profile supplies keyboard-accessible start/end controls, native wheel reveal, clipped Tab exit, property retention and unmount retirement. Arbitrary plugin reveal remains plugin-owned; trackpad momentum and VoiceOver are separate. Do not revive already completed parser/profile/defaults work from old checkpoints. |
| OCH-41: runnable public gallery, themes/scales, keyboard and teardown | Public and installed-consumer builds have evidence at individual checkpoints. [Real macOS forms and editor-group checks](evidence/gallery-editor-admission-och41.md) now pass after fixing gallery editor-memory admission. The [native theme-file walkthrough](evidence/gallery-theme-files-och41.md#physical-macos-walkthrough-and-cancellation-repair--2026-10-05) now passes picker/reload, selection/undo, independent windows and delayed-read cancellation after a loading-status repair. The remaining expanded gallery physical walkthrough, visual review and native input/accessibility qualification remain. Keep production TestPlatform evidence separate from real desktop evidence. |
| OCH-17: real macOS IME, clipboard, focus, file drop, accessibility/VoiceOver and GPU | Historical scoped results exist. [Real multiline Japanese IME](evidence/multiline-ime-cancellation-och17.md) now passes wrapped and horizontally scrolled candidate/commit/undo/cancel checks after repairing cancellation rollback; the single-line Settings follow-up also passes at the same source. Other consolidated physical acceptance remains open. The source-view AX selection gap now has a [native repair and real macOS selection/range/copy checks](evidence/text-selection-och17.md), including Unicode, stale/disabled/composition rejection and password privacy. Rendered Markdown selection, visual character geometry and VoiceOver remain open. The owner paused all VoiceOver testing/configuration on 2026-10-05; continue other release work until that scope is reopened. Other open work includes point-based AX coverage beyond the now-passing two-window editor regression; the original routing gap was identified in [the diagnostics review](catalog/diagnostics-review.md); [window accessibility](evidence/window-accessibility-och17.md) and [gallery evidence](evidence/gallery-och41.md) retain limitations. |
| OCH-17: performance and resources | The [qualification plan](design/performance-qualification.md) now declares reference hardware, workload sizes and targets before optimized acceptance runs; the optional collector and three full optimized 10k loaded-list runs now pass locally. Six alternating ordinary/profiled comparisons pass, with median CPU +2.60% and RSS +1.60% in this workload; overlapping ranges are not a general overhead bound. The [paged-table workload](evidence/paged-table-performance-och17.md) exposed and now regression-tests compact-cell admission and horizontal visibility repairs; three full table runs now pass; explicit focused/unfocused idle is qualified separately below. Three optimized [resource-lifecycle runs](evidence/resource-lifecycle-och17.md) pass 3 warm-ups + 30 measured cycles with acknowledged registry cleanup and final-ten RSS growth below 64 MiB; native-entity and physical-memory evidence follow separately below. Three full optimized [growing-document runs](evidence/growing-document-och17.md) now pass 20 MiB aggregate growth and complete page traversal, with draw p95 ≤2.503 ms, p99 ≤3.638 ms and RSS ≤237,518,848 bytes; source-fallback and AX-selection boundaries remain explicit. Three full [streaming/typing runs](evidence/streaming-typing-och17.md) pass four 20 Hz streams with 1,200 native keys per run, exact content and cleanup, draw p99 ≤6.300 ms, input-to-submission p99 ≤10.429 ms and RSS ≤138,018,816 bytes. The [physical-memory audit](evidence/physical-memory-och17.md) subsequently found 2.52 GB retained after 33 closes despite the RSS pass; a macOS accessibility-adapter cycle repair now passes three full repeated audits, with no IOSurface category in all 99 closed checkpoints and peak settled physical footprint below 107 MB. Three optimized [explicit idle runs](evidence/idle-performance-och17.md) pass 60 seconds focused and 60 seconds unfocused each, with zero draws, verified editor blur and sampled native visibility. Three full [native entity audits](evidence/native-entity-retention-och17.md) pass all 90 post-warmup checks and 99 application retirement checkpoints, with final-ten RSS growth below 5.6 MB. Three full [renderer-device Metal allocation runs](evidence/metal-resource-och17.md) now pass on the M1 Max: all 99 closes retire their application resources, post-warmup closed allocations remain 4,587,520 bytes with zero final-ten growth, and closed IOSurface checks still pass. A [standalone Metal presentation calibration](evidence/metal-presentation-calibration-och17.md) now passes120 actual callbacks and establishes separate GPU/presentation/callback clocks. The [GPUI integration design](design/metal-presentation-qualification.md) and [actual renderer hook](evidence/metal-presentation-hook-och17.md) are implemented with local qualification. [Three streaming/typing presentation runs](evidence/presentation-typing-full-och17.md) pass; list startup, table/document repetition and presentation-collector overhead/resources remain open. Other release gates remain separate. Local ownership tests are only part of this gate. |
| OCH-17: chart delay investigation | The staged physical rerun now completes all 80 publications without the historical 63,050.99ms outlier. A controlled three-second minimize reproduces a 2.93-second publication→Ready wait, demonstrating visibility-sensitive readiness. The historical sample lacks evidence to prove that cause; optimized performance qualification remains. [Chart evidence](evidence/chart-streaming-och40.md). |
| OCH-17: initial black window | Reproduced historical startup capture; source ordering reviewed, diagnostic added. A fresh foreground run captured twelve nonblack UI samples; this does not rule out earlier blank frames or qualify background presentation. No production startup fix is claimed. [Startup evidence](evidence/window-startup-och17.md). |
| OCH-17: notices, source maintenance and distribution | All eight Bonsai-family packages now have [exact 2,348-entry reconstruction evidence](evidence/bonsai-reconstruction-och17.md). Complete remaining notice/asset/system review, final release-artifact qualification, versions/platform minimums and the chosen signing/distribution workflow. All three internal ad-hoc reference archives now pass on a separate hosted macOS receiver without project builds or dependency installation; [fresh-runner evidence](evidence/package-runtime-och17.md#fresh-macos-receiver-qualification--2026-10-05) records the actual source and hashes. All three reference applications now have [extracted-archive runtime evidence](evidence/package-runtime-och17.md) with development-directory access denied; a Copy-button contrast issue found during inspection is repaired. This is existing-Mac isolation with internal test notices, not fresh-machine or signed-release acceptance. [Distribution](distribution.md), [maintenance](component-adapters.md). |
| OCH-17: API/versioning, examples, review and publication | The [application guide](getting-started.md), [compatibility/limits](api-compatibility.md) and compiled starter now have [local installed-default-backend evidence](evidence/public-api-starter-och17.md). The [API boundary review](evidence/api-boundaries-och17.md) documents application versus integration layers and scoped-delivery contracts. Final whole-surface API/limitations review, required check results for the delivered sources, reviewed repository changes and release publication remain. Scratch artifacts are not release dependencies. |
| Required Linux nongraphical and hosted checks | [Run 37286788836](https://github.com/dakotamurphyucf/gpuio/actions/runs/37286788836) passes both foundation jobs and the separate macOS extracted-app receiver. The tested merge `689b3fc` has the same tree as head `56885cf`. Required Linux build/unit/private-bus/consumer checks pass; informational X11/Wayland GUI failures remain tracked in OCH-47. Later local changes need their own hosted run. [Detailed history and boundaries](evidence/milestone-07-ci.md). |
| Linear completion evidence | OCH-41/OCH-17 are In Progress in the latest read. Completion requires their actual acceptance criteria; no ticket may be closed from this summary alone. |

## Environment constraints and next action

On 2026-10-04 the owner supplied unrestricted filesystem access and enabled
network access. Archive downloads now work; all eight Bonsai-family packages
reconstruct exactly. macOS accessibility and screen-capture preflights pass.
The earlier access denials remain historical evidence, not current acceptance
results. Native testing and delivery can resume under the updated permissions;
do not substitute these preflights for the actual physical/release checks.

Use the current requirement table and its linked evidence when choosing the next
check. Historical test counts and earlier open items remain in the dated evidence
and [status history](status-history.md); later physical qualifications supersede
those earlier gaps only within their stated scope. In particular, theme-file
loading/cancellation, two-window appearance, plugin-owned document scrolling and
window lifecycle/input/selection now have scoped physical results. Do not repeat
these solely because an older checkpoint says they were untested.

VoiceOver remains on the owner's explicit hold. Continue other performance,
catalog, public API and release requirements. The held checks are not a reason to
claim completion or pause unrelated work. Current branch changes still need their
own required hosted checks and review before delivery.
