# Developer-preview coverage reconciliation — C1 / OCH-41

Reviewed 2026-10-08 at `50e5a500`; latest implementation `500f9b4e`.
This closes **C1's inventory/reconciliation**, not final candidate CI, timing,
notices, publication or full platform qualification. It applies the owner's
[developer-preview scope](../milestone-07-closeout.md).

All **41 v1 families and five additional accepted capabilities** have public
interfaces, runnable example routes and scoped behavior evidence. The two other
root-module families remain explicitly classified: docking is post-v1; inspector
is excluded development tooling. No missing family implementation was found in
this bounded pass. This is not a claim that every upstream option, permutation
or current-candidate native path has been exhaustively qualified.

## Evidence and change impact

The family and addition ledgers supply interface/example paths; gallery routing
is implemented in `examples/gallery/pages.ml` and its child components. This
review matched those routes and the narrower results below, rather than treating
source presence as runtime proof. Linked reports retain original revisions,
commands, binary identities, failures, successes and platform limits. They must
not be interpreted as fresh executions at this review's HEAD.

The latest implementation changes since the hosted candidate are Bonsai scrollbar
routing and Eio exception-safe window cleanup. Their focused regressions, full
OCaml suite and applicable root/installed native checks pass in the
[scrollbar](../evidence/managed-scrollbar-routing-och41.md) and
[cleanup](../evidence/force-close-cleanup-och17.md) reports. The root/installed
galleries each pass 18 managed-scrollbar cases. Later planning commits are docs
only. Required final-candidate integration remains R2; old successful tests are
not automatically called current-source acceptance.

A snapshot of hosted run `37805065605` at `734fca1c` shows Linux complete/pass and
macOS still running, with 108 completed successful steps and no failures at that
snapshot. This is supplementary evidence only, not a passing whole run. Do not
restart it just because a progress observation ends. Current outcomes belong in
[the CI record](../evidence/milestone-07-ci.md).

## Required family mapping

Each row records an existing exercised scope and the relevant limit/follow-up.
Detailed mappings remain in [families.json](families.json). “Follow-up” means
OCH-164 under the preview policy unless a current ordinary-use defect is found.

| Family | Public example route | Existing behavior evidence | Limitation / remaining owner |
| --- | --- | --- | --- |
| `presentation` | Presentation + Settings | [Rich slots, settings fields, native editing and state/retirement scenarios.](presentation-review.md) | Platform/assistive permutations are follow-up. |
| `forms` | Text inputs | [52 form geometry cases and actual control/overlay point routing after label repair.](../evidence/form-label-point-routing-och17.md) | No complete screen-reader claim. |
| `avatar` | Presentation | [GPU fallback/source/clipping and fresh installed gallery behavior.](../evidence/avatar-native-consumer-och41.md) | Broader platform coverage deferred. |
| `rating` | Numeric inputs | [Native paint/state checks; public input/appearance evidence linked in family review.](../evidence/rating-gpu-och41.md) | Full assistive/motion qualification deferred. |
| `loading` | Presentation | [Installed custom-spinner source recovery, geometry, input and retirement.](../evidence/installed-indicators-och41.md) | No universal animation frame-rate promise. |
| `buttons` | Selection & actions | [Fresh installed combined rich-button/split/command-hint checks after identity repairs.](../evidence/gallery-lifetime-repairs-och41.md) | Optional appearance/OS combinations deferred. |
| `selection` | Selection & actions | [Installed keyboard/pointer/AX, Tab order and remount; checked/mixed groups also covered.](../evidence/installed-radio-navigation-och41.md) | Full screen-reader qualification deferred. |
| `select` | Selection & actions / Pickers | [Grouped and controlled selection, clearing, Escape and empty/create/reset.](../evidence/choice-picker-macos-och41.md) | Candidate IME and broader geometry deferred. |
| `combobox` | Pickers | [Real typing, Backspace, navigation/commit and retained-query reopening.](../evidence/choice-keyboard-och41.md) | No all-IME/platform claim. |
| `editor` | Text inputs + Settings | [Actual Japanese IME preedit/candidate/commit/cancel and undo/redo, wrapped/unwrapped.](../evidence/multiline-ime-cancellation-och17.md) | Full VoiceOver and all IME/display combinations deferred; code-editor/LSP remains post-v1. |
| `numbers` | Numeric inputs | [Repaired duplicate-slot painting; installed numeric input/history/application stepping.](../evidence/installed-numeric-otp-och41.md) | Additional repeat timing/permutation coverage deferred. |
| `sliders` | Numeric inputs | [Installed range/axis/log input, commit/cancel, policies and GPU appearance.](../evidence/installed-sliders-och41.md) | Arbitrary thumb replacements are not exposed. |
| `otp` | Numeric inputs | [Installed segmented typing/history, masking, normalization and remount.](../evidence/installed-numeric-otp-och41.md) | Physical caret timing and full assistive coverage deferred. |
| `calendar` | Pickers | [Installed pane/date-grid oracle and cross-month range behavior; preset evidence linked.](../evidence/installed-calendar-color-och41.md) | Additional locales/constraints and screen reader deferred. |
| `color` | Pickers | [Installed palette pixels, roving keys, draft/history/read-only and remount.](../evidence/installed-calendar-color-och41.md) | Broader channel/IME combinations deferred. |
| `dialogs` | Overlays | [Root/installed modal focus, confirmation, all-edge sheet inset and theme/size checks.](../evidence/overlay-macos-och41.md) | No unsupported interactive sheet resizing claim. |
| `popovers` | Overlays | [Root/installed fixed-point/corner geometry and interactive help.](../evidence/overlay-macos-och41.md) | Broader nested/IME/assistive combinations deferred. |
| `tooltips` | Overlays / Controls | [Focus-trigger replacement, Escape removal and focus return.](../evidence/overlay-macos-och41.md) | Physical per-frame motion qualification deferred. |
| `disclosure` | Navigation | [Six root/installed cases: draft/history retention, heading keys, disabled policy and Unmount.](../evidence/disclosure-macos-och41.md) | Expanded screen-reader/IME matrix deferred. |
| `breadcrumb-pagination` | Navigation | [Breadcrumb routing and bounded billion-page chooser with actual native input.](../evidence/pagination-macos-och41.md) | No all-locale/display-scale claim. |
| `workspace` | Navigation / multi-window examples | [Retained panels and 30 variant/theme/size cases; split-group evidence covers resizing.](../evidence/tabs-macos-och41.md) | Comprehensive docking remains post-v1. |
| `navigation-stack` | Journeys | [Root/installed push/replace/root/reset and native editor retention/remount.](../evidence/navigation-history-macos-och41.md) | Custom per-frame rendering uses the extension boundary. |
| `carousel` | Journeys | [Root/installed measured track, keys/editing, dragging/wheel and interruption checks.](../evidence/carousel-track-macos-och41.md) | Precise hardware trackpad/full motion qualification deferred; remount harness repaired separately. |
| `sidebar` | Journeys | [Root/installed branch activation, keys/pointer, appearance and page remount.](../evidence/sidebar-macos-och41.md) | Full VoiceOver/motion qualification deferred. |
| `managed-list` | Collections | [Root/installed retained offsets and range input; linked card/search/follow checks.](../evidence/managed-scrollbar-routing-och41.md) | Known long-list jitter/idle qualification remains P1; not a catalog omission. |
| `tree` | Collections / Tree Lab | [Hierarchy, lazy loading, selection/reveal/moves and public macOS behavior; scrollbar repair supplements.](../evidence/managed-trees-och38.md) | Broader assistive/OS combinations deferred. |
| `table` | Collections / Table Lab | [Native rich-header input/geometry and public table behavior; retained scrollbar checks supplement.](../evidence/table-header-fit-och41.md) | Read-only table; editable grids remain post-v1. |
| `documents` | Markdown & code / Highlighting | [Public profile keyboard actions, plugin navigation, updates, scrolling and source retirement.](../evidence/document-profile-macos-och41.md) | Flow-focus repair supplements; full screen-reader geometry deferred. Rich-text editing is not promised. |
| `charts` | Charts / Signal Studio | [Seven families with native/GPU/root/installed data, axes, labels, inspection and appearance evidence.](charts-review.md) | No arbitrary live pixel-bound fill callbacks; historical outlier tracked, not silently passed. |
| `motion` | Motion | [Typed iteration/direction semantics, native state and installed Motion walkthroughs.](../evidence/animation-iterations-och41.md) | General exit-presence/shared-layout/keyframes exclusions remain. |
| `commands-menus` | Feedback / Controls | [Palette/search and native popup/bar command, focus, ownership and installed scenarios.](commands-menus-review.md) | Native menus have scoped passive content/SVG limits; Linux uses drawn fallback. |
| `progress` | Feedback | [Installed progress geometry/input/center-editor retention and page teardown.](../evidence/installed-indicators-och41.md) | Further physical transition timing deferred. |
| `toasts` | Feedback | [Root/installed placements, dismissal/restoration, hover/focus expiry and reduced-policy checks.](../evidence/notification-gallery-och41.md) | Complete frame-by-frame and VoiceOver qualification deferred. |
| `clipboard` | Assets | [Public literal/current Unicode writes, native input and original-clipboard restoration.](../evidence/clipboard-och41.md) | Write success is invocation, not durable ownership. |
| `assets` | Assets | [GPU/AppKit and root/installed icon transforms, source and identity behavior.](../evidence/icon-transforms-och41.md) | No arbitrary filesystem icon lookup API implied. |
| `style-theme` | Styling details / all pages | [Real file load/reload, palette pixels, invalid-input recovery, independent windows and cancellation.](../evidence/gallery-theme-files-och41.md) | Functional vocabulary differs from upstream JSON/pixel presets. |
| `native-events` | Input & transfers / Observations | [Native event contracts and public gallery capture/bubble, key/focus/wheel delivery.](../evidence/input-observations-och41.md) | Raw events are not IME or synchronous OCaml vetoes. |
| `geometry` | Styles / Responsive / Overlays | [Checked placement/axis/edge mappings backed by native geometry and container-rule tests.](geometry-review.md) | No generic OCaml layout callback or bounds subscription. |
| `window` | Shell / Runtime | [Real standard/custom windows: minimize/restore, draft/history, move/fullscreen/resize.](../evidence/window-lifecycle-och41.md) | Linux GUI and additional OS gesture policies unqualified. |
| `runtime` | Runtime / all pages | [Full OCaml cleanup/wakeup/reentrancy regressions; Scope repair and native runtime evidence supplement.](../evidence/force-close-cleanup-och17.md) | Core performance/resources still P1/P2; no per-component timing claim. |
| `diagnostics` | Runtime / Observations | [Actual system AX point routing; public counters have documented units/limits.](../evidence/form-label-point-routing-och17.md) | Diagnostic output is not by itself screen-reader or performance acceptance. |

## Accepted additions outside the root-module map

These five are part of preview coverage too; they must not disappear from the
count just because the pinned UI toolkit does not export corresponding modules.

| Capability | Public example route | Existing behavior evidence | Limit |
| --- | --- | --- | --- |
| `canvas` | Canvas / Signal Studio | [Retained scene/input/selection/drag/viewports and native cleanup.](../evidence/canvas-och24.md) | General multimedia is not included. |
| `native-extensions` | Extensions / independent consumer | [Independent native package, catalog negotiation, typed events and lifetime checks.](../evidence/extensions-och23.md) | Static linking; no dynamic plugin ABI. |
| `container-rules` | Responsive | [Native breakpoint selection, retained owners and bounded observation.](../evidence/container-queries-och26.md) | No synchronous OCaml measurement callbacks. |
| `desktop-integration` | Desktop / reference apps | [Readiness-aware links/document integration and packaged consumer evidence.](../milestone-6.md) | Real signed/notarized delivery remains OCH-164. |
| `os-notifications` | Desktop / Signal Studio | [Native macOS/public notification and private-bus lifecycle tests.](../evidence/os-notifications-och28.md) | Permission/service availability is queried; Linux desktop remains OCH-47. |

## Concrete remaining preview work

This pass found **no additional missing-family implementation blocker**. It does
not close these already-named gates:

- **P1:** long variable-height list scrolling/jitter and unexplained idle draws;
  obtain the prescribed valid full trials after a targeted investigation. The
  sampled geometry and interrupted attempts do not establish resolution.
- **P2:** establish sufficient collector-overhead validity and current-candidate
  impact review for rapid updates/typing, idle and bounded resource retirement.
- **D1:** finish actual release notice/input review; verified provenance is only
  part of that requirement.
- **R1:** finish the preview install/API/known-limits/feedback path review.
- **R2:** finish candidate integration/CI, explicitly handle unavailable hosted
  timing without masking actual failures, then review/tag/publish the preview.

C2 has no newly discovered independent catalog defect to implement from C1;
keep it pending the remaining integration results, with P1 linked rather than
opening a duplicate scrolling task. A new reproducible normal-use failure gets
one named owner and targeted regression. It does not trigger another complete
permutation audit.

Historical notes about unrun input, scrollbar routing, carousel remount, rich
labels and cleanup must be read alongside their later repair reports. Full
VoiceOver, every display/gesture combination, stable API promises and signed-app
qualification are explicitly unclaimed and tracked in OCH-164. Linux desktop
remains OCH-47. These follow-ups do not erase known ordinary-use failures.
