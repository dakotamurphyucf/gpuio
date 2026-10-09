# Selection state: available formatting and one alignment owner

[selection_state.ml](selection_state.ml) and its [interface](selection_state.mli)
define controlled formatting/alignment values behind
[selection_preview.ml](../selection_preview.ml). The pure reducer knows selection,
availability and busy flags, not toolbar geometry, Bonsai graphs, native focus,
I/O or editor formatting commands. These buttons demonstrate application state;
they do not apply styles to a document editor.

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
./scripts/gpuio exec _build/default/examples/gallery/main.exe
```

Choose **Selection & actions**, find **Selection, with a clear owner**, toggle
formatting and All formatting options, disable Italic or make Bold busy. No model
entry point, assets or diagnostic flag exists. [Development](../../../docs/development.md)
explains prerequisites; this guide adds no keyboard, assistive or platform acceptance.

Read `Format`/`Alignment`, internal t, availability/accessors, `master`, then `apply`.
Format is Bold/Italic/Monospace and Alignment is Left/Center/Right. Their explicit
`all` lists control enumeration; labels are display strings while `command_id`
uses stable `gallery.format.*` and `gallery.align.*` IDs. Derived typed equality
compares variants rather than labels. Changing displayed text need not change command
identity.

Initial state has global enabled=true, italic_enabled=true, bold_loading=false,
Bold selected, Italic/Monospace unselected and alignment Left. `format_enabled`
requires global permission and additionally Italic permission for that item.
`format_loading` reports Bold's busy flag independently of global permission.
Internal `can_toggle` combines enabled and not loading. Busy/disabled changes retain
selection; they do not imply a reset. Alignment is one variant, guaranteeing a single
choice rather than independent contradictory selected Booleans.

`master` counts all three selections, including unavailable ones: none is Unchecked,
all is Checked, otherwise Indeterminate. Toggle_all uses
`Check_state.activate (master t)` to choose the bulk target; Indeterminate becomes
Checked. It changes only currently available items, preserving busy/disabled selections.
As a result, the complete master state may remain Indeterminate after repeated bulk
activation when unavailable children prevent reaching the requested target. The
[check-state contract](../../../lib/core/check_state.mli) supplies that activation rule.

`apply` first handles permission/busy toggles, then rejects queued Toggle_format,
Align or Toggle_all while globally disabled. It additionally rejects per-item
unavailable formatting. Valid toggles use the current Boolean, so two rapidly queued
toggles restore the original selection rather than reusing a captured rendered value.
Align simply replaces the variant. The abstract model is immutable and needs no
cleanup or cancellation.

The caller wraps `State.apply` in `B.state_machine0`, receiving reactive model and
injection effect. `let%arr` derives `Entry` records and command registry entries with
checked/enabled state; `V.command_button_with_content` receives stable command IDs
and a `Button.Config` loading flag. Native command activation schedules its injection,
then Bonsai derives updated command/button readouts. Additional connected/vertical/
single-group preferences belong to the caller, not this model. Native GPUIO handles
focus/input/rendering; this reducer rechecks availability when the action actually
runs. See [command.mli](../../../lib/core/command.mli) and
[button.mli](../../../lib/core/button.mli) for those concrete APIs.

For a concrete bulk trace, start with only Bold selected. Disable Italic and make
Bold busy: both keep their values. Toggle All formatting options: the current master
is Indeterminate, so target is Checked; Bold stays selected/busy, Italic stays false
and Monospace becomes true. A queued individual Bold toggle is rejected. Re-enable
Italic and clear busy, then bulk activation can select all; the next activation clears
all. Alignment can independently move Left→Right without selecting two alignments.

The [existing gallery expect tests](../../../test/gallery/gallery_test.ml) cover
unavailable children, partial bulk selection, latest-state rapid actions and exclusive
alignment. They were read, not executed for this guide. To add a format, extend the
variant, all/labels/IDs, internal representation, accessors and exhaustive reducer;
ensure master counts the same complete domain and bulk availability gates remain
consistent. To connect real editor formatting, treat editor commands/results as a
separate native controller boundary instead of assuming these preview Booleans
already changed editor content.
