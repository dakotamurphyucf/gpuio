# Split action/menu walkthrough

Read [split_preview.ml](split_preview.ml) and its [interface](split_preview.mli). [pages.ml](pages.ml) mounts it on **Selection & actions**. `B` aliases Bonsai.Cont and `V` Gpuio_bonsai.View. `graph` hosts the reactive state computations; `let%arr` reads their current values and palette to derive GPUIO view descriptions.

`B.state_machine0` owns mode 0 and request count 0, returning each current model plus an action-injection effect constructor. Mode cycles both/action-only/menu-only. Three toggles start false for pair disabled, primary disabled and loading. Effects constructed during view building do not execute then. A native Split mode activation runs `cycle ()`, reduces against the latest mode, derives optional primary/menu parts, and reconciles the native pair.

Command `gallery.split.run` is registered in `V.command_scope`; both the keyed primary command button and keyed menu item invoke its same counter effect. Primary is wrapped in its own tooltip. `Button.Config.loading` blocks only primary activation while preserving normal focus; primary-disabled likewise leaves the menu independent. Pair Disabled applies to the composed control. Menu activation invokes the command, updates the counter model and derives the request readout; nothing performs real work.

[`Split_button.Appearance`](../../lib/core/split_button.mli) coordinates shared surface and menu-open painting in Rust. The helper does not create one combined focus/action owner: both halves retain ordinary accessibility, focus and input behavior. Per-half hover/pressed styles take precedence; disabled/loading halves receive no coordination styles. This file observes no hover/menu-open boolean to drive painting. Allowed coordination declarations are paint properties, not layout/input policy.

Open the menu, move away, compare shared surface, then load/disable only primary and try the menu. Switch to single-part modes to remove the other owner. Stable keys preserve surviving owners but cannot preserve removed parts. No native editor, resource scope or background task exists. Adapt with a real registry command, separate per-owner availability and explicit pair disabling when both must stop. Keep menu navigation native and avoid wrapping this composition in a second synthetic action.

## Run and review

From the repository root:

```sh
./scripts/gpuio build examples/gallery/main.exe
./_build/default/examples/gallery/main.exe
```

These repository-wrapper commands are instructions, not validation performed for this documentation change. There is no standalone executable or self-test for this component. Compilation does not establish native focus, keyboard, animation or platform acceptance. See [gallery instructions](README.md) and [development](../../docs/development.md).
