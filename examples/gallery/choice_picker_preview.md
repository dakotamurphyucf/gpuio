# Grouped multi-choice picker walkthrough

Read [choice_picker_preview.ml](choice_picker_preview.ml) and its
[interface](choice_picker_preview.mli). [pickers_page.ml](pickers_page.ml) mounts it on **Dates
& colors**, alongside [additional picker cases](choice_picker_cases.md).

`B` aliases `Bonsai.Cont`, `E` `Bonsai.Effect`, `V` `Gpuio_bonsai.View`, `P` `Choice_picker` and
`Controller` Gpuio_eio.Choice_picker. Graph hosts reactive models/controllers; `let%arr` reads
their current contents to derive config/UI. `group` constructs validated groups: `Create`
contains Research/Drafting, Finish contains Review/disabled Export. Pure `config` enables
substring search, clearability and an application-owned multiple selection.

`B.state_machine0` begins with no selected IDs and returns the current model plus request
injector. Executed semantic requests reduce through
[`P.Config.apply_request`](../../lib/core/choice_picker.mli) against latest catalog/selection;
constructing an effect does not toggle immediately. Native Research activation emits
`Selection_requested`, executes the request effect, updates selection, and derives new
controller config/readout through `let%arr` for native reconciliation. Export/removed/wrong-mode
requests do nothing; Clear requires enabled clearable configuration and clears the selection
while preserving mode.

`Controller.create` routes selection events and observes closing to update the notice “Selection
kept · search draft retained”; query/open events are ignored here. Default popup visibility is
native-managed. [`Gpuio_eio.Choice_picker`](../../lib/eio/choice_picker.mli) keeps search
text/caret/composition/scroll/focus in Rust, with native query retained when popup closes.
Application selection is not its editor snapshot; search disabling/removal retires query leases.

`Controller.view` supplies popup appearance 360×300 bounds, grouped-header style and passive
rich option content for Research/Drafting. `P.Option_content.create` keeps the native selected
indicator; remaining options use default labels. Footer Clear capabilities is a separate
interactive action, not nested in an option. `Select` multiple, filter, close/reopen and clear:
selection remains application-owned while search draft follows native retention.

No filesystem creation, network search, asset registration or background worker happens. Catalog
membership including selected disabled entries is validated. Adapt with stable IDs, full
selected catalog preservation and current-model requests; use external search APIs for remote
work rather than interpreting Substring as asynchronous fetching. Add scoped tasks explicitly if
selection should launch work, and handle external validation errors rather than trusting ok_exn
constants.

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
