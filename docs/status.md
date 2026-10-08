# Implementation status

Current handoff: 2026-10-07. Milestone
**07 — Expanded v1 macOS validation and release** is active. **OCH-41 and OCH-17
remain In Progress.** OCH-48 example walkthrough acceptance is complete.
This page records current work; it does not certify release readiness.

<a id="scope-and-platform-policy"></a>

## Scope and working rules

[Platform policy](platform-release-policy.md): macOS behavior, accessibility,
performance/resources and distribution are release gates. Linux build, unit,
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
Older records of a VoiceOver hold no longer apply. Access was restored after the
2026-10-07 session restart; filesystem, Linear and GitHub access are working.

<a id="current-implementation-and-evidence"></a>

## Latest delivered behavior and validation

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

The live Linear tickets and linked contracts define completion. Historical local
passes apply to their recorded sources and coverage, not automatically to the
current release candidate.

| Requirement | Evidence and remaining work |
| --- | --- |
| OCH-41: complete component mapping | The [catalog](catalog/README.md) and [family ledger](catalog/families.json) map 41 v1 families, post-v1 docking and excluded development tooling. Finish nested-functionality review and consolidated meaningful behavior evidence for every required row; root-module presence alone is insufficient. Helpers belong to their owning APIs. Keep post-v1 grid editing/docking/editor/LSP boundaries explicit. |
| OCH-41: public gallery and document/input contracts | Public OCaml examples and scoped installed-consumer/physical results exist. Complete the expanded gallery walkthrough, visual review, themes/scales, keyboard/accessibility and teardown acceptance. Preserve completed [form/editor admission](evidence/gallery-editor-admission-och41.md), [theme-file loading/cancellation](evidence/gallery-theme-files-och41.md), [two-window appearance](evidence/window-appearance-och41.md), [window lifecycle](evidence/window-lifecycle-och41.md), [input queries](evidence/window-input-query-och41.md), [window selection](evidence/window-selection-och41.md), [document profiles](evidence/document-profile-macos-och41.md) and [plugin scrolling](evidence/document-profile-scroll-och41.md). Do not redo these solely because an old checkpoint calls them untested. |
| OCH-17: actual macOS native/accessibility acceptance | [Multiline and single-line Japanese IME](evidence/multiline-ime-cancellation-och17.md), source-editor selection/geometry and rendered-document selection have scoped evidence. Complete consolidated focus/clipboard/file-drop/OS accessibility coverage, real VoiceOver reading/tracking/navigation, rendered visual-line/character-geometry qualification and point-based AX coverage beyond the passing two-window editor regression. Use [document accessibility](design/document-accessibility.md), [editor geometry](design/editor-accessibility-geometry.md), [window accessibility](evidence/window-accessibility-och17.md) and [gallery evidence](evidence/gallery-och41.md). Tree presence or TestPlatform success is not screen-reader acceptance. |
| OCH-17: performance and resources | Rebuild and qualify corrected optimized sources under the fixed [performance plan](design/performance-qualification.md); reports from the withdrawn allocation optimization do not establish acceptance after [lifetime repairs](evidence/gallery-lifetime-repairs-och41.md). Required workloads include 10k loaded variable-height rows, 100k paged rows, growing documents, concurrent streaming/typing, repeated windows/history traversal, focused/unfocused idle, native-entity/physical/Metal resource retirement and collector overhead. Separate CPU work, queueing, input-to-paint and actual presentation. Historical [list](evidence/presentation-list-full-och17.md), [table](evidence/paged-table-performance-och17.md), [document](evidence/growing-document-och17.md), [streaming/typing](evidence/presentation-typing-full-och17.md), [idle](evidence/idle-performance-och17.md), [physical memory](evidence/physical-memory-och17.md), [native entities](evidence/native-entity-retention-och17.md) and [Metal](evidence/metal-resource-och17.md) reports retain their exact scope. List startup still has a recorded failure against the unchanged 100ms bound; do not rerun until lucky or raise the limit. Table/document presentation repetitions and presentation-collector overhead/resources remain. |
| OCH-17: chart/startup findings | Investigate/qualify the historical 63,050.99ms chart publication delay. Later staged runs and a controlled minimize demonstrate visibility-sensitive readiness but do not establish the original cause; [chart evidence](evidence/chart-streaming-och40.md). Twelve nonblack startup samples do not rule out earlier blank frames; no production startup fix is claimed by [that evidence](evidence/window-startup-och17.md). |
| OCH-17: notices and reproducible distribution | [Bonsai reconstruction](evidence/bonsai-reconstruction-och17.md) covers all eight vendored roots. Finish [OCaml](evidence/ocaml-notices-och17.md)/[Rust](evidence/dependency-notices-och17.md) notice, asset and system review, current release-artifact qualification, platform minimums and the chosen signing/distribution workflow. Earlier internal ad-hoc archives pass on a separate hosted macOS receiver and locally with development paths denied; [package evidence](evidence/package-runtime-och17.md). That is not signed-release acceptance. Follow [distribution](distribution.md) and [fork maintenance](component-adapters.md). |
| OCH-17: API, examples and release | The [application guide](getting-started.md), [compatibility/limits](api-compatibility.md), [installed starter](evidence/public-api-starter-och17.md) and [API boundary review](evidence/api-boundaries-och17.md) exist. Finish whole-surface API/limitations review, versioning, reviewed changes, required final checks and release publication. Maintain adjacent implementation walkthroughs and [the example inventory](../examples/coverage-guide.md); scratch is not a release dependency. |
| Required Linux/hosted checks | [Run 37286788836](https://github.com/dakotamurphyucf/gpuio/actions/runs/37286788836) passed both foundation jobs and the separate macOS extracted-app receiver at the recorded older tree. Later changes require their own accepted run. Linux build/unit/private-bus/consumer remain mandatory; graphical failures stay informational in OCH-47. See [CI evidence](evidence/milestone-07-ci.md). |
| Linear completion | Close OCH-41/OCH-17 only after their full acceptance criteria pass. No local summary, screenshot, test count or partial family review substitutes for that audit. |

<a id="environment-constraints-and-next-action"></a>

## Next action and history

Proceed with macOS accessibility qualification, then the remaining consolidated
catalog/performance/distribution gates above. Fix reproduced defects and retain
failed evidence; do not keep expanding speculative engine tests instead of the
release requirements. Batch coherent changes and validate locally before waiting
on CI; required merge gates remain in force.

The complete pre-consolidation status snapshot and older checkpoints are preserved
in [status history](status-history.md). Use that history and individual evidence
for provenance, not as a list of still-open tasks or current permissions. Durable
designs live under `docs/design/`; current implementation claims need their own
source and platform evidence.
