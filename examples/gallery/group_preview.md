# Group-box walkthrough

Read [group_preview.ml](group_preview.ml) and its [interface](group_preview.mli). [pages.ml](pages.ml) mounts `component palette graph` on **Presentation**.

`B.state Presentation.Group_variant.Card` owns the reactive variant. `B.toggle` owns header/footer presence (true), slot refinements (false) and the content checkbox (false). `B.state_machine0` returns the current click model and a unit-action injector; the reducer increments the latest count when the effect executes. `let%arr` derives descriptions from current values. Constructing `set_variant choice` or `click ()` merely prepares an effect for later activation.

The local `control` builder makes 210-pixel ordinary buttons, all sharing the mock counter. `checkbox` maps booleans to checked states. [`Presentation.group_box`](../../lib/core/presentation.mli) receives stable key `group-preview`, width 440 and optional header/footer buttons. Card decorates the whole group; Plain adds no panel; Filled/Outline decorate and pad only the body. Refined slot styles add different header/body/footer padding. Stable wrappers preserve body identity as neighboring slots change. `named` supplies “Group content” and “Configurable group” accessibility groups; the helper itself adds no focus stop.

A native **Group: Outline** activation executes its setter effect, updates the variant model, and makes `let%arr` derive Outline composition and the layout readout. GPUIO reconciles the group while retaining surviving body controls. Check the content checkbox, then remove header/footer and cycle variants to exercise that continuity. Native Run group action activation injects a unit action, updates the latest counter and derives its readout; header/footer actions do the same.

**Keep group updates** only owns the checkbox’s checked value: it does not freeze updates, enable persistence or control retention. Removing header/footer actually removes their buttons; body continuity does not preserve removed descendants. No asset, native controller or Eio work is allocated; GPUIO owns native focus/layout and Bonsai owns application values. Adapt with useful header/body/footer content and explicit application actions. Keep real retention or persistence settings separate and name controls according to their actual behavior.

## Run and review

From the repository root:

```sh
./scripts/gpuio build examples/gallery/main.exe
./_build/default/examples/gallery/main.exe
```

The wrapper uses the repository toolchain. These commands are instructions, not checks executed for this documentation change. This component has no standalone executable or self-test. Native interaction requires the running gallery; compilation does not establish focus, keyboard, accessibility or platform acceptance. See [gallery instructions](README.md) and the [development environment](../../docs/development.md).
