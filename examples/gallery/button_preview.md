# Rich command button walkthrough

Read [button_preview.ml](button_preview.ml) and its [interface](button_preview.mli). [pages.ml](pages.ml) mounts it on **Selection & actions** as “An action with room for detail”.

`B.toggle` owns busy, preserve-focus, skip-Tab and rich-content choices; only rich starts true. `B.state_machine0` increments the publish-request counter. `let%arr` reads these reactive values and constructs a command registry and view. The command `gallery.button.publish` is shared by two buttons; its `on_invoke` is the counter effect, so no publishing or background work happens.

The state is reactive: updates cause dependent `let%arr` computations to derive current view descriptions. `state_machine0` returns a current counter model and an action injector producing an effect; executing it reduces against the latest count. Native secondary-button activation resolves the publish command, runs its injected unit action, increments the counter, and derives a changed request readout that GPUIO reconciles. Constructing the callback does not execute `increment`. Clicking the busy checkbox instead executes its toggle effect, updates the busy model, and derives a primary configuration whose native owner blocks activation.

[`Button.Config`](../../lib/core/button.mli) supplies per-owner loading and focus policy. Busy blocks activation on the primary owner while keeping its ordinary focus eligibility; it does not disable the command or the secondary button. `Button.Focus.Preserve` preserves sibling focus on pointer activation and supplies no separate Tab/keyboard/semantic Focus target. Otherwise `Focusable (Tab_order.create ~tab_stop:(not skip) ())` allows pointer/accessibility focus even when excluded from Tab. The skip checkbox is disabled while Preserve is active, but its saved value remains for the next Focusable configuration.

With rich content, `V.command_button_with_content` wraps a row containing `V.loading` and two text lines. The spinner’s `Loading.Config` is animated only while busy; its presence is constant. Without rich content, `V.command_button` uses the command label. Both branches keep key `publish-primary`. The secondary uses `publish-secondary` and default policy. `V.command_scope` resolves both against the same registry, and named accessibility groups distinguish them.

Activate the primary, enable busy, then activate the secondary: only the secondary should add a request while primary is busy. Switch detailed content off and back to compare composition around the same primary key. Enable Skip primary with Tab to omit sequential traversal while retaining pointer focus; enable Preserve to make it an auxiliary focus-preserving action instead. The demo readout counts requests, not completed publication.

Bonsai owns the options and counter. GPUIO owns native button activation/focus and spinner animation; there is no application timer, asset handle or Eio worker to dispose. Adapt this by connecting the shared command to real work and deriving primary loading from that work’s lifecycle. Use command disabling when all owners must become unavailable, and per-owner loading when another owner should remain usable.

## Run and review

From the repository root, use the isolated repository wrapper:

```sh
./scripts/gpuio build examples/gallery/main.exe
./_build/default/examples/gallery/main.exe
```

These are instructions, not validation performed for this documentation change. This component has no standalone executable or self-test. Interactive behavior requires the native gallery; compilation alone does not establish keyboard, focus, accessibility or platform acceptance. See the [gallery README](README.md) and [development environment](../../docs/development.md).
