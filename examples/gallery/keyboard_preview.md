# Displayed keycaps and registered shortcuts

Read [keyboard_preview.ml](keyboard_preview.ml) and its [interface](keyboard_preview.mli).
[pages.ml](pages.ml) mounts it on **Presentation**. `B` aliases `Bonsai.Cont`, `V`
`Gpuio_bonsai.View`, `K` `Presentation.Kbd` and `Editor` Gpuio_eio.Text_input. Graph hosts
reactive state/controller computations; `let%arr` reads current values to derive command/view
descriptions.

Linux/refined/registered/reversed start false; enabled/shown true. A `B.state_machine0` cycles
index 0 through k/delete/left/é, while another counts invocations. Each returns current model
and action injector; effects reduce against latest state when executed. Native Keyboard key
activation injects a unit action, advances the model and derives a new Shortcut/command/keycap
trio for native reconciliation. Effect construction does not register or invoke it.

`Shortcut.create` supplies Primary and Override priority. `Command.create` associates it with
gallery.keyboard-example and counter effect. `V.command_scope` registers the command only when
the registration toggle is on; keycap creation itself never registers a binding.
[`Presentation.Kbd.create`](../../lib/core/presentation.mli) renders Filled/Outline/Plain
variants from the same declared shortcut with explicit Macos/Linux formatting and optional
font/padding/radius refinements. Platform formatting is display-only; routing still uses the
running machine’s platform and native policy.

The single-line editor stays mounted outside the optional labels, seeded “Type here; keycaps are
just labels.” Hiding/reversing the three named keyed groups affects display without removing the
editor/registry. A delivered enabled registered shortcut runs invoke(), updates the counter
model and derives invocation text. Displaying a keycap is no availability guarantee; native/OS
reservations and composition can affect actual delivery. See
[live binding inspection](binding_preview.md) for observed availability.

Type a draft, change displayed platform/key, hide labels and compare retained text. Explicitly
register/enable the command before trying it; disabled registry invocation does not run the
counter. GPUIO owns native editor/focus/routing, Bonsai options/counter and adapter editor
lease. No asset/task scope exists. Adapt with stable command IDs and meaningful localized
accessible shortcut labels, keeping display formatting separate from registration and
verification. Do not claim Linux execution acceptance from Linux labels.

## Run and review

From the repository root:

```sh./scripts/gpuio build examples/gallery/main.exe./_build/default/examples/gallery/main.exe
```

These repository-wrapper commands are instructions, not checks run for this documentation
change. There is no separate executable/self-test for this component. Compilation does not
establish native keyboard, focus, IME or platform acceptance. See
[gallery instructions](README.md) and [development](../../docs/development.md).
