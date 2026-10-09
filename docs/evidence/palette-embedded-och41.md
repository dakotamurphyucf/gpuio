# Embedded command palettes — OCH-41

Local implementation based on `1760db5`, macOS 14.5 arm64 (M1 Max),
2026-10-06 UTC. This is component evidence, not complete palette/catalog or
release acceptance. The independent consumer builds and passes the complete
Feedback native walkthrough after the focus-order correction.

## Public behavior

[The contract](../design/palette-embedded.md) adds
`Command_palette.Presentation.Embedded` to the existing checked configuration.
The default remains Modal. Embedded choosers occupy ordinary layout, keep their
native query, and invoke repeated registry commands without closing. Escape
clears the query first when configured, then requests application cancellation
without discarding native state. Outside clicks do not dismiss the chooser.

Normal Tab traversal, nonmodal accessibility grouping, hidden/disabled state
retention and blocking by a higher modal use the shared native focus manager.
Native edit commands target the current/last eligible application document,
never the private query. No synchronous OCaml input/layout/paint callback is added.
Nested scrolling consumes only movement used by the list; its boundaries and
perpendicular gestures remain available to ordinary ancestors.

Live presentation changes retain query identity. Becoming modal recaptures the
eligible document target before the old focus gate becomes a trap. The shared
focus manager captures outside return focus at that transition; if focus already
belongs to the chooser, it preserves the previous outside return target. The
new tests first reproduced missing document targeting and incorrect Escape
restoration, then passed after the respective corrections.

The public OCaml gallery adds
[`Embedded_palette_preview`](../../examples/gallery/embedded_palette_preview.ml)
with a note editor, repeated command counter, clear/cancel feedback and retained
hide/show. Its ordinary application code contains no Rust or protocol access.

## Completed local checks

- Eight focused native tests pass: coexistence/repeated dispatch, hidden/disabled
  retention, higher-modal gating, native document selection, nested wheel
  consumption/bubbling, mode-change targeting, mode-change Escape restoration and admission between
  an earlier frame and its queued focus callback.
- The full native suite passes **979 tests**, with two existing ignored tests.
  Strict native/protocol all-target Clippy passes.
- Full OCaml `@runtest`, all examples (`@all`) and formatting checks pass before
  the final private Rust focus-order correction. The current native suite and
  independent consumer rebuild cover that correction; OCaml APIs are unchanged.
  Independent OCaml/Rust bytes include the appended presentation tag; malformed
  tags are rejected. The unchanged wire implementation's complete protocol suite
  passed **414 tests** in the preceding implementation slice, retained here.
- The complete local Feedback walkthrough passes real native Enter, Tab,
  Shift-Tab, Escape, document replacement and undo, plus AX-driven hide/show and
  explicit query focus. It verifies two invocations without dismissal, clear
  before cancellation, retained query on reappearance, and document selection
  without overwriting the private query. Existing Feedback checks also pass.
- Screenshot inspection shows a normal-flow chooser alongside its editor and
  counters. The screenshot is supplementary; keyboard and state assertions are
  the interaction evidence. The driver reaps the app and restores saved clipboard
  contents.
- A fresh independent gallery consumer builds against staged installed public
  libraries and passes catalog negotiation. After rebuilding its composed Rust
  backend for the focus-order repair, the complete installed Feedback walkthrough
  passes. Installed OCaml libraries and example sources are unchanged.

## Retained failures and coverage limits

Early failures identified hidden scopes missing a paint handle and modal hitbox
occlusion preventing ancestor wheel delivery. A stable inactive handle and the
visibility guard allow disabled paint without granting input; embedded surfaces
no longer install a modal occluding hitbox. The native regression suite covers
both repairs. One OCaml fixture required the new explicit default Modal byte;
expectation output was not automatically promoted.

The first local walkthrough reached repeated activation and Tab but the new
Python driver passed modifier lists where integer macOS flags are required.
Correcting Shift/Command flags fixed that harness error. An intermediate native
focus fixture also needed to deliver GPUI's post-paint callback with
`simulate_next_frame`; repaint alone does not execute that callback.

A subsequent full Feedback run failed the existing modal `Preview commands`
opening-focus assertion, before reaching the new embedded checks. AX reported
the window itself as focused. The failure log and screenshot are retained; this
repeats the previously reported intermittent modal-focus issue. Its cause is
unestablished, and the later passing full run does not establish a fix. The
passing local run began near the end of the all-example build, which completed
before the walkthrough finished; concurrent builds are context, not a diagnosis.

TestPlatform wheel/focus checks do not establish physical trackpad behavior.
These runs do not qualify a real IME candidate panel, VoiceOver, physical frame
cadence, Linux desktop behavior or complete family acceptance. Required Linux
build/unit/consumer and current-source hosted checks remain separate. Native OS
popup menus, other catalog work and broader OCH-17 release gates remain open.


## Focus callback ordering correction

The first installed consumer walkthrough repeated the modal opening-focus failure.
An exact-boundary native regression then reproduced a concrete ordering defect:
when a transaction admits a new modal before a previous frame's reconciliation
callback runs, that callback can consume the new entry request against old paint
entries. No query entry exists yet, so focus remains on the modal scope even after
its query subsequently paints. GPUI's native frame loop executes queued callbacks
before drawing new dirty content, making this ordering relevant to real windows.

The shared focus manager now records that a newly requested scope entry needs a
paint. An older callback leaves that request pending; the first paint clears the
flag and the subsequent reconciliation chooses the query. This adds no timer,
busy polling or forced redraw. The new regression fails before the change and
passes afterward. Earlier test variants that allowed intervening native work did
not pin the problematic boundary and passed; they were refined rather than
used as evidence of a fix.

The correction is a demonstrated race fix consistent with the observed symptom.
Historical failed runs lack internal focus traces, so they do not prove this was
the sole cause of every prior failure. The complete installed Feedback walkthrough passes after the fix; no focus
assertion was weakened or replaced with a forced query-focus request. The driver
reaps its app and restores the saved clipboard. Final executable SHA-256:
`fc1217ab05f886c3fcb81a632c6542d8cf3e809a9b4082be908a751545fbad77`.


[Source patch/new modules, exact commands, logs and screenshots](palette-embedded-och41/validation.tar.gz)
are retained with a [verified manifest](palette-embedded-och41/manifest.json).
Hosted run 37400903839 targets the older `2e9cd54` checkpoint and fails native
navigation resize and presentation probes; its Linux pass does not qualify this
newer implementation. Current-source hosted verification remains required.
