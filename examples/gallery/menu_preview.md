# Menu visibility walkthrough

Read [menu_preview.ml](menu_preview.ml) and its [interface](menu_preview.mli).
[pages.ml](pages.ml) mounts it on **Selection & actions**. `B` aliases `Bonsai.Cont`, `V`
`Gpuio_bonsai.View`; `graph` hosts reactive state and `let%arr` reads current values to derive
descriptions.

`open_` starts false and stores an observation, not requested popup state. Observe starts true;
disabled/right/trailing false. `B.state_machine0` returns current count plus unit-action
injector that increments the latest model when executed. Constructing setters/injectors during
view building does not execute them.

One registry command `gallery.menu.preview` is used by both the root action and More previews
submenu. `Menu.create` supplies menu-level disabled policy. `Placement.create` prefers
Bottom/Right, Start/End and offset 8; this affects root placement rather than submenu
navigation. Stable key observed-preview-menu identifies the native trigger.

Native menu opening produces `on_open_change true`; its `set_open` effect updates the model and
`let%arr` derives “open” for native readout reconciliation. Navigation/activation does not wait
for this callback. The [View contract](../../lib/core/view.mli) emits an initial visibility
sample when attached, later root transitions, but not every submenu/style change. Definition
changes retire queued old observations; node removal retires the callback without a final close
call. Observe off omits callback and displays “not observed”, so the stored boolean is not
authoritative while detached.

Activate either command to inject a count action, derive a new request readout and update native
text. Try observing off/on, disabled and placement changes. GPUIO owns popup
focus/navigation/visibility; Bonsai owns display preferences and last sample/count. No
controller/resource/task exists. Adapt with a shared registry and observations as diagnostics;
do not use the last callback value as an independently controlled open state or assume preferred
placement is exact screen geometry.

## Run

From the repository root:

```sh
./scripts/gpuio build examples/gallery/main.exe
./_build/default/examples/gallery/main.exe
```

These commands were not executed for this documentation change. The component has no standalone
executable/self-test; compilation does not establish native interaction or platform acceptance.
See [gallery instructions](README.md) and [development](../../docs/development.md).
