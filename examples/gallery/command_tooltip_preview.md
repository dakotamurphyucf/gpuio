# Command shortcut tooltip walkthrough

[command_tooltip_preview.ml](command_tooltip_preview.ml) has no separate interface. [pages.ml](pages.ml) mounts `component palette graph` on **Selection & actions**, in “Hints that follow the action”.

Four `B.toggle` values own alternate shortcut, assignment, enabled state and Linux label formatting; assignment and enabled start true. `B.state_machine0` owns the request counter. The command ID `gallery.hinted-action` is a validated constant. A native binding observer is mounted with key `command-hint-observer`, a constant [`Command_binding.Config`](../../lib/core/command_binding.mli) using Context.here and that command target. Here describes the enclosing command registry without following unrelated editor focus.

The pure `hint` function finds the matching command in a Ready observation. Registry entries expose enabled state and candidate shortcuts; absent commands, unavailable native entries and terminal/suspended observer states have separate fallback messages. This preview displays assignment information, not each candidate’s disposition. “Assigned shortcut” therefore does not prove that the OS or current focus will deliver that key. Candidate dispositions matter if you adapt this into an availability inspector; compare the [binding preview](binding_preview.md).

Bonsai values are reactive. The counter’s `state_machine0` returns its current model and an action injector producing an effect; executing it increments the latest model. Native activation of the enabled command button resolves `gallery.hinted-action`, runs the injected unit action, updates the count, and causes the outer `let%arr` to derive the new native readout. Constructing `invoke ()` inside `on_invoke` is separate from executing the returned effect. A shortcut-assignment checkbox changes its reactive toggle model, the outer computation derives a new registry, native sampling publishes its bindings, and the observer callback derives updated tooltip content.

Inside the observer callback, `let%arr` reads palette, observation and display platform. `Presentation.Kbd.create` formats candidate shortcuts. A native-managed `V.tooltip` has width 280, fixed semantic label “Command shortcut hint”, a content column with current description/caps and a `V.command_button` anchor keyed `hinted-action`. [`Tooltip.Config`](../../lib/core/tooltip.mli) owns transient visibility natively; the application does not track hover to open it.

The outer `let%arr` creates a registry command with Primary+Shift+H or J, omitting its shortcut list when assignment is off and supplying `enabled`. `V.command_scope` wraps the observed button. Observation does not register the command; this enclosing registry does. Its `on_invoke` only increments the local counter.

Hover or focus the action, alternate its shortcut, then remove assignment: the tip follows the registry and reports no shortcut. The enabled action can still be clicked without an assigned shortcut. Disable it to suppress command activation. Linux labels alter display conventions only and provide no Linux execution evidence. Observations arrive asynchronously; a rendered sample is not a guarantee about later focus/key delivery.

The native observer and tooltip retire with the page; no editor, asset registration or background Eio worker is created here. Adapt with a real command effect, preserve stable command/anchor identity and handle missing observations honestly. Do not infer executable permission from a tooltip’s list of assigned keys.

## Run and review

From the repository root, use the isolated repository wrapper:

```sh
./scripts/gpuio build examples/gallery/main.exe
./_build/default/examples/gallery/main.exe
```

These are instructions, not validation performed for this documentation change. This component has no standalone executable or self-test. Interactive behavior requires the native gallery; compilation alone does not establish keyboard, focus, accessibility or platform acceptance. See the [gallery README](README.md) and [development environment](../../docs/development.md).
