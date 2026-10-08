# Implementation status

Current handoff: 2026-10-08. Milestone
**07 — Expanded v1 macOS validation and release** is active. **OCH-41 and OCH-17
remain In Progress.** OCH-48 example walkthrough acceptance is complete.
The owner’s 2026-10-08 direction makes the next delivery a **developer preview**
for real-user feedback. Broader qualification and signed-app distribution move
to [OCH-164](https://linear.app/ochat/issue/OCH-164); see the closeout checklist.
This page records current work; it does not certify release readiness.

Start with the [milestone 07 closeout checklist](milestone-07-closeout.md) for
the remaining work, dependencies, execution order and rules for reusing evidence.
The detailed entries below are historical and scoped evidence, not a queue of
new tasks to repeat.

<a id="scope-and-platform-policy"></a>

Earlier local repair: flow-layout document plugins now exclude controls clipped
by their own scroll viewports from keyboard traversal. A failing regression and
Comfortable/Large native walkthroughs verify the fix; see
[focus-clipping evidence](evidence/document-profile-flow-focus-och17.md).
Hosted validation of this batch remains pending.

## Scope and working rules

[Platform policy](platform-release-policy.md): the current preview checklist
supersedes the earlier stable-release gate assignments. Linux build, unit,
private-D-Bus and independent-consumer checks remain required. X11/Wayland GUI
smoke is informational; full desktop qualification is OCH-47 in deferred **07b**
and does not block milestone 07, milestone 08 or feature work. No VM is required.

Use the isolated stock OCaml 5.3/Bonsai v0.17/Core/Eio toolchain. Follow
[CONTRIBUTING](../CONTRIBUTING.md), [engineering standards](design/engineering-standards.md)
and [development](development.md); do not mutate unrelated switches. Keep personal,
per-ticket recovery notes under ignored `scratch/agents/<session>/`.

Local foreground GUI/focus/IME testing is authorized; prefer background windows
when appropriate. VoiceOver testing, configuration and automation are authorized
again by the owner's 2026-10-05 revision; restore temporary settings afterward.
Older records of a VoiceOver hold no longer apply. Filesystem/network access was
restored on 2026-10-08; GitHub reads and Linear updates have been verified again.
The preview remains unpublished pending final candidate checks and delivery.

<a id="current-implementation-and-evidence"></a>

## Latest delivered behavior and validation

The [preview coverage reconciliation](catalog/preview-coverage.md) closes C1:
41 required families and five accepted additions have public example routes and
scoped behavior evidence. No new missing-family implementation blocker was found.
The [list comparison](evidence/preview-list-comparison-och17.md),
[scroll assessment](evidence/preview-scroll-assessment-och17.md) and
[streaming/resource impact review](evidence/preview-performance-impact-och17.md)
close P1/P2 for the focused preview: three full current list trials, measured
instrumentation cost, and targeted checks supporting reuse of earlier full
streaming/resource evidence. The original transient visual report remains a
follow-up without an original-cause/fix claim. The source notice/artifact and installation handoffs are now complete for the
preview; final candidate integration/publication remain open.

Foundation `37805065605` is terminal: Linux and the fresh macOS extracted-app
receiver pass; the macOS foundation's only failed steps are its two Metal timing
probes. The [new classifier](evidence/hosted-metal-preview-och17.md) distinguishes
complete all-zero clocks from failures and cross-checks the independent probes.
Portable tests and Swift compilation pass; local mixed/invalid-clock cases are
correctly rejected and retained. Run `37825342234` is terminal: Linux and fresh
macOS extracted apps pass; macOS Foundation fails only the dates/colors final
composer-draft assertion. Both hosted timing probes agree on unavailable timing;
their gate passes without claiming presentation qualification. The
[draft-observation follow-up](evidence/chat-draft-observation-och17.md) adds exact
asynchronous-value checks and passes locally; hosted validation remains pending.
See the [CI record](evidence/milestone-07-ci.md). Final integration remains open.

The [preview adoption guide](developer-preview.md) and GitHub feedback templates
are ready. A fresh independent starter consumer build passes at `87b43c73`;
the [installation/API handoff](evidence/preview-installation-och17.md) now closes R1.
The [source payload review](evidence/preview-source-inputs-och17.md) repairs a
retained example-font notice and verifies the assembled source archive, closing
D1 for the source preview. Binary-distribution notice completion remains OCH-164.

The [managed-scrollbar routing repair](evidence/managed-scrollbar-routing-och41.md)
adds a reactive component argument that reaches native viewports inside Bonsai
layout wrappers. All five adapter regressions and the full OCaml suite pass;
the repository and freshly installed galleries each pass 18 scoped
theme/size/owner input cases.
Frame timing, VoiceOver and broader release acceptance remain separate.

The [force-close cleanup repair](evidence/force-close-cleanup-och17.md) ensures
raising cleanup cannot skip queued native close/shutdown work or leave the
opening-window path unwoken. Regression tests cover first-failure propagation,
reentrancy, delayed opening acknowledgments and cleanup; hosted validation remains.

The [OCaml notice provenance audit](evidence/ocaml-notice-provenance-och17.md)
verifies all 499 collected texts against original checksum-verified archives.
The ten zero-text metadata classifications remain visible; embedded attribution
and whole-release notice review are still open.

The [loaded-list paint diagnostic](evidence/list-paint-geometry-och17.md) found no
row overlap in 24,338 observed list paints, but the full attempt timed out with
an inactive window before completion. This narrows the geometry investigation;
it is neither a visual-jitter fix nor performance acceptance.

Hosted run37789987337 is terminal: Linux and the separate macOS extracted-app
receiver pass; the previously repaired scrollbar-preference and document-profile
checks now pass too. macOS Foundation fails two carousel remount tests (now
repaired locally) and both unresolved Metal presentation probes. All three
receiver apps finish without reproducing the earlier Python trap. See
[CI evidence](evidence/milestone-07-ci.md); later local changes still need hosted
validation, and signed-release acceptance remains open.

The [scope cancellation repair](evidence/scope-cancellation-och17.md) prevents a
raising cleanup from stranding sibling scopes, producers or resource accounting.
Deterministic regressions cover first-failure propagation, reentrancy, queued
stream suppression and ancestor unregister ordering; full OCaml tests pass.

The [scrollbar animated-width repair](evidence/scrollbar-input-macos-och41.md)
keeps native ranges/focus alive when appearance changes narrow an animated track.
The failing regression now passes alongside the full native suite. The ordinary
viewport walkthrough checks pointer/key input and retained offsets separately
from frame timing, VoiceOver and managed-owner release acceptance.

The [carousel remount test repair](evidence/carousel-remount-harness-och41.md)
reproduces two hosted page-return failures and adds the missing outer-page reveal
before focus/editor queries. Both local and installed lifecycle/automatic checks
pass without application changes or relaxed assertions; hosted revalidation remains.

The [message-follow desktop walkthrough](evidence/message-follow-macos-och41.md)
repairs a clipped Large-size follow button in the OCaml gallery. Repository and
installed tests pass both themes/three sizes, native Space/pointer/wheel routing
and settled reading anchors during message updates. Per-frame scrolling and
VoiceOver qualification remain separate.

The [searchable-list desktop matrix](evidence/selectable-list-macos-och41.md)
passes both themes and three application sizes locally and in the installed
consumer: native query typing, confirmation/context/cancel intents, hidden
selection, fetched membership, retry, axis changes and source reset. It adds
scoped evidence without a production change; broader pointer, VoiceOver and
performance/resource acceptance remain separate.

The [horizontal-card desktop follow-up](evidence/horizontal-list-macos-och41.md)
fixes a clipped reaction button in the OCaml example through palette-scaled card
and viewport sizing. Repository and installed tests pass both axes, themes and
three application sizes, including pointer/Space, wheel geometry, surviving row
state, edit anchors and follow-tail. This is separate from the reported benchmark
jitter and does not qualify physical frame timing or VoiceOver.

The [application teardown repair](evidence/application-teardown-och17.md) ensures
pending desktop/notification/resource requests finish with Closed even if another
completion raises. Deterministic regressions cover all six request families,
reentrant disposal, rejected new work and ignored late responses. This extends
the existing window-cleanup policy; OS-service and release acceptance remain separate.

The [overlay desktop matrix](evidence/overlay-macos-och41.md) passes both themes
and three application sizes locally and in the installed consumer. It verifies
modal focus, live all-edge drawer insets, hover-card actions, fixed-point popup
corners and page retirement. VoiceOver, motion timing and full resource/release
acceptance remain separate.

The pagination gallery follow-up exercises both themes and three application
sizes, including billion-page chooser bounds, native input and model-change
cancellation. It also repairs chooser buttons that used pale text on a pale
accent in the dark theme. [Evidence](evidence/pagination-macos-och41.md) records
repository/installed behavior, pixel checks and the distinct model/native lifetimes.


The [disclosure desktop walkthrough](evidence/disclosure-macos-och41.md) passes
six theme/size combinations in the repository and freshly installed galleries.
It covers native draft/undo retention, hidden accessibility nodes, expansion
policies, heading keys, disabled skipping/group state and fresh buffers after
Unmount. Motion timing, VoiceOver and resource/release acceptance remain separate.

The [inherited list-metrics repair](evidence/list-inherited-metrics-och17.md)
fixes a reproduced 44-pixel movement from a 24-pixel wheel event after an
ancestor line-height change. The production host now invalidates cached
offscreen measurements when inherited metrics change. The deterministic native
suite passes; this does not identify the cause of the separate benchmark overlap.
The linked real-window follow-up also passes all three line-height cases using
fresh painted-row bounds; its direct GPUI input is separate from OS trackpad
delivery and presentation timing.

The [notarization tooling follow-up](evidence/developer-id-tooling-och17.md#notarization-finalization-tooling--2026-10-08)
adds a submission/recovery guide and a finalizer that binds acceptance to the
original archive, staples a copy and verifies the final ZIP after extraction.
Portable tests pass with simulated Apple tools. Actual service, signing and
quarantined receiver acceptance remain pending; no artifact was submitted.

The [navigation-history gallery follow-up](evidence/navigation-history-macos-och41.md)
adds a modular public example for new visits, replacement, root/reset and explicit
motion/retention policies. Six theme/motion combinations pass native keyboard/AX
history and draft-lifetime checks locally and in a fresh installed consumer.
Frame timing, VoiceOver and consolidated release acceptance remain separate.

The [table-worker callback repair](evidence/api-boundaries-och17.md#obsolete-table-worker-notifications--2026-10-08)
suppresses redundant application notifications from obsolete query completions
while returning worker capacity to current requests. Deterministic full-inbox
before/after evidence is recorded separately from native or performance acceptance.

The [list-paging ownership repair](evidence/api-boundaries-och17.md#paging-ownership-repair--2026-10-08)
keeps cancellation cleanup inside the two-producer bound and publishes final
reactive state on closure. Deterministic before/after regressions and the native
managed-conversation self-test pass. This does not resolve the separate visual
scrolling report or qualify full-list performance.

The [ARM Metal pacing follow-up](evidence/hosted-metal-diagnostic-och17.md#arm-sequential-and-startup-delay-follow-up--2026-10-08)
retains two failed probes: sequential submission, with and without a fixed startup
delay, stalls waiting for presentation callbacks. GPU completion is not presentation
acceptance. Main probe behavior and release gates remain unchanged.

The [scrollbar oracle repair](evidence/scrollbar-preference-och41.md#hosted-oracle-initialization-repair--2026-10-08)
fixes a CI helper reading AppKit before automatic device policy initialized.
Hosted phase probes reproduce the transition and validate the corrected helper;
all three local gallery cases pass. Production snapshot behavior is unchanged,
and the full corrected hosted gallery check remains pending.

The [macOS focus-forwarding repair](evidence/window-focus-forwarding-och17.md)
fixes application accessibility queries returning the window instead of its focused
control. Twelve native carousel axis/theme/scale cases pass locally and in a fresh
installed consumer, alongside two-window input/undo/remount checks. VoiceOver and
full release acceptance remain separate.

The [resource API review](evidence/api-boundaries-och17.md#resource-publication-and-recovery-review--2026-10-08)
clarifies desired state versus native publication, resource limits and recovery
after document/chart/canvas upload failures. Deterministic runtime tests cover
fatal retirement and rejected retries; production behavior is unchanged. This
is scoped contract evidence, not full API or release acceptance.

The [split-group desktop walkthrough](evidence/split-group-macos-och41.md) passes
pointer cancellation, keyboard/AX resizing, delivery into Bonsai, constrained
serialled requests and retained editing through structural changes, locally and
in the installed consumer. Full accessibility, paint/resource and release
acceptance remain separate.

The [native tabs walkthrough](evidence/tabs-macos-och41.md) passes retained panels,
independent close/menu controls, explicit reveal and thirty geometry/keyboard
cases across five variants, both themes and three scales, locally and in the
installed consumer. Page remount also passes; motion/VoiceOver/resource and full
workspace acceptance remain separate.

The [macOS scroll phase repair](evidence/macos-scroll-phases-och41.md) preserves
AppKit cancelled-scroll events instead of treating them as continued movement.
An actual AppKit factory/converter regression fails before the fix and passes
afterward. This does not establish hardware gesture delivery or resolve the
separately reported loaded-list overlap.

The [measured carousel desktop follow-up](evidence/carousel-track-macos-och41.md)
passes native keys/editing, both axes/themes, clipped-neighbor pointer selection,
reorder/resize retention and remount locally and in the installed gallery.
Its automatic-policy follow-up also passes five pause conditions, fresh resumes
and page-retired timers. Twenty OS pointer cases and native child text selection
now pass locally and in the installed consumer, including observed drag previews
and Escape cancellation. Discrete line-wheel navigation and vertical endpoint
handoff also pass in both axes/themes. Twelve interrupted-drag cases pass for
window deactivation, viewport resize and page retirement in each binary. Precise
hardware trackpads, full focus/accessibility and resources remain open.

The [sidebar desktop walkthrough](evidence/sidebar-macos-och41.md) passes
branch policies, actual keyboard/pointer selection, twelve appearance cases,
compact/offcanvas preferences and page remount in local and installed galleries.
It adds scoped evidence without a production change; broader release gates remain.

The [foreground input diagnostic](evidence/foreground-input-diagnostic-och17.md)
now distinguishes quiet intervals from event-kind counts in opt-in runs. Quiet and
four-mouse-movement checks validate collection. A subsequent full 10,000-row
diagnostic completes sixty-second idle with zero draws, inputs and lost entries;
its history journal is truncated and reported separately. These diagnostic runs
cannot qualify performance or identify the earlier event producer. The reported
visual overlap and full overhead comparison remain open.

The [notification gallery follow-up](evidence/notification-gallery-och41.md)
fixes restoration during an animated exit using fresh batch identities and stale
dismissal guards in the OCaml example. Actual macOS placement, collapsed semantics,
keyboard dismissal/restoration and page retirement pass alongside the gallery
expect tests. Full toast motion/accessibility/resource and release acceptance remain.
Its policy follow-up also passes real pointer expansion/collapse and hover/focus
timeout pausing under Full/Reduced application preferences, locally and in the
installed consumer. Presentation timing and VoiceOver remain separate.

The [Developer ID tooling checkpoint](evidence/developer-id-tooling-och17.md)
adds explicit certificate/team selection, hardened-runtime signing and strict
verification before archiving or runtime extraction. Local portable checks and
native rejection of an ad-hoc fixture pass; real Developer ID signing,
notarization and release acceptance remain unqualified. The full isolated local
expect-test and formatting aliases pass at `e58cec44`.

The [form label and screen-point follow-up](evidence/form-label-point-routing-och17.md)
fixes rich labels collapsing to zero width. The real 52-case form walkthrough
and 21 exact system-wide AX control/overlay hits pass locally, with API tests
and formatting. Hosted validation, full catalog and VoiceOver acceptance remain.

The [nested native-popup follow-up](evidence/native-menu-icons-och41.md#nested-popup-artwork-follow-up--2026-10-07)
qualifies root/submenu/nested disabled SVG artwork through the public Feedback
example, both locally and in the staged-library consumer. Actual keyboard
navigation, passive/disabled state and Save-to-Bonsai delivery pass. This adds
scoped pixel evidence; full menu-family and release acceptance remain open.

The [gallery header/consumer follow-up](evidence/gallery-header-layout-och41.md)
fixes clipped header controls through ordinary OCaml wrapping styles. Both local
and independently installed galleries pass 59 shell-layout cases and actual New
window activation. The installed native menu-bar walkthrough also passes. These
are scoped shell/public-API results, not full-gallery or release acceptance.

The [full-suite menu fixture repair](evidence/native-menu-bar-icons-och41.md#full-suite-fixture-generations--2026-10-08)
corrects retired-window generation reuse across the combined controls fixture.
Both complete local controls and standalone menus pass, with strict lint and
formatting. Production ownership checks are unchanged; hosted revalidation remains.

The [native menu-bar artwork checkpoint](evidence/native-menu-bar-icons-och41.md)
extends passive SVG item paths to platform bars. Actual AppKit and public gallery
checks pass nested/disabled artwork, clear/restore, command delivery and window
ownership. It also fixes AppKit validation re-enabling explicitly disabled rows.
The native suite passes 1,187 tests (two existing skips), and strict Clippy and
vendor reconstruction pass. The linked follow-up adds installed-consumer evidence; final-source hosted/Linux
and consolidated release acceptance remain open.

The [document worker-pressure repair](evidence/document-worker-pressure-och17.md)
keeps short profile updates pending while an active worker holds temporary
preparation capacity. The unchanged memory cap still rejects persistent
exhaustion. The deterministic before-case fails; 22 job tests, 1,187 native tests
(two existing skips) and both normal/Large actual profile walkthroughs pass.
The prior-source hosted run passed Linux and the fresh macOS package receiver,
but failed the document profile and both Metal presentation probes. Final-source
hosted revalidation and presentation qualification remain open.

The [native-menu retirement repair](evidence/native-menu-retirement-och41.md)
fixes retained Rust commands after actual menu-bar/Dock replacement. Real AppKit
checks pass 128 replacements, stale native-item rejection, current dispatch and
independent Dock teardown. Three portable ownership regressions and the
1,184-test native suite pass (two existing skips). That ownership checkpoint predates the separate artwork qualification above.

The [rich-header sizing follow-up](evidence/table-header-fit-och41.md) fixes the
public gallery's clipped Inspect label using ordinary OCaml button styles.
Six theme/scale combinations pass actual glyph/rectangle fit, pointer/Space
activation, fixed table geometry and page remount checks.

The [hover-layout repair](evidence/hover-layout-och41.md) fixes stale hover paint
when initial paint or a layout change puts a widget under a stationary pointer.
Two original native cases fail. Five focused regressions and the full
1,184-test native suite pass (two existing skips), including preserved
hover-driven layout. The repaired real table walkthrough passes scoped
paint/reset, fixed geometry and input across two themes and three scales. This is separate from the loaded-list scrolling report and does not
establish full catalog, VoiceOver or performance acceptance.

The [loaded-list scroll investigation](evidence/list-scroll-diagnostics-och17.md)
distinguishes automated row jumps from ordinary wheel input. Eighty wheel steps
showed no sampled row overlap; 524 retained-row comparisons matched their pixel
deltas. This does not rule out a brief paint glitch. Full runs still failed, and
controlled minimization reproduced a frame wait timeout. Failure-only diagnostics
now retain window/viewport/runtime state; no list performance acceptance or
rendering fix is claimed.

The [rendered visual-line repair](evidence/rendered-visual-lines-och17.md) connects
styled and Unicode fragments on the same painted line without joining paragraphs
or table cells. Native reflow/scope tests and actual macOS paragraph line/range and
character-point queries pass; the native suite reports 1,179 passed, 2 ignored.
Broader geometry and VoiceOver reading/navigation remain open. VoiceOver scripting
setup is awaiting the owner's macOS authorization; other milestone work continues.

The [VoiceOver attribute-query repair](evidence/voiceover-attribute-query-och17.md)
fixes a reproduced process crash in GPUIO's AccessKit compatibility patch. A
read-only attribute query called a nonexistent superclass method. Actual OS
mutability/selection/Copy checks now pass for rendered documents, code and editable
text. VoiceOver speech capture remains unqualified. Native tree discovery/action
checks and 200,000 history visits passed after isolating the no-input retention
phase from foreground focus; the evidence records earlier failures and one
required reactivation during the foreground focus phase.

The [cached prepaint retry repair](evidence/cached-prepaint-retry-och17.md) fixes a
reproduced panic after an aborted prepaint overwrote saved frame indices. Candidate
ranges now commit only in paint; fresh unpainted state cannot be reused as a scene.
Tests cover initial/retained caches, direct/deferred controls, repeated retries,
unchanged render counters, stable semantic IDs and live actions. A real cached
TextView also rejects stale selection requests after source replacement and
publishes new text through normal invalidation.

This follows [native text-selection cache replay](evidence/cached-text-selection-och17.md),
[callback-owner cleanup](evidence/cached-accessibility-owners-och17.md),
[semantic cache replay](evidence/cached-accessibility-replay-och17.md),
[flow selection reveal](evidence/rendered-selection-flow-och17.md), and
[real rendered-document selection dispatch](evidence/rendered-selection-dispatch-och17.md).
These are completed scoped repairs, not newly unimplemented items to rediscover.

At **2845f1c9**, local macOS arm64 validation passed:

- **1,175 native tests, 2 ignored**, strict workspace/all-targets Clippy,
  workspace and vendor formatting, Rust workspace tests and gallery rebuild.
- Actual macOS editor/document tests, plus public-gallery heading/CJK/joined-emoji/
  code range setters, exact Copy and caret checks. Owned windows closed normally;
  clipboard restored and verified. VoiceOver was not operated in these checks.
- Exact reconstruction of all 159 GPUI files (generated Cargo.lock excluded).
  Base is unchanged from 968e423b, whose 243-file reconstruction passed.

Exact commands, revisions, hashes, before/after failures and bundles are linked
from the evidence above. The full incremental `dune runtest -j2` alias now passes
on **88d3c3d8** with notice-only working changes; the
[theme notice follow-up](evidence/syntect-themes-och17.md#original-author-notices)
retains its log. This does not claim a clean rebuild or foreground native testing.
Repeat required final checks on the release source. No direct upstream GPUI/Base
unit-suite pass is claimed. Native fixtures
exercise their production code; OS tests are separately scoped. Current-source
hosted/Linux results still need acceptance; canceled runs are not passes.

<a id="open-milestone-requirements"></a>

## Remaining milestone acceptance

The [preview checklist](milestone-07-closeout.md) and amended live tickets define
current completion. The table below retains the broader qualification inventory;
OCH-164 owns the explicitly deferred portions. Historical local
passes apply to their recorded sources and coverage, not automatically to the
current release candidate.

| Requirement | Evidence and remaining work |
| --- | --- |
| OCH-41: complete component mapping | The [catalog](catalog/README.md) and [family ledger](catalog/families.json) map 41 v1 families, post-v1 docking and excluded development tooling. Finish nested-functionality review and consolidated meaningful behavior evidence for every required row; root-module presence alone is insufficient. Helpers belong to their owning APIs. Keep post-v1 grid editing/docking/editor/LSP boundaries explicit. |
| OCH-41: public gallery and document/input contracts | Public OCaml examples and scoped installed-consumer/physical results exist. Complete the expanded gallery walkthrough, visual review, themes/scales, keyboard/accessibility and teardown acceptance. Preserve completed [form/editor admission](evidence/gallery-editor-admission-och41.md), [theme-file loading/cancellation](evidence/gallery-theme-files-och41.md), [two-window appearance](evidence/window-appearance-och41.md), [window lifecycle](evidence/window-lifecycle-och41.md), [input queries](evidence/window-input-query-och41.md), [window selection](evidence/window-selection-och41.md), [document profiles](evidence/document-profile-macos-och41.md) and [plugin scrolling](evidence/document-profile-scroll-och41.md). Do not redo these solely because an old checkpoint calls them untested. |
| OCH-17: actual macOS native/accessibility acceptance | [Multiline and single-line Japanese IME](evidence/multiline-ime-cancellation-och17.md), source-editor selection/geometry and rendered-document selection have scoped evidence. Complete consolidated focus/clipboard/file-drop/OS accessibility coverage, real VoiceOver reading/tracking/navigation, rendered visual-line/character-geometry qualification. Representative point-based routing now passes for the two-window editors and [21 control/overlay points](evidence/form-label-point-routing-och17.md). Use [document accessibility](design/document-accessibility.md), [editor geometry](design/editor-accessibility-geometry.md), [window accessibility](evidence/window-accessibility-och17.md) and [gallery evidence](evidence/gallery-och41.md). Tree presence or TestPlatform success is not screen-reader acceptance. |
| OCH-17: performance and resources | Owner direction on 2026-10-08 narrows mandatory timing to **10k variable-height lists** and **rapid streaming while typing**, with existing budgets/three valid trials and shared idle/resource/measurement-validity checks. See the [focused plan](design/performance-qualification.md#focused-v1-scope--owner-direction-2026-10-08) and P1/P2 in [closeout](milestone-07-closeout.md). Preserve the recorded three-trial table/document/typing and lifecycle evidence; review change impact rather than rerun all matrices. The loaded-list batch has two full passes and an unresolved 14-draw idle failure; interrupted later diagnostics and collector comparisons do not establish acceptance. No resource or current user-facing failure is waived. |
| OCH-17: chart/startup findings | The historical 63,050.99ms chart publication delay, startup-bound failure and nonblack startup samples remain documented in [chart](evidence/chart-streaming-och40.md) and [startup evidence](evidence/window-startup-och17.md). Under the owner’s focused performance scope, unresolved original causes are follow-up diagnostics rather than independent release gates. Reproducible current user-facing stalls/blank windows remain defects to fix; old failures are not relabeled as passes. |
| OCH-17: notices and reproducible distribution | [Bonsai reconstruction](evidence/bonsai-reconstruction-och17.md) covers all eight vendored roots. Finish [OCaml](evidence/ocaml-notices-och17.md)/[Rust](evidence/dependency-notices-och17.md) notice, asset and system review, current release-artifact qualification, platform minimums and the chosen signing/distribution workflow. Earlier internal ad-hoc archives pass on a separate hosted macOS receiver and locally with development paths denied; [package evidence](evidence/package-runtime-och17.md). That is not signed-release acceptance. Follow [distribution](distribution.md) and [fork maintenance](component-adapters.md). |
| OCH-17: API, examples and release | The [application guide](getting-started.md), [compatibility/limits](api-compatibility.md), [installed starter](evidence/public-api-starter-och17.md) and [API boundary review](evidence/api-boundaries-och17.md) exist. Finish whole-surface API/limitations review, versioning, reviewed changes, required final checks and release publication. Maintain adjacent implementation walkthroughs and [the example inventory](../examples/coverage-guide.md); scratch is not a release dependency. |
| Required Linux/hosted checks | [Run 37286788836](https://github.com/dakotamurphyucf/gpuio/actions/runs/37286788836) passed both foundation jobs and the separate macOS extracted-app receiver at the recorded older tree. The newer [run 37744396036](https://github.com/dakotamurphyucf/gpuio/actions/runs/37744396036) at `497236b7` passes Linux and the separate macOS receiver. macOS Foundation passes the repaired control fixture and full table-history traversal; only both Metal probes fail, with zero presentation timestamps in all 90 frames per GPUI window and all 120 calibration frames. The subsequent [run 37758529147](https://github.com/dakotamurphyucf/gpuio/actions/runs/37758529147) at `61944bfc` passes Linux but exceeds the macOS two-hour job limit during artifact upload. Scrollbar-preference and both Metal checks also fail; the receiver cannot obtain the incomplete upload. The macOS job allowance is now three hours, with individual test limits and acceptance thresholds unchanged. Current changes require their own accepted run. The [hosted Metal investigation](evidence/hosted-metal-diagnostic-och17.md) distinguishes ARM zero timestamps from a passing sequential Intel standalone API probe; the unchanged Intel GPUI probe fails on an initial two-second zero-timestamp interval. An idle-delay experiment still fails with one initial zero per window; a separately recorded active-warmup experiment also fails with one initial zero in its active window. A subsequent isolated Intel continuous-warm-up batch passes all three predeclared trials (90 positive presentations per window, zero measured losses/zeros); it retains warm-up zeros and does not qualify cold startup or replace the original Foundation gates. No gate is waived. Linux build/unit/private-bus/consumer remain mandatory; graphical failures stay informational in OCH-47. See [CI evidence](evidence/milestone-07-ci.md). |
| Linear completion | Close OCH-41/OCH-17 only after their full acceptance criteria pass. No local summary, screenshot, test count or partial family review substitutes for that audit. |

<a id="environment-constraints-and-next-action"></a>

## Next action and history

Follow the [closeout checklist](milestone-07-closeout.md): C1 is complete; prioritize the two core performance
workloads and their measurement validity. Continue notices/API review while
waiting for CI. Broader accessibility qualification is OCH-164. Fix named required defects; reuse valid evidence
and retain failed trials. Batch coherent changes; required merge gates remain.

The complete pre-consolidation status snapshot and older checkpoints are preserved
in [status history](status-history.md). Use that history and individual evidence
for provenance, not as a list of still-open tasks or current permissions. Durable
designs live under `docs/design/`; current implementation claims need their own
source and platform evidence.
