# Picker edge-case compositions

Read [choice_picker_cases.ml](choice_picker_cases.ml) and its
[interface](choice_picker_cases.mli). [choice_picker_preview.ml](choice_picker_preview.ml)
mounts `component` beneath capabilities on **Dates & colors**. Despite its name, this module
contains three Bonsai components, not a pure fixture list.

`B` is `Bonsai.Cont`, `E` `Bonsai.Effect`, `V` `Gpuio_bonsai.View`, `P` `Choice_picker`,
`Controller` `Gpuio_eio.Choice_picker` and `State` the [picker model](model/picker_state.md).
Graph hosts state/controller computations; `let%arr` reads current reactive values to derive
descriptions. `B.state_machine0` returns current model plus an injector constructing effects
that reduce against the latest model when executed. Effect construction does not change
selection or open a popup.

`appearance` and `trigger_style` are pure validated styling helpers using palette-scaled
geometry. `controlled` uses `State.initial` with no selected destination, opening permitted but
not requested/visible. It forwards `Selection_requested`, `Open_requested` and `Visibility` to
the reducer; `Query_changed` is ignored. `State.config` supplies controlled requested visibility
and disabled policy. Native trigger activation injects `Request_open`, reducer checks
permission, config derivation updates native requested state, and a later native visibility
event injects `Observe` to update its readout. Requested opening and observed visibility are
separate values. Permission off closes requested state; `Reset` clears selection/request without
inventing a visibility observation.

`directory` starts empty single selection over State.workspaces (4,096 named entries).
`workspace_config` enables substring search and clearability. Native selection injects its
semantic request, `P.Config.apply_request` reduces against the current catalog/selection, and
`let%arr` derives selected label/config for native reconciliation. Search text/scrolling/focus
stay native rather than a second optimistic selected-value cache.

`fresh_start` owns `(populated, selection)` initially false/empty. `Create` changes the
in-memory catalog to one workspace without selecting it; `Reset` returns empty
catalog/selection. Its optional empty view and interactive footer explain the next step; search
with a populated catalog can still have no matches. `Create` is a mock model update, not
filesystem/service work.

Open controlled chooser and turn permission off; search 2048/4096 in the directory; create a
first workspace and explicitly select it. The
[controller contract](../../lib/eio/choice_picker.mli) retains the native query across popup
closing, retires its lease when search is disabled or placement destroyed, and leaves
selection/catalog in Bonsai. No asset or Eio worker exists. Adapt with stable catalogs
preserving selected entries and current-model request reduction. Handle external validation
errors instead of relying on trusted-constant ok_exn; mock creation is not persistence.

## Run and review

From the repository root:

```sh
./scripts/gpuio build examples/gallery/main.exe
./_build/default/examples/gallery/main.exe
```

These repository-wrapper commands are instructions, not checks run for this documentation
change. There is no separate executable/self-test for this component. Compilation does not
establish native keyboard, focus, IME or platform acceptance. See
[gallery instructions](README.md) and [development](../../docs/development.md).
