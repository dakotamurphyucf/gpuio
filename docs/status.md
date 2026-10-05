# Implementation status

Current handoff: 2026-10-04. Milestone **07 — Expanded v1 macOS validation and
release** is in progress. **OCH-41 and OCH-17 remain open.** This page separates
current work from historical checkpoints; it does not certify release readiness.

Work is tracked on branch `milestone-07-gallery-release` and draft PR #16.
The last completed hosted run used `6d6d96f`: Linux passed, macOS failed.
All sixteen failed macOS steps now have passing local reruns; fresh hosted
qualification remains required. The first full optimized loaded-list run also
passes its declared budgets, with repeated/other workload acceptance still open.
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

- Real macOS Japanese IME now has a passing public gallery sequence: OS candidate
  window, preedit versus committed form state, commit, one-step undo/redo and
  Escape cancellation. Input-source selection/enable states are restored and
  checked after the run. This is one single-line editor/method qualification;
  multiline geometry and other physical release checks remain.
  [IME evidence and recovery](evidence/macos-ime-och17.md).

- The first full optimized 10,000-row loaded-list run passes predeclared budgets:
  26,881 draws, p95 3.13 ms, p99 3.74 ms, peak RSS 184.5 MB, zero draws during
  60 seconds of settled idle, bounded active rows and full cleanup. Repeated runs,
  profiler-overhead comparison and other workloads remain open.
  [Workload evidence](evidence/performance-collector-och17.md).

- An opt-in native histogram collector now has five focused tests, a full 925-test native suite (two existing skips), strict lint and default-feature compilation evidence. It adds no normal-app polling or redraws. Optimized workload measurements remain open. [Collector evidence](evidence/performance-collector-och17.md).

- Structural row-header ranges now work through external macOS AX queries. A selector-availability regression reproduces the prior omission; the scoped adapter repair, exact reconstruction and real table walkthrough pass. [Structural-table evidence](evidence/structural-tables-och41.md).

- Custom choice-picker empty instructions are now exposed to accessibility readers; native open/close/reopen checks and the full physical picker walkthrough pass. [Choice picker evidence](evidence/choice-picker-macos-och41.md).

- The public calendar-content wrapper now passes native tree admission after a physical gallery run exposed an empty style declaration on a structural slot. Shared OCaml/Rust transaction coverage and a passing physical picker rerun are recorded in [calendar content evidence](evidence/calendar-content-och41.md).

- Resumed physical macOS gallery testing: [forms and editor admission](evidence/gallery-editor-admission-och41.md) and [shimmer with bounded AX traversal](evidence/ax-traversal-och41.md) pass locally. Additional physical presentation checks, including tags and managed chat scrolling, pass at this follow-up. The broader gallery run and final release acceptance remain open.

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
  current native/Signal/gallery Rust graphs have 27 missing-text packages each.
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
Actual OS operation remains open; see [minimize evidence](evidence/window-minimize-och41.md),
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
| OCH-41: all required public families have a functional mapping and example | The ledger contains 41 v1 families, one post-v1 docking row and one excluded development-tool row. Review nested functionality, not just root ownership. Custom chrome/move/region integration has local evidence; automatic tiling-aware client framing has local checks. Focused-input and bounded window-wide selection helpers have local native/codec/OCaml/gallery coverage. Geometry, runtime, native-event and diagnostic helpers now have detailed source reviews with API/evidence boundaries; the [nested style/theme review](catalog/style-theme-review.md) now records value/schema differences and the locally implemented file-theme example and native scrollbar snapshot. Default monospace selection has local policy/ownership and native regression evidence. Native appearance observation and the gallery Follow system option now have [local codec/native/OCaml evidence](evidence/window-appearance-och41.md); physical OS appearance switching remains unverified. |
| OCH-41: input/document contracts and lifecycle | Local implementations and tests exist. The [native range-helper repair](evidence/editor-range-geometry-och41.md) fixes reproduced multiline geometry and adds source/layout guards; its [public OCaml query](evidence/editor-range-api-och41.md), codecs/controller and gallery inspector are implemented with local Core/Eio/native coverage. Mounted document profiles now have [local Text/NonText/Opaque semantics evidence](evidence/document-profile-semantics-och41.md) for search paint, select-all copy and AX roles. Declared Text pointer selection and 96-passive-candidate focus traversal now have [local repair/stress evidence](evidence/document-profile-selection-och41.md). Independent plugin viewport clipping/exit/wheel reveal now has [local qualification](evidence/document-profile-scroll-och41.md); inner keyboard reveal remains plugin-owned and physical qualification remains open. Do not revive already completed parser/profile/defaults work from old checkpoints. |
| OCH-41: runnable public gallery, themes/scales, keyboard and teardown | Public and installed-consumer builds have evidence at individual checkpoints. [Real macOS forms and editor-group checks](evidence/gallery-editor-admission-och41.md) now pass after fixing gallery editor-memory admission. The remaining expanded gallery physical walkthrough, visual review and native input/accessibility qualification remain. Keep production TestPlatform evidence separate from real desktop evidence. |
| OCH-17: real macOS IME, clipboard, focus, file drop, accessibility/VoiceOver and GPU | Historical scoped results exist. Consolidated final-source physical acceptance remains open, including point-based AX coverage beyond the now-passing two-window editor regression; the original routing gap was identified in [the diagnostics review](catalog/diagnostics-review.md); [window accessibility](evidence/window-accessibility-och17.md) and [gallery evidence](evidence/gallery-och41.md) retain limitations. |
| OCH-17: performance and resources | The [qualification plan](design/performance-qualification.md) now declares reference hardware, workload sizes and targets before optimized acceptance runs; the optional collector and first full optimized 10k loaded-list run now pass locally. Repeat/overhead checks remain required. Also cover 100k logical paged rows, growing documents, concurrent streams while typing, full-history traversal, repeated windows and idle redraws. Local ownership tests are only part of this gate. |
| OCH-17: chart delay investigation | The staged physical rerun now completes all 80 publications without the historical 63,050.99ms outlier. A controlled three-second minimize reproduces a 2.93-second publication→Ready wait, demonstrating visibility-sensitive readiness. The historical sample lacks evidence to prove that cause; optimized performance qualification remains. [Chart evidence](evidence/chart-streaming-och40.md). |
| OCH-17: initial black window | Reproduced historical startup capture; source ordering reviewed, diagnostic added. A fresh foreground run captured twelve nonblack UI samples; this does not rule out earlier blank frames or qualify background presentation. No production startup fix is claimed. [Startup evidence](evidence/window-startup-och17.md). |
| OCH-17: notices, source maintenance and distribution | All eight Bonsai-family packages now have [exact 2,348-entry reconstruction evidence](evidence/bonsai-reconstruction-och17.md). Complete remaining notice/asset/system review, clean-machine source and packaged GUI execution, final versions/platform minimums and the chosen signing/distribution workflow. Existing assembly/inventory success is insufficient. [Distribution](distribution.md), [maintenance](component-adapters.md). |
| OCH-17: API/versioning, examples, review and publication | The [application guide](getting-started.md), [compatibility/limits](api-compatibility.md) and compiled starter now have [local installed-default-backend evidence](evidence/public-api-starter-och17.md). The [API boundary review](evidence/api-boundaries-och17.md) documents application versus integration layers and scoped-delivery contracts. Final whole-surface API/limitations review, required check results for the delivered sources, reviewed repository changes and release publication remain. Scratch artifacts are not release dependencies. |
| Required Linux nongraphical and hosted checks | Run 37236900248 got past the missing test-feature guard: macOS unit/lint passes, then the standalone table test build fails on an observation-wrapper type; Linux aborts in a sidebar fixture with stack overflow. Table repair now passes its independent native build, strict lint and native fixture locally. Plain/image/canvas native feature checks also pass after four further test-module guard corrections. Run 37239659043 reproduces the Linux failure; GDB identifies finite recursive renderer stack pressure and the 8 MiB comparison passes. The component-builder extraction reduces the common arm64 debug stack frame by about 44% and passes 920 native tests plus strict lint. Run 37241646017 at 6d6d96f is terminal: the entire Linux job passes; macOS reports 16 failed steps. All sixteen failed steps now have passing local reruns after the [macOS repair batch](evidence/macos-validation-repairs-och17.md); fresh hosted checks are required. No overall hosted pass is claimed. Preserve required builds/unit/private-bus/consumer checks and report informational GUI failures separately. [Hosted evidence](evidence/milestone-07-ci.md). |
| Linear completion evidence | OCH-41/OCH-17 are In Progress in the latest read. Completion requires their actual acceptance criteria; no ticket may be closed from this summary alone. |

## Environment constraints and next action

On 2026-10-04 the owner supplied unrestricted filesystem access and enabled
network access. Archive downloads now work; all eight Bonsai-family packages
reconstruct exactly. macOS accessibility and screen-capture preflights pass.
The earlier access denials remain historical evidence, not current acceptance
results. Native testing and delivery can resume under the updated permissions;
do not substitute these preflights for the actual physical/release checks.

Native scrollbar preference snapshots and the explicit Collections gallery action
now have [local evidence](evidence/scrollbar-preference-och41.md): 906 native tests
(two existing skips), 390 protocol tests and the full OCaml/gallery build pass. Linux returns
Unsupported because the pinned backend has no real preference source. The public
editor range query now passes 914 native tests with two existing skips, 392 protocol
tests, the full OCaml/gallery build, strict lint and formatting. The subsequent
[document profile semantics checkpoint](evidence/document-profile-semantics-och41.md)
adds production-host search paint, select-all copy and AX-role coverage; 915 native
tests pass with the same two skips. The subsequent [selection repair](evidence/document-profile-selection-och41.md)
passes 918 native and 238 Base text tests, with exact 235-file reconstruction.
Its 15 document SDK tests, strict lint, formatting and full OCaml/gallery build
also pass. Continue the physical/release acceptance gaps above, final public API
review and physical plugin-scroll qualification. Independent inner viewport clipping, focus exit, wheel reveal and stamped activation now have [local coverage](evidence/document-profile-scroll-och41.md); this does not supply automatic keyboard reveal inside arbitrary plugin scroll areas.
The preceding native helper repair has 268 Base input-related tests and exact
235-file reconstruction evidence. The [file-theme gallery example](evidence/gallery-theme-files-och41.md)
now passes local parser/Eio/selection/reconciliation tests and the full OCaml/gallery build;
its physical picker/reload walkthrough remains open.
Default monospace selection is locally validated. [Native appearance observation](design/window-appearance.md)
is implemented and locally validated; actual OS switching remains a physical check. Follow the
explicit acceptance gaps in the [geometry](catalog/geometry-review.md),
[runtime](catalog/runtime-helpers-review.md), [event](catalog/native-events-review.md)
and [diagnostics](catalog/diagnostics-review.md) reviews. Window helper mappings have local evidence in
[catalog/window-review.md](catalog/window-review.md); physical acceptance remains open. Do not mark the milestone complete
until every open requirement above has direct evidence at the delivered sources.
