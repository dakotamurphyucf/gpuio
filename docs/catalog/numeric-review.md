# Numeric inputs, OTP and workflow-stepper review — OCH-41

Current follow-up: [installed numeric/OTP evidence](../evidence/installed-numeric-otp-och41.md)
records the numeric slot-duplication repair, before/after native regression and
fresh installed macOS walkthroughs: 24 quantity presentation cases with retained
history/application steps, and 16 OTP retention cases with native input/history,
masking and read-only behavior. These supersede corresponding historical unrun
statements below. Timing, resource/performance and release gates remain separate;
workflow Stepper is not covered by those numeric walkthroughs. VoiceOver stays on hold.

Source review, 2026-10-01. This distinguishes existing numeric behavior from
remaining component coverage; it is not whole-family release acceptance.

## Pinned sources

Eight unmodified Git blobs from GPUI Kit
`84f57fdfcb4910623fb0bb7f795b077e249f9271` are recorded with SHA-256 hashes in
[sources/manifest.json](sources/manifest.json): base `number_input.rs` and
`otp_input.rs`, component `input/number_input.rs` and `input/otp_input.rs`, and
component `stepper/{mod,stepper,item,trigger}.rs`. Source snapshots are review
inputs, not build dependencies. Slider functionality is a separate family and
is not audited by this checkpoint.

## Numeric editing

| Pinned functionality | GPUIO equivalent and limits |
| --- | --- |
| Editable value, placeholder, focus and disabled state | `Number_input.Config` and Eio controller retain one native editor. Mount-only committed value and independent draft seeds are explicit. |
| Minimum/maximum/fixed step | `Numeric.Domain` validates finite, resolvable bounded intervals, min-anchored stepping and an irregular upper endpoint. GPUIO intentionally separates an unfinished draft from a committed value instead of treating invalid text as zero. |
| Increment/decrement keys and buttons | Native Up/Down, accessibility actions and `Number_input.Step_controls` sides/stacked/hidden. Repeat has a scoped native timer while held; no idle repeat task. |
| Current-value callback step policy and custom step handler | `Config.step_mode:Application` intercepts user keys/buttons/AX before stepping and emits an opaque request with current snapshot, direction and source. OCaml selects a value or declines through guarded resolution. Repeat waits without a timer for the reply; stale replies cannot overwrite edits. Proposals normalize to the existing domain and invalid/composing drafts retain rejection rules. Managed Eio handlers cancel scoped work and queue guarded declines on cancellation/failure, including ordinary-command saturation. Deterministic Eio/bridge tests pass; physical acceptance remains open; see the [request contract](../design/number-step-requests.md). |
| Styled prefix/suffix and custom button/input regions | `View.number_frame` adds stable leading/trailing ordinary views and passive decrement/increment content around the same numeric editor. Numeric disable reaches auxiliary descendants; read-only leaves their independent actions available. |
| Size, border/background, focused appearance, custom icons | `Number_input.Appearance` supplies bounded geometry and theme-resolved frame/editor/button styles. Native editor focus styles the whole frame; buttons expose hover/pressed/disabled paint and passive custom glyphs. Local TestPlatform retention, composition/history, paint, repeat/cancellation and side/stacked/hidden checks pass. Physical acceptance remains open. |
| Events | GPUIO emits ordered observation, draft change, commit/reject/cancel events, with source and numeric revision. Focus loss does not commit. Programmatic operations are distinguished from user events. |
| History and async ownership | Existing revision/lease guards, exact reads and composition rejection. Undo changes draft/history without reverting the independently committed value. These guarantees exceed the simple upstream facade but do not fill its missing presentation/policy features. |

The Numbers gallery now demonstrates all three step-control layouts on the same
editor, a `1e-` draft with committed value `12`, quarter-step normalization,
commit/restore/undo/redo, optional-empty policy, read-only and disabled behavior.
A custom-step switch selects quarter steps below 10, single steps below 50,
and steps of five above that, computed in OCaml through the public request API.
Frame and custom-symbol switches expose integrated Qty/units content and per-part
styling. The Qty prefix has an independent action. See the
[presentation contract](../design/number-presentation.md).
Its result messages are guarded against older command replies and page retirement.
It uses public APIs; controls do not replace drafts when changing layout/policy.
The build and existing gallery tests pass. Desktop walkthrough/visual review of
this expanded card remains outstanding.

## Segmented OTP

| Pinned functionality | GPUIO equivalent and limits |
| --- | --- |
| Fixed length, digits, initial value and replacements | Validated `Otp_input.Policy` (1–32), mount seed and explicit revision/lease-checked commands. GPUIO additionally supports ASCII alphanumeric codes and normalizes full-width accepted characters. Unbounded arbitrary programmatic strings are deliberately rejected. |
| Change/completion/focus | Native change and ordered completion boundaries; focus is part of snapshots. Initial/programmatic values do not manufacture user completion. Completion is not authentication. |
| Masking and disabled state | Retained masking, copy/cut and accessibility privacy policy, disabled/read-only gates. Snapshots still contain actual text; no secure-storage claim. |
| Selection, paste, IME and history | One native input handler, accepted-value versus provisional draft, atomic validation, selection, undo/redo and composition cleanup already exist. The pinned simple base OTP handles digit append/backspace rather than these full editing capabilities. |
| Grouped cell layout | `Otp_input.Appearance` supplies bounded group count, clamped to length with ceil-sized groups and no empty trailing visual groups. Native cells, glyphs, caret, selection and hit testing share positions. Live changes preserve one editor and retire stale geometry. |
| Per-cell size, colors, radius, focus treatment | Optional cell width, cell/group gaps, radius, border width and theme-resolved background/border/focus/selection/caret colors. Root style controls height/font/foreground. Focused border applies to every cell, preserving field-wide focus treatment. The gallery changes grouping and size on the retained editor. Local codec, admission and TestPlatform checks pass; physical visual/input acceptance remains open. |
| Blink and inactive lifetime | Native 500 ms phases run on one cancellable task while the focused editor is eligible and visible. Editing restarts visibility; appearance changes preserve phase. Composition/read-only/reduced motion use a steady caret. Blur, inactivity, hiding, clipping, disable, removal and close cancel timing. Fake-clock/native-paint and ownership checks pass; actual macOS timing and resource qualification remain open. See the [caret contract](../design/otp-caret.md) and [evidence](../evidence/otp-caret-och41.md). |

See the [OTP presentation contract](../design/otp-presentation.md) and
[local evidence](../evidence/otp-presentation-och41.md). The presentation API does
not create a separate input per cell or split provisional Unicode composition.

## Workflow Stepper is a separate component

The old root-module ledger incorrectly assigned `component/stepper` to numeric
inputs. Its actual API describes a wizard/workflow navigator: selected index,
horizontal/vertical layout, completed/current/upcoming indicators, connectors,
custom icons/content, centered labels, sizes, per-item/whole-control disable and
selection callbacks. Numeric side/stacked steppers do not provide this behavior.

The module now belongs with navigation in the ledger. `Gpuio.Stepper` provides a
validated model (stable IDs, at most 64 steps), latest-model requests and an
ordinary View composition with native rich-button triggers. Horizontal/vertical
layout, centered labels, custom passive content/indicators, configurable styles
and localized current/status/position semantics are implemented locally. Selecting
a stage does not validate or unmount associated content. The navigation gallery
keeps a notes editor alive through stage and presentation changes.

The pinned trigger's click handler alone is not a complete keyboard contract.
GPUIO uses existing native button semantics: Tab/Shift-Tab, Enter/Space and AX
Click. The exact public OCaml transaction is exercised by a native TestPlatform
test, including queued activation after disable. Physical macOS visual/keyboard/
VoiceOver and release acceptance remain open. See the
[contract](../design/workflow-stepper.md) and
[local evidence](../evidence/workflow-stepper-och41.md).

## Evidence boundaries

[Numeric/OTP evidence](../evidence/numeric-inputs-och34.md) records historical
OCH-34 native input, appearance, history and resource checks; those are not fresh
acceptance of the latest milestone-07 worktree. The current new gallery changes
have build/OCaml test coverage, not new OS input evidence. Current macOS physical,
performance/resource and installed-consumer qualification and Linux automated
checks remain required. Linux desktop qualification is deferred to OCH-47.

The source audit checks hashes and structural ownership only. It cannot establish
feature parity, as the corrected Stepper mapping demonstrates. Keep each gap
above open until it has a public contract, implementation and meaningful evidence.
