# Workflow stepper walkthrough

Read [stepper_preview.ml](stepper_preview.ml) and its [interface](stepper_preview.mli). [navigation_page.ml](navigation_page.ml) mounts it on **Navigation**. This workflow stepper is distinct from numeric increment/decrement controls.

`B` aliases `Bonsai.Cont`, `V` aliases `Gpuio_bonsai.View`, and `Editor` aliases `Gpuio_eio.Text_input`. The `graph` argument hosts Bonsai state/controller computations. Palette and the workflow model are reactive values: `let%arr` reads their current contents and derives a view when those inputs change. `V` builds view descriptions; the `Stepper` module supplies the pure workflow model and its composition helper.

`definitions` gives stable Plan/Draft/Review/Launch IDs; `steps` constructs a validated Choice collection and optionally disables Review. Local `Action.t` is Navigate/Toggle_disabled/Toggle_review. The pure `apply` delegates to [`Stepper`](../../lib/core/stepper.mli), updating the model without starting work. `B.state_machine0` starts at Plan and returns a reactive current model and action-injection effect constructor. On execution, actions reduce against the latest model. `B.toggle` owns axis (horizontal), centered labels (true) and symbols (false).

Native Next stage activation executes `inject (Navigate Next)`, the reducer advances the latest current ID, and `let%arr` derives statuses/current label and Stepper.view. GPUIO reconciles the keyed stage buttons. Constructing that effect does not navigate immediately. Relative requests skip disabled steps, never wrap, and become no-ops while globally disabled. Disabling the current Review does not erase its identity; its historical selection is valid. `review_disabled` reads collection metadata to keep the switch controlled.

Appearance supplies palette-scaled 32-pixel indicators and status styles. `status_text` and optional symbol callback derive passive label content/✓/●/· from Completed/Current/Upcoming. Completed is positional—before the current stage—not evidence of validation or successful work. Stepper.view is a named navigation region with ordinary native button keyboard/AX behavior and stable IDs; it accepts at most 64 steps, and custom content must remain passive.

`Editor.create` allocates a separate multiline Workflow notes placement, seeded “A small idea, ready to take shape.” with 2–4 rows. It is outside stage selection, so changing stages does not swap editor content. Type notes, enable Skip review stage and use Next/Previous; change orientation/symbols to compare presentation around the same notes. Navigation does not validate or submit the draft.

Bonsai owns workflow/options; GPUIO owns button focus and editor text/history; the Eio adapter owns its editor lease. No page loader, asset registration or background worker exists. Adapt with domain-stable IDs, latest-model requests and explicit business transitions. If stages should change content/lifetimes, implement that separately; this model has no implicit task cancellation, persistence or form validation.

## Run and review

From the repository root:

```sh
./scripts/gpuio build examples/gallery/main.exe
./_build/default/examples/gallery/main.exe
```

These repository-wrapper commands are instructions, not executed checks for this documentation change. There is no standalone executable or self-test for this component. Compilation does not establish native keyboard/IME/focus or platform acceptance. See [gallery instructions](README.md) and [development](../../docs/development.md).
