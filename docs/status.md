# Implementation status

Current handoff: 2026-10-06. Milestone **07 — Expanded v1 macOS validation and
release** is in progress. **OCH-41 and OCH-17 remain open.** Owner-requested **OCH-48** adds adjacent
Markdown walkthroughs for every example component; its documentation acceptance is complete. This page separates
current work from historical checkpoints; it does not certify release readiness.

[Example walkthrough coverage](evidence/example-walkthroughs-och48.md) now has an
explicit inventory of 417 OCaml/Rust source files in 260 groups, with all 260 groups
reviewed and none pending. The starter, palette/scope/gallery/theme guides,
chart samples and application, controls/editors/menus, and Agent Chat startup/model/
message/motion/conversation guides, gallery data/media pages and positioned-menu
controllers explain actual application, Bonsai, GPUIO and Eio code.
Three owner-authorized GPT-6.1 Sol agents contributed scoped walkthroughs in parallel.
Contributor guidance and a structural CI check keep new sources visible; neither
file presence nor that check establishes documentation or platform acceptance.

[Radar label clipping](evidence/radar-label-clipping-och41.md) now retires native
focus and stale AppKit actions when resize clips a child inside a partly visible
composite, without a tree/source update. Zero/oversized bounds and recovery pass.
Broader testing caught and repaired cleanup ordering against carousel focus;
the final native suite, strict lint and root gallery walkthrough pass. Idle
rendering settles. Broader label/catalog/release qualification remains open.

[Hosted run 37487278162](evidence/hosted-presentation-calibration-och17.md#hosted-run-37487278162)
is terminal: Linux foundation and all three independently extracted macOS apps
pass. Only the two macOS Metal presentation probes fail, again receiving zero-time
callbacks on Apple Paravirtual. This covers tree-equivalent `c373e3b`, before the
subsequent radar input/clipping repairs; current-source CI remains required.
Neither physical-presentation gate is waived.

[Radar label actions](evidence/radar-label-actions-och41.md) now reject ordinary
button and command gestures that cross a hide/return transition, before and after
repaint. Revocable native lifetimes also reject queued AppKit actions and retired
AX objects while fresh targets recover. Native/table suites and root/fresh-installed
public gallery checks pass. Specialized actions and broader label/catalog/release
qualification remain open.

[Radar label InputRegion](evidence/radar-label-input-region-och41.md) now passes
foreground native event dispatch and stale-click rejection across axis/browser
hide and return before repaint. The regression exposed consumed mouse-down and
pending clicks surviving hiding; both are repaired. Existing chart, InputRegion
and popup harnesses pass. Broader label/catalog/release qualification remains open.

[Radar label sliders](evidence/radar-label-slider-och41.md) now pass native track
input and immediate source-hide cancellation. The regression exposed inner
mouse-down suppression and stale slider capture; both are repaired, with
same-axis publication preservation and stale-release recovery verified.
Native chart/input/popup and ordinary slider suites pass. Other label wrappers
and broader catalog/release qualification remain open.

[Radar label popup retirement](evidence/radar-label-popup-och41.md) now has real
AppKit evidence for queued hide/return and cancellation during native tracking.
The regression exposed missing menu-lease cleanup and a focus-only data-browser
path. Both are repaired; browser entry now retires held label gestures too.
Native unit/chart/input/popup checks pass. Remaining label/catalog/release
qualification is explicit in the evidence record.

[Radar label capture](evidence/radar-label-capture-och41.md) now has a native
regression for ordinary View gestures. It exposed and repaired inner mouse-down
suppression and capture surviving source-driven hiding until another frame.
Native dispatch checks pass removal/reset/family/hide/source replacement/release,
stale mouse-up rejection and fresh gesture recovery. Broader qualification remains
open; hidden-window dispatch is not physical-mouse or VoiceOver evidence.

[Radar label identity](evidence/radar-label-identity-och41.md) now follows stable
axis IDs across caption changes/reordering. Broader lint also exposed and repaired
chart configuration inflating every protocol operation: its Rust payload is now
boxed, preserving wire bytes and reducing local operation size from 584 to 352
bytes. Protocol/chart/tree tests and dependency-inclusive strict lint pass.
The [arbitrary View label adapter](design/radar-label-content.md) is now implemented:
ordinary OCaml buttons, composed text and native editors can replace axis captions.
[Scoped local evidence](evidence/radar-label-content-och41.md) covers paired codecs,
retained-tree validation, real GPU layout and foreground input/draft retention.
Additional lifecycle/interaction qualification and broader catalog work remain.

[Radar projection options](evidence/radar-projection-och41.md) now expose shared
maxima, fixed radius and label spacing from OCaml, preserving per-axis defaults
and original source values. Paired codecs, worker/mesh tests, full local checks,
and root/fresh-installed macOS gallery walkthroughs pass. Options schema is 7;
style/data remain -2/1. Rich radar labels and broader chart/catalog work remain.

[Hosted run 37470525492](https://github.com/dakotamurphyucf/gpuio/actions/runs/37470525492)
is terminal: Linux foundation and all three independently extracted macOS apps
pass. The macOS foundation job fails only the GPUI Metal presentation hook and
standalone Metal calibration. Signal Studio and Agent Workspace Diagram now pass
on the runner. This checks older `00f43c8`, not subsequent radar work; no
presentation gate is waived.

[Hosted run 37460415879](evidence/hosted-presentation-calibration-och17.md#hosted-run-37460415879-and-local-input-synchronization-repairs)
is terminal: Linux and all three independently extracted macOS apps pass. The
macOS foundation job failed Signal Studio input, Agent Workspace Diagram and both
Metal presentation probes (again 180/120 zero-time callbacks on Apple Paravirtual).
The two demo drivers now wait for the relevant native focus/closure transition;
both complete local foreground walkthroughs pass. Hosted confirmation is pending.
Hosted evidence covers tree-equivalent `75ce53d`, not newer work. No gate is waived.

[Measured Sankey labels](evidence/sankey-label-gallery.md) now pass actual hidden-window
text pixels and root/fresh-installed public gallery checks with three columns,
inside/outside, long/rich/hidden captions, narrow/font/theme transitions, unchanged
raw selection/update and cleanup. Screenshot review exposed and fixed wrapping
inside one-line caption rectangles; its failing/passing pixel regression is retained.
Full local OCaml checks and strict native lint pass. Options schema remains 6;
current-source hosted checks and broader catalog/release acceptance remain open.

[Code document controls](evidence/document-code-controls-och41.md) now append/reset
the displayed code source independently of Markdown. Focused build/tests/format and
root/fresh-installed macOS document walkthroughs pass. A reproduced test-driver
scroll target used stale layout bounds; the current-bounds correction passes both
walkthroughs without changing native scrolling or visibility assertions.

[Sankey ribbon color policies](evidence/sankey-link-colors-och41.md) now expose
Source, Target and Gradient through the public OCaml API. Endpoint colors resolve
on the worker and one retained mesh carries the whole-ribbon gradient. Paired
codecs, native unit tests, GPU readback at four scales, full OCaml checks and
root/fresh-installed macOS chart walkthroughs pass. Options schema is 5; style/data
remain -2/1. Broader catalog and release qualification remain open.

[Rich Sankey labels](evidence/chart-node-labels-och41.md) now provide bounded
ID-keyed multiline captions, per-line font/color and explicit hiding through the
public OCaml API. Native/protocol/Core/lint checks, the root gallery and a rerun
of the fresh installed binary pass. The first installed launch exposed no AX
window and remains unexplained; it is recorded as failed, not erased by retry.
Style schema -2 requires matching bridge packages. Original names/values and
selection remain intact. Broader label placement, ribbon gradients and catalog/
release gates remain open.

[Sankey presentation controls](evidence/sankey-presentation-och41.md) now expose
validated node corners, ribbon opacity/minimum thickness and label spacing.
Native/protocol/OCaml/lint checks and root/fresh-installed macOS chart walkthroughs
pass. Widened tiny flows retain their raw value and identity; zero flows remain
absent. Options schema 4 requires matching bridge packages. Rich multiline labels
are covered by the subsequent entry above; ribbon gradients and broader catalog/
release acceptance remain open. The adjacent
[sample walkthrough](../examples/charts/samples/sankey_presentation.md) adds partial
OCH-48 coverage.

[Native chart inspection controls](evidence/chart-inspection-och41.md) now expose
typed card placement/appearance, crosshair axes/dashes/bands and marker styling.
Clean native/protocol/OCaml/lint checks and full root/fresh-installed macOS chart
walkthroughs pass, including actual pixels, selection, updates and cleanup. A
plot hitbox correction lets enclosing views scroll while retaining chart pointer
selection. Style schema -1 supersedes version 0 and requires matching bridge
packages. Rich tooltip rows, per-datum annotations and broader catalog/release
acceptance remain open. The adjacent [preset walkthrough](../examples/charts/samples/inspection.md)
adds partial OCH-48 coverage.

[Hosted run 37433332134](evidence/hosted-presentation-calibration-och17.md#repeat-on-hosted-run-37433332134)
is terminal: Linux foundation and all three extracted macOS apps on a separate
runner pass. The only macOS foundation failures are the two Metal presentation
probes (180/120 zero timestamps on Apple Paravirtual). The tested tree matches
`746b29b`, including stacked charts; newer ordinal/inspection changes remain
outside its coverage. No gate is waived.

[Stable ordinal chart colors](evidence/chart-ordinal-colors-och41.md) now expose
explicit namespaced keys, cyclic ranges and unknown-color policies. Full local
native/protocol/OCaml/lint checks, actual GPU pixels and root/fresh-installed
gallery walkthroughs pass, including reorder/selection, fallback changes and
scope teardown. Style envelope 0 requires matching bridge packages. Richer
chart presentation options and broader catalog/release acceptance remain open.
The [sample companion](../examples/charts/samples/ordinal_colors.md) adds partial
OCH-48 coverage; the completed every-example inventory is linked above.

[Stacked Cartesian charts](evidence/stacked-charts-och41.md) now add typed opt-in
cumulative bars/areas, shared area sampling/curves and raw-value selection with
explicit stack bounds. Local native/codec/GPU checks and root/fresh-installed
gallery walkthroughs pass, including signed values, missing observations, all
four directions, Grouped/Stacked changes and scope teardown. Options envelope 3
requires matching bridge packages. Ordinal colors, richer presentation options
and broader catalog/release acceptance remain open. The adjacent
[sample walkthrough](../examples/charts/samples/stacked.md) adds partial OCH-48 coverage.

Run [37421611438](https://github.com/dakotamurphyucf/gpuio/actions/runs/37421611438)
is terminal: Linux foundation and the independent fresh macOS extracted-app job
pass. macOS foundation fails only the GPUI Metal presentation hook and standalone
Metal calibration, with 180/120 zero presentation timestamps respectively on
Apple Paravirtual. [Verified reports and source identity](evidence/hosted-presentation-calibration-och17.md#repeat-on-hosted-run-37421611438)
cover branch `23650c8`, not the newer menu/controller or chart changes. No gate
is waived; final-source and release qualification remain open.

[Typed categorical charts](evidence/categorical-charts-och41.md) now add explicit
category IDs/order, missing observations and native Auto/Point/Band layouts.
The shared preparation path borrows source data, preserves category-aware sampling
and selection spans, and exposes category labels/IDs in tooltips and the original
paged table. Local native tests and the fresh installed gallery walkthrough pass;
source updates, layout changes, missing-value browsing and teardown are covered.
The options envelope advances to version 2 and requires matching bridge packages.
Ordinal colors and richer presentation options remain catalog work.
The [sample walkthrough](../examples/charts/samples/categorical.md) adds OCH-48
coverage without completing the every-example inventory.

[Four Cartesian value directions](evidence/chart-directions-och41.md) now have
public OCaml options and a gallery reversal control. Local paired-codec,
signed mixed geometry/provenance, native hit-index and real GPU pixel checks pass;
a fresh installed gallery passes all four directions, selection, original-data
browsing and scope cleanup. Ordinal colors, richer labels/tooltips and broader chart/catalog/release
qualification remain open. The adjacent
[chart-page walkthrough](../examples/gallery/charts_page.md) adds partial OCH-48
coverage; it does not complete the every-example inventory.

[Native popup window/editor ownership](evidence/menu-multiwindow-och41.md) now has
a public two-window example and passing installed-consumer macOS qualification.
The expanded test exposed a popup remaining open after its owner became inactive;
the activation observer now cancels that owner's native tracking lease without
restoring its focus. Independent dispatch, editor focus/removal guards and owner
close/recovery pass. A separate [real-AppKit queued-start harness](evidence/menu-popup-queue-och41.md)
also passes Close, observer/definition replacement, hide/disable, owner
removal/remount and window-close cases before tracking begins, with a current
popup positive control. Broader catalog/release acceptance remains open.

[Native popup SVG icons](evidence/native-menu-icons-och41.md) now have typed
Core/Bonsai APIs, bounded worker rasterization and AppKit template snapshots.
The native suite passes 985 tests (two existing skips), and an installed public
consumer passes actual icon pixels, clearing, release during tracking, retained
readers and command dispatch. Full OCaml checks and strict native lint pass.
New-source hosted checks and broader popup/catalog/release acceptance remain open.

Run [37410532617](https://github.com/dakotamurphyucf/gpuio/actions/runs/37410532617)
is terminal: Linux and all three extracted applications on the independent fresh
macOS runner pass. The macOS foundation job fails only the two presentation
probes, which again return zero timestamps on Apple Paravirtual. Navigation passes
in this run. [Verified reports and source identity](evidence/hosted-presentation-calibration-och17.md#repeat-on-hosted-run-37410532617)
cover branch `8cf5b5f`, not the newer popup/controller/icon work. No gate is waived.

The [native OS context-popup slice](evidence/native-popup-och41.md) now adds
`View.context_menu ~platform:true`, with AppKit on macOS and drawn Linux fallback.
Local and independently installed macOS tests pass physical opening, outside-window
bounds, nested selection, Escape, native Copy, owner removal, stale-command
rejection and window close during tracking. The close test first exposed main-queue
starvation; a main-run-loop callback now keeps native work progressing. Additional lifecycle cases and consolidated gallery acceptance remain open. No Linux GUI or VoiceOver acceptance is claimed.
The [owner-transition follow-up](evidence/native-popup-och41.md#owner-transition-and-recovery-follow-up)
now passes definition replacement, hidden/disabled ancestors and modal entry,
including actual keyboard recovery and retained editor content. A fresh installed
consumer passes the complete eight-run popup matrix. The added CI step awaits
hosted execution; multi-window/editor-target
qualification and consolidated acceptance remain open.

[Positioned menu commands](evidence/menu-commands-och41.md) now add a public
OCaml `Menu_controller`, validated logical coordinates and correlated Show/Close.
A fresh installed consumer passes actual AppKit placement, same/different-owner
Busy, explicit close, selection, observer detach/recovery and stale-definition
rejection. Native/protocol suites pass 981/416 tests (two existing native skips);
full OCaml checks and strict lint pass. Multi-window/editor
focus coverage and consolidated catalog/release acceptance remain open.

Run [37400903839](https://github.com/dakotamurphyucf/gpuio/actions/runs/37400903839)
at older branch `2e9cd54` is terminal: Linux passes; macOS fails native navigation
resize and both presentation probes; the fresh receiver is skipped. The viewport
wait passed but child width remained 500 rather than 400. [Raw reports](evidence/hosted-presentation-calibration-och17.md#repeat-on-hosted-run-37400903839)
retain that failure and the repeated zero presentation timestamps. Current-source
qualification and the newer receiver scheduling remain pending; no gate is waived.

Run [37398392336](https://github.com/dakotamurphyucf/gpuio/actions/runs/37398392336)
at branch `c0694a2` is terminal: both platform builds fail on missing palette
event cases in two low-level examples; macOS also fails both Metal presentation
probes. Later unit/consumer/window checks and the fresh receiver are skipped.
[Retained reports and correction](evidence/hosted-presentation-calibration-och17.md#repeat-on-hosted-run-37398392336)
include a passing local all-example build and formatting check at `282daf0`
plus the explicit event-match patch. Corrected hosted validation remains pending.
No gates are waived.

Fresh-machine package qualification now has [independent receiver scheduling](evidence/package-runtime-och17.md#independent-receiver-scheduling--2026-10-06-utc):
a later native/timing failure no longer suppresses staging from a successful
build. Artifact checks and both foundation gates remain required. Local lint and
eight transfer/admission tests pass; the changed scheduling awaits hosted execution.

The [window-placement follow-up](evidence/window-readiness-och17.md#owned-window-placement-follow-up--2026-10-06-utc)
now passes both complete local window walkthroughs. It moves only the test window
away from the usual notification area and records display/placement geometry;
physical pointer ownership remains mandatory. Hosted confirmation remains open.

Work is tracked on branch `milestone-07-gallery-release` and draft PR #16.
Latest fully passing hosted checkpoint: [37286788836](https://github.com/dakotamurphyucf/gpuio/actions/runs/37286788836)
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
The later [run 37321333808](https://github.com/dakotamurphyucf/gpuio/actions/runs/37321333808)
also passes Linux and the other macOS checks, but both the GPUI hook and independent
Swift/Metal calibration return only zero presentation timestamps on the hosted
Apple Paravirtual device. The fresh receiver is skipped. [Calibration evidence](evidence/hosted-presentation-calibration-och17.md)
is preserved; neither failure nor any gate has been waived.
Run [37374125077](https://github.com/dakotamurphyucf/gpuio/actions/runs/37374125077)
at `4c959f5` is now terminal: macOS failed custom-chrome pointer ownership and
both presentation probes; Linux was cancelled before receiving a runner, and the
fresh receiver was skipped. [Exact reports](evidence/hosted-presentation-calibration-och17.md#repeat-on-hosted-run-37374125077)
retain 180 GPUI and 120 standalone zero-time frames on Apple Paravirtual. The
newer local palette/menu changes remain outside this run's coverage.
The custom-window driver now has a [locally passing readiness follow-up](evidence/window-readiness-och17.md):
foreground/stable physical ownership is required before the Fullscreen click,
with bounded occluder diagnostics. Both complete local window walkthroughs pass;
the hosted failure was not reproduced locally and still needs hosted confirmation.
The owner lifted the VoiceOver hold on 2026-10-05; accessibility validation may
resume. VoiceOver acceptance remains open.
The subsequent [run 37343201640](https://github.com/dakotamurphyucf/gpuio/actions/runs/37343201640)
at branch checkpoint `470210a` finished with failure on 2026-10-05. Linux passed;
macOS failed scoped file/theme reads, calendar accessibility, sidebar motion
sampling and both Metal presentation probes. The fresh extracted-app job was
skipped. [Terminal probe reports](evidence/hosted-presentation-calibration-och17.md#repeat-on-hosted-run-37343201640)
again show all-zero timestamps in GPUI and standalone Metal on the paravirtual
device. This run does not qualify the newer local numeric-slot and popup-focus
repairs or expanded walkthroughs.
The two available failure logs now have [local readiness corrections](evidence/macos-ci-readiness-och17.md):
the calendar acknowledges focus before keys, and file-theme Reload waits for its
page scope to become ready. The full calendar walkthrough and all 12 theme cases
pass locally. Corrected hosted execution remains required; no failures are waived.
The same run later failed the public sidebar motion sampler after its AX searches
missed an intermediate width. The [sampling correction](evidence/sidebar-ci-sampling-och17.md)
now passes locally on both left and right layouts; corrected hosted execution is
still pending. The [motion catalog review](catalog/motion-review.md) maps all eight
pinned source inputs and names remaining timing/easing gaps without claiming parity.
[Command/menu](catalog/commands-menus-review.md) and [chart/plot](catalog/charts-review.md)
reviews now cover the other previously pending detailed source mappings, with
public-surface gaps and accepted-contract differences explicit. Exact cubic
ease-in/out presets now have [local API/native/gallery and consumer-build evidence](evidence/cubic-easing-och41.md).
This does not close component or release acceptance.
The subsequent piecewise cubic preset and all four stepped-easing policies have
[local API/codec/native-state evidence and a passing installed Motion walkthrough](evidence/stepped-easing-och41.md).
At `6bc15d3`, the fresh public consumer builds and passes the full macOS sequence,
including discrete widths, interruption, reduced motion, shared phase and cleanup.
Broader motion/catalog and release qualification remain open.
Piecewise-linear easing now has a typed 2–256-stop API, native shared curves,
bounded position inference/decoding and a [passing installed Motion walkthrough](evidence/stepped-easing-och41.md#piecewise-linear-stops--2026-10-05).
Variable curve payloads count toward program limits and retained memory.
Signed initial delays now have [public API, codec, native-state and installed macOS evidence](evidence/signed-delay-och41.md).
The complete Motion walkthrough passes. Explicit finite counts across the full
unsigned 64-bit range and all four playback directions now have
[API, codec, native-state and installed macOS evidence](evidence/animation-iterations-och41.md).
Legacy policies keep their semantics; finite completion and bounded timer waits
are documented. Broader catalog and release qualification remain open.
Drawn menu section labels now have [API, codec, native and installed macOS evidence](evidence/menu-labels-och41.md),
including passive accessibility text, keyboard navigation and context-menu focus
restoration. Platform bars reject labels explicitly. [Passive rich menu content](evidence/menu-content-och41.md)
now adds registered SVGs and composed labels, with paired transaction admission,
paint/animation lifecycle checks and an installed macOS pixel/interaction walkthrough.
Palette search/visibility/Escape policies now have [API, native lifecycle and installed macOS evidence](evidence/palette-policies-och41.md). [Grouped palette presentation](evidence/palette-layout-och41.md) now adds stable groups, filtered passive headings and separators with retained measured scrolling, paired codecs, native regressions and an installed macOS walkthrough. [Rich palette content](evidence/palette-content-och41.md) now adds measured command rows and interactive header/footer/empty Views, with native lifecycle/1,000-row/focus tests and a passing installed macOS walkthrough. Query/highlight controls now have [local controller qualification](evidence/palette-commands-och41.md); [Native loading](evidence/palette-loading-och41.md) now preserves editing/undo and query identity with native lifecycle/reduced-motion and installed macOS typing/undo/pixel evidence. [External results](evidence/palette-external-results-och41.md) now have local query-fenced publication and installed delayed-search/typing evidence. [Persistent embedding](evidence/palette-embedded-och41.md) now has normal-layout/focus, repeated dispatch, nested scrolling, transition restoration and installed macOS keyboard/document evidence. Consolidated palette/menu family acceptance remains open catalog work.
The current unmodified table smoke at `5c3956d` fails the 100 ms startup gate
at approximately 147.364 ms. A separate paced replay also reproduces the owner's
reported table flicker: three overlapping rows remain populated while newly
visible rows briefly have empty cells. [Failure and screen evidence](evidence/table-scroll-flicker-och17.md)
preserve both open issues; successful traversal does not establish visual stability.
The subsequent bounded programmatic-scroll correction passes dev/release
virtual-list tests and a complete functional table smoke. Paced screen samples
no longer show the blank-row pattern; a fast capture retains similar evidence
but ends with a screenshot-deadline failure. [Correction and limits](evidence/table-scroll-flicker-och17.md#bounded-command-preparation--local-correction)
leave ordinary wheel behavior and presentation/startup qualification open.
The subsequent native pixel-wheel diagnostic moves through rows 0–48 and back
with 32/96-pixel inputs; its 49 samples remain populated. Physical trackpad
momentum, changed geometry and complete frame coverage remain outside that evidence.
The owner also visually reports that the table flicker looks fixed.
Run `37356882651` finished with failure: Linux passed; macOS failed Settings
composition, navigation resize and both Metal probes. The fresh receiver was
skipped. [Terminal presentation reports](evidence/hosted-presentation-calibration-och17.md#repeat-on-hosted-run-37356882651)
again retain all-zero timestamps on the paravirtual device. [Bounded readiness
corrections](evidence/macos-ci-readiness-och17.md#settings-and-navigation-follow-up--run-37356882651)
pass both complete local walkthroughs; corrected hosted execution remains pending.
A subsequent [nonconstant lifetime-seed optimization](evidence/presentation-startup-investigation-och17.md#nonconstant-lifetime-seed--2026-10-05-follow-up)
passes full development/release OCaml suites and a fresh installed button/menu
walkthrough, including branch retirement and reattachment. It removes most of
the traced Bonsai initialization cost, but its first uninstrumented table smoke
still fails startup at 104.322 ms against 100 ms. Performance acceptance remains open.
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

An [isolated production-codec diagnostic](evidence/bridge-codec-diagnostic-och17.md)
now records optimized local encode/decode timings and reproducible inputs. It
excludes FFI/queueing/native admission/rendering and does not qualify end-to-end
bridge latency or close performance acceptance.

The reference examples now have [clearer application/state/view boundaries and
local validation](evidence/example-readability-och17.md). Begin with the small
counter, then follow the gallery's Application → Component → Shell reading map.
Chat and Signal Studio keep optional acceptance runners separate from startup.
This onboarding improvement does not close release or catalog acceptance.

Palettes now expose [asynchronous query/highlight snapshots](evidence/palette-observation-och41.md)
with subscription/query identity, bounded delivery and a passing installed macOS
typing/navigation walkthrough. [Native query/highlight commands](evidence/palette-commands-och41.md) now have
correlated subscription checks and an optional current-native-query fence, with
full native/OCaml and installed macOS evidence. [Native loading](evidence/palette-loading-och41.md)
now has retained editing/undo, empty-content gating and reduced-motion/cleanup
coverage. [External results](evidence/palette-external-results-och41.md) now support bounded query-fenced publication after accepted command staging, with local native/OCaml and installed macOS evidence. [Persistent embedding](evidence/palette-embedded-och41.md) now reuses the controller with normal layout and retained native state; a reproduced modal entry callback race is corrected. Consolidated family/release acceptance remains open.

The public gallery and reference applications exist, with public Core/Bonsai/Eio
APIs and native Rust ownership. Local source reviews and behavior evidence cover
many catalog additions; neither root-module coverage nor compilation proves
whole-family acceptance. Start from [the catalog](catalog/README.md),
[family ledger](catalog/families.json) and [gallery evidence](evidence/gallery-och41.md).

Recent completed local checkpoints:

- Installed color hover previews pass 27 GPU cases across two themes/three sizes,
  retained keyboard draft/history, read-only cancellation and roving panel keys.
  Multi-month calendars pass 11 independent date-grid cases, real cross-month
  range keys, navigation and owner retention/remount.
  [Evidence and harness corrections](evidence/installed-calendar-color-och41.md)
  keep physical coverage limits explicit; no VoiceOver work was performed.

- Installed date presets now pass draft/Apply/Cancel/Escape/read-only checks.
  The expanded walkthrough exposed a nonmodal popup focus-loss bug while controls
  were unavailable; a native regression and shared focus-manager repair pass
  931 native tests (two existing skips) and a fresh installed picker walkthrough.
  [Evidence and preserved failures](evidence/installed-picker-presets-och41.md).
  VoiceOver remains untouched on owner hold; wider picker qualification stays open.

- Numeric frame slots now render once: screenshot review found duplication
  despite passing input checks, and a native regression reproduced it. The fix
  passes 930 native tests (two existing skips), strict lint, formatting and a fresh
  installed gallery: 24 numeric presentation/input/history cases and 16 OTP
  retention/input/policy cases. [Before/after evidence](evidence/installed-numeric-otp-och41.md).

- Installed sliders pass range-thumb keyboard/focus and bounds, linear and
  vertical logarithmic pointer preview/commit/cancel, policies, retained owners,
  remount and 32 static GPU cases. [Evidence](evidence/installed-sliders-och41.md)
  preserves the scrolling/grab-offset harness corrections and remaining scope.

- Installed split-button paint passes 20 Light/Dark GPU cases; menu placement
  passes 16 geometry samples, including actual root scrolling while open and
  Escape focus return. [Physical paint/placement evidence](evidence/installed-split-paint-menu-placement-och41.md)
  distinguishes this scope from the broader TestPlatform matrix and release work.

- Installed standalone radio/navigation checks pass real Tab/Shift-Tab, Space,
  Return, pointer selection of skipped stops, signed ordering, rich/plain labels,
  retained identity and page retirement/remount. The disabled AX selection
  projection and corrected test expectation are documented without adapter changes.
  [Evidence](evidence/installed-radio-navigation-och41.md). VoiceOver stays on hold.

- Fresh installed toolbar/toggle checks pass. Tooltip anchor identity and
  constant-configuration binding lifetime are now repaired; a fresh installed
  gallery passes the combined rich-button, appearance/Link, menu, split and
  command-hint sequence. Actual full OCaml dev/release inline suites pass after
  correcting release test configuration. Historical invalid release-suite claims
  are withdrawn; current performance still needs requalification.
  [Repairs and preserved failures](evidence/gallery-lifetime-repairs-och41.md).

- Fresh installed spinner, progress and checkable-control walkthroughs pass:
  actual animation/paint/input, retained editor undo through inert hiding, 24
  control part-bound cases, rich labels and lifecycle checks.
  [Evidence and corrected test assumptions](evidence/installed-indicators-och41.md).

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

- The managed-lifetime allocation optimization is withdrawn: constant-key branch
  reactivation reuses retired tokens. Earlier release-profile inline-suite claims
  were invalid because those tests were disabled by Dune's default profile policy.
  First-party test configuration now enables them; fresh regressions reproduce
  the failure. Lifecycle-safe allocation is restored and full dev/release suites pass; historical
  optimized workload reports remain scoped to their original binaries.
  [Investigation and repairs](evidence/gallery-lifetime-repairs-och41.md).

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
| OCH-17: real macOS IME, clipboard, focus, file drop, accessibility/VoiceOver and GPU | Historical scoped results exist. [Real multiline Japanese IME](evidence/multiline-ime-cancellation-och17.md) now passes wrapped and horizontally scrolled candidate/commit/undo/cancel checks after repairing cancellation rollback; the single-line Settings follow-up also passes at the same source. Other consolidated physical acceptance remains open. The source-view AX selection gap now has a [native repair and real macOS selection/range/copy checks](evidence/text-selection-och17.md), including Unicode, stale/disabled/composition rejection and password privacy. Rendered Markdown selection, visual character geometry and VoiceOver remain open. The owner lifted the earlier VoiceOver testing/configuration hold on 2026-10-05; that scope is authorized again, with acceptance still open. Other open work includes point-based AX coverage beyond the now-passing two-window editor regression; the original routing gap was identified in [the diagnostics review](catalog/diagnostics-review.md); [window accessibility](evidence/window-accessibility-och17.md) and [gallery evidence](evidence/gallery-och41.md) retain limitations. |
| OCH-17: performance and resources | **The following reports are historical evidence at their recorded revisions. After the [lifetime correction](evidence/gallery-lifetime-repairs-och41.md), rebuild and requalify current optimized workloads; measurements of the withdrawn allocation optimization do not establish corrected-source acceptance.** The [qualification plan](design/performance-qualification.md) now declares reference hardware, workload sizes and targets before optimized acceptance runs; the optional collector and three full optimized 10k loaded-list runs now pass locally. Six alternating ordinary/profiled comparisons pass, with median CPU +2.60% and RSS +1.60% in this workload; overlapping ranges are not a general overhead bound. The [paged-table workload](evidence/paged-table-performance-och17.md) exposed and now regression-tests compact-cell admission and horizontal visibility repairs; three full table runs now pass; explicit focused/unfocused idle is qualified separately below. Three optimized [resource-lifecycle runs](evidence/resource-lifecycle-och17.md) pass 3 warm-ups + 30 measured cycles with acknowledged registry cleanup and final-ten RSS growth below 64 MiB; native-entity and physical-memory evidence follow separately below. Three full optimized [growing-document runs](evidence/growing-document-och17.md) now pass 20 MiB aggregate growth and complete page traversal, with draw p95 ≤2.503 ms, p99 ≤3.638 ms and RSS ≤237,518,848 bytes; source-fallback and AX-selection boundaries remain explicit. Three full [streaming/typing runs](evidence/streaming-typing-och17.md) pass four 20 Hz streams with 1,200 native keys per run, exact content and cleanup, draw p99 ≤6.300 ms, input-to-submission p99 ≤10.429 ms and RSS ≤138,018,816 bytes. The [physical-memory audit](evidence/physical-memory-och17.md) subsequently found 2.52 GB retained after 33 closes despite the RSS pass; a macOS accessibility-adapter cycle repair now passes three full repeated audits, with no IOSurface category in all 99 closed checkpoints and peak settled physical footprint below 107 MB. Three optimized [explicit idle runs](evidence/idle-performance-och17.md) pass 60 seconds focused and 60 seconds unfocused each, with zero draws, verified editor blur and sampled native visibility. Three full [native entity audits](evidence/native-entity-retention-och17.md) pass all 90 post-warmup checks and 99 application retirement checkpoints, with final-ten RSS growth below 5.6 MB. Three full [renderer-device Metal allocation runs](evidence/metal-resource-och17.md) now pass on the M1 Max: all 99 closes retire their application resources, post-warmup closed allocations remain 4,587,520 bytes with zero final-ten growth, and closed IOSurface checks still pass. A [standalone Metal presentation calibration](evidence/metal-presentation-calibration-och17.md) now passes120 actual callbacks and establishes separate GPU/presentation/callback clocks. The [GPUI integration design](design/metal-presentation-qualification.md) and [actual renderer hook](evidence/metal-presentation-hook-och17.md) are implemented with local qualification. [Three streaming/typing presentation runs](evidence/presentation-typing-full-och17.md) pass; list startup, table/document repetition and presentation-collector overhead/resources remain open. Other release gates remain separate. Local ownership tests are only part of this gate. |
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

VoiceOver validation is authorized again following the owner's 2026-10-05
revision. Earlier dated evidence accurately records the hold at the time of those
runs; it is no longer an active restriction. Continue accessibility, performance,
catalog, public API and release requirements without claiming untested acceptance. Current branch changes still need their
own required hosted checks and review before delivery.
