# Numeric draft and application-step walkthrough

Read [number_preview.ml](number_preview.ml) and its [interface](number_preview.mli). [numeric_page.ml](numeric_page.ml) supplies window, palette and shared reactive read-only state. Domain is 0–100 with quarter steps; committed seed is 12, while the independent mount-only draft is deliberately unfinished `1e-`.

`B` aliases `Bonsai.Cont`, `V` aliases `Gpuio_bonsai.View`, and local `Number` aliases the `Gpuio_eio.Number_input` controller adapter; `Number_input` remains the public model/configuration module. `graph` hosts Bonsai state/controller computations. Caller palette/read-only and local options are reactive values. `let%arr` reads their current contents to derive configurations and view descriptions for native reconciliation.

`B.state` owns step-control layout (Sides) and notice; toggles own allow-empty/disabled/custom-step/custom-symbols (false) and framing (true). Configuration `let%arr` derives Number_input.Config, choosing Native/Application step mode. [`Gpuio_eio.Number_input`](../../lib/eio/number_input.mli) creates one numeric controller; native Rust owns text, composition, undo and committed value. `classification` derives Empty/Incomplete/Invalid/Valid/Out_of_range readouts from observations, with a composition-specific message. These snapshots never replace the draft.

Native Choose step size in OCaml activation executes its toggle effect, updates the reactive model, derives application-step configuration, and reconciles the native editor. Constructing command/setter effects does not execute them. `command` uses effect `let%bind` to increment a request epoch, await a native command and publish notice only if latest; deactivation increments the epoch too. This protects notice publication, while native lease/revision checks protect edits themselves.

In Application mode a native step event invokes the view’s Step_requested handler. `adaptive_resolution` reads that exact request snapshot: Empty seeds zero; Valid/Out_of_range normalizes, and incomplete/invalid declines. It chooses amount 0.25 below 10, 1 below 50, otherwise 5, and proposes direction-adjusted value. `Number.run_step` executes work in the visit-owned [Preview_scope](preview_scope.md), resolves against the original lease/revision and suppresses completion after cancellation. The completion effect updates notice, causing view derivation/native text update. Missing scope declines the request. No callback accesses Bonsai from the worker; publication returns through effects.

Side controls, Stacked controls and Keyboard only execute setters selecting Sides/Stacked/Hidden without replacing the draft. Hidden removes step artwork, not keyboard stepping policy. `Number.view` receives the unfinished draft seed once per new mount, not every evaluation. Optional `V.number_frame` refines native frame/button styles and adds Qty action, units and optional arrow symbols. Commit normalizes/clamps finite drafts; incomplete/composing drafts reject without losing text. Cancel restores committed text; Undo/Redo change draft/history but not committed value. Losing focus is not a commit. Buttons disable while disabled/read-only and preserve editor focus on pointer activation.

Repair `1e-`, commit, undo and compare committed versus draft classification; then enable application steps and test thresholds. GPUIO owns edits/policy, the adapter owns guarded commands/step tasks, Bonsai owns options/notices, and the visit scope cancels step work. Adapt with external durable state and explicit error handling; do not infer a committed number from arbitrary draft text or use a delayed proposal to overwrite newer edits.

## Run and review

From the repository root:

```sh
./scripts/gpuio build examples/gallery/main.exe
./_build/default/examples/gallery/main.exe
```

These repository-wrapper commands are instructions, not executed checks for this documentation change. There is no standalone executable or self-test for this component. Compilation does not establish native keyboard/IME/focus or platform acceptance. See [gallery instructions](README.md) and [development](../../docs/development.md).
