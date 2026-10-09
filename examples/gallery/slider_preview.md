# Slider controller walkthrough

[slider_preview.ml](slider_preview.ml) has no separate interface. [numeric_page.ml](numeric_page.ml) mounts `component window palette ~read_only graph` on **Numeric inputs**, supplying shared reactive read-only state. The constants validate linear domain 0–100/step 1, logarithmic domain 1–1000/step 1, single value 35 and range 20–80.

`B` is `Bonsai.Cont`, `V` is `Gpuio_bonsai.View`, `E` is `Bonsai.Effect`, and `Controller` is `Gpuio_eio.Slider`. The `graph` argument is where Bonsai allocates this component’s state and controller computations. Palette and read-only are reactive values supplied by the caller, not plain records/booleans. `let%arr` reads their current values together with local state to derive a view; `V` builds GPUIO descriptions for native reconciliation.

Six `B.toggle` values start false and own axis/scale/disabled/fill/colors/size choices; `B.state` owns the notice. Configuration `let%arr` reads these reactive values and caller read-only state. Two [`Gpuio_eio.Slider`](../../lib/eio/slider.mli) controllers own separate single/range placements, while Rust owns values, drag previews and commits. The outer `let%arr` derives views/readouts; setters/toggle effects are descriptions of future actions, not mutations during view construction.

Native Vertical sliders activation runs its toggle effect, updates the Bonsai model, derives new axis/configuration and dimensions, and reconciles both native owners. Scale/axis changes cancel active drags and keep values normalized to the new domain. Native dragging instead updates Rust’s preview; controller observations make `description` derive “Preview” and committed-value text. Escape/Cancel drag restores committed values. The Bonsai booleans are not the slider values.

`command` sequences effects with `let%bind`: increment a shared mutable epoch, await Controller.command, and publish a notice only if its epoch is still latest. `B.Edge.lifecycle` increments that epoch on deactivation, preventing late command results from overwriting the notice on a later visit. This is result-publication fencing; it does not undo native commands. Error messages distinguish stale revisions, policy blocks, unavailable placements and other failures.

Reset sends Replace with no revision guard, intentionally an unconditional programmatic reset; the public controller permits replacement even while disabled/read-only. Focus and Cancel have their own native policy checks. Auxiliary buttons use Button.Focus.Preserve. Appearance controls specify Selected/Remaining fill, track/thumb/target/ring geometry and independent colors; Remaining is for the single-value presentation. Dimensions are palette-scaled horizontal 320×40 or vertical 48×180.

Drag, change scale, inspect preview/committed values, then reset and focus each thumb. GPUIO owns focus/drag/layout, the adapter owns lease-aware commands, and Bonsai owns options/notices. No asset/task scope is created. Adapt with validated domain/values, handle every command failure, and use replace_if_unchanged when replacement must preserve intervening user changes rather than emulate this explicit Reset button. Store durable values outside destroyed native placements when needed.

## Run and review

From the repository root:

```sh
./scripts/gpuio build examples/gallery/main.exe
./_build/default/examples/gallery/main.exe
```

These repository-wrapper commands are instructions, not executed checks for this documentation change. There is no standalone executable or self-test for this component. Compilation does not establish native keyboard/IME/focus or platform acceptance. See [gallery instructions](README.md) and [development](../../docs/development.md).
