# Native indicator appearance walkthrough

Read [control_appearance_preview.ml](control_appearance_preview.ml) and its
[interface](control_appearance_preview.mli). [pages.ml](pages.ml) mounts it on **Selection &
actions**. `B` is `Bonsai.Cont` and `V` `Gpuio_bonsai.View`; graph hosts reactive options, and
`let%arr` derives current appearance/control descriptions.

Custom/enabled/checked start true; large/labels-before/inert/mixed/rich start false. Radio
selection starts `Some Balanced`. Checkbox and switch intentionally share one checked boolean.
Native Stream responses activation runs toggle_checked, updates the reactive model and derives
both controls’ checked states plus readout; GPUIO updates their stable owners. Constructing
toggle/setter effects during view derivation does not execute them.

[`Control_appearance.create`](../../lib/core/control_appearance.mli) supplies size 18/32, switch
width 36/64, gap 12 and label placement. Base/Checked/`Indeterminate`/Disabled indicator styles
define paint; mark foreground is separate. These are checked configuration values, not
application selection. Only supported bounded geometry/states/properties are accepted, with
disabled styling taking precedence. Custom off omits appearance and restores native defaults.

Rich branches replace passive labels through checkbox_with_label/switch_with_label and
radio_group_with_labels while keeping keys appearance-checkbox/switch/radio and explicit
accessible names. Radio choices are Balanced/Fast/disabled Unavailable; rich overrides apply to
Balanced/Unavailable, leaving Fast’s default label. Native radio selection runs
set_selected(Some choice), updates model and derives controlled Choice.Config. The wrapper
independently applies Disabled(not enabled) and Inert inert.

Mixed forces checkbox display `Indeterminate` independently of checked. Clicking it still
toggles checked through the supplied effect; while Mixed stays true the checkbox remains
visually indeterminate, even as the switch/readout change. It is a demo override, not a reducer
for a full three-state business value. Turn Mixed off to reveal the shared boolean again.

Compare rich/simple labels, indicator geometry and order while selecting controls. Bonsai owns
booleans/radio selection; GPUIO owns native input/focus/paint. No controller, asset scope or Eio
work is allocated. Adapt with separate application values where checkbox/switch should be
independent, explicit indeterminate-state semantics and validated presentation. Disabled/inert
affect native interaction; they do not erase the application’s retained selected values.

## Run and review

From the repository root:

```sh./scripts/gpuio build examples/gallery/main.exe./_build/default/examples/gallery/main.exe
```

These repository-wrapper commands are instructions, not checks run for this documentation
change. There is no separate executable/self-test for this component. Compilation does not
establish native keyboard, focus, IME or platform acceptance. See
[gallery instructions](README.md) and [development](../../docs/development.md).
