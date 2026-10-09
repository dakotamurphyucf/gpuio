# Controlled selection and command-backed toolbar presentation

[selection_preview.ml](selection_preview.ml) and its
[interface](selection_preview.mli) demonstrate formatting/alignment state rather
than editing a document. Read Entry, state/toggles, toolbar command generation,
seam styles/accessibility and final controls. `B = Bonsai.Cont` constructs the
reactive graph; `V = Gpuio_bonsai.View` describes native controls; the pure
[Selection_state](model/selection_state.md) owns all selected/available values.

After [setup](../../docs/development.md), from the root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/gallery/main.exe -j 2
./scripts/gpuio exec dune exec examples/gallery/main.exe
python3 scripts/test_gallery.py --section selection
```

Choose Selection & actions. Try single alignment, combined formatting, disabled
Italic, busy Bold, orientation, connected/singleton geometry and Select all.
[README](README.md) separates native keyboard/pixel coverage and Linux scope.
No command or GUI check was newly run here; all labels are fixed local fixtures.

## Typed reducer and reactive state

Entry.t holds stable `Command.Id`, label, checked/enabled/loading flags and typed
`State.Action`. `state_machine0` applies `State.apply` against latest state on injection,
starting global/Italic enabled, Bold selected, no loading and alignment Left.
The reducer rechecks global/individual availability and busy status, so queued
intents cannot bypass newer policy. Alignment is one variant; formatting is three
independent Booleans. The presentation starts horizontal, connected and with multiple buttons per group.
`let%arr` combines palette/state/injector/preferences into views. act action returns
a deferred effect, not an eager mutation at render time.

The master Check_state counts all three selections, including unavailable ones.
`Toggle_all` derives the target from Check_state.activate but changes only currently
toggleable entries, preserving disabled/loading selections. The master can remain
Indeterminate when unavailable children prevent the requested target. This is an
explicit model policy, not automatic native select-all behavior.

## Commands own labels/actions; buttons own loading and styles

toolbar builds `Command.create` with stable typed IDs, checked/enabled and `on_invoke`
returning act action. `Command.Registry` plus View.`command_scope` makes those commands
resolvable by descendant command_button_with_content. Button accessible name comes
from command label. Its passive content is text plus a labelled spinner when loading;
`Button.Config.loading` blocks that owner's activation while retaining focus. It
does not start asynchronous work or change the command's enabled policy. See
[Button](../../lib/core/button.mli) and [View](../../lib/core/view.mli).

Connected buttons share seams: each preceding button paints its bottom/right
shared edge; later buttons suppress top(vertical)/left(horizontal) border. First/
last corners round according to orientation; a singleton is both first and last,
retaining all four corners/edges. Disconnected buttons have radius 8/border 1/gap 6.
Checked/Focused/Disabled state styles change surface/accent/border/opacity; selected
values survive geometry changes. An Accessibility.Toolbar role declares horizontal
or vertical semantics, while instructions use Tab then Space/Enter. This source
does not implement a separate arrow-roving toolbar controller.

Final state creates alignment entries from all choices (or just current alignment
in singleton mode) and formatting entries from all (or Bold only). Model values
are not deleted when a button is absent. Disabled/global/busy toggles remain
separate from formatting selection; All formatting options is disabled globally.

Trace: make Bold busy → Button owner retains focus/checked appearance but refuses
activation → `Toggle_all` effect reaches latest reducer → only available Italic/
Monospace change → `let%arr` re-derives master/command checked flags. Busy does not
claim a real formatter job or erase Bold selection. No resource scope/Eio task
exists here; branch models survive page visits while native controls unmount.

To add a format, extend pure enumeration/selection/master/action/ID policy and
these entries; retain stable IDs independent of labels. For a real editor action,
add an explicit validated editor command/effect boundary rather than assuming
selected toolbar values mutate document text. Pure model tests (linked in its
guide) and native geometry/input driver cover distinct layers.
