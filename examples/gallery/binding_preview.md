# Live binding preview walkthrough

Read [binding_preview.ml](binding_preview.ml) and its [interface](binding_preview.mli). [pages.ml](pages.ml) mounts `component window palette graph` in **Presentation**.

`Mode.t` cycles `Focused → Editor → Here → Native`. `B.state_machine0` implements that cycle and a separate invocation counter. Toggles control observer visibility, command registration/enabling, alternate shortcut, malformed native facts and display platform. `Editor.create` allocates an independently retained single-line native editor through the Eio/Bonsai adapter. Its initial draft is supplied once; changing query configuration does not recreate the editor.

Bonsai state is reactive: changing its model makes dependent computations see the new value. Each `state_machine0` returns the current model plus an action injector returning an effect. The reducer receives the latest model when that effect executes; creating `next_mode ()` does not advance the mode immediately. For example, a native click on **Binding context** runs the injected unit action, `Mode.next` changes Focused to Editor, and the configuration `let%arr` derives an editor-lease query. The mounted adapter updates native sampling; its later observation then derives the displayed binding rows.

The first `let%arr` derives a [`Command_binding.Config`](../../lib/core/command_binding.mli). Focused inspects current focus; Editor captures the exact native editor lease from its snapshot, waiting if none exists. Here queries only the local command; Native queries only native Copy using `Input` or deliberately malformed `Input !` facts. Configuration requires 1–64 unique targets and compatible context/target combinations. Hypothetical contexts do not move focus.

`match%sub` selects reactive branches: hidden removes the observer, absent editor configuration shows a waiting message, and a ready configuration mounts `Gpuio_bonsai.Command_binding.component` with key `live-query`. Its callback reads observations with `let%arr`. `observed` distinguishes Ready, Suspended, Context_gone, Invalid_context, Epoch_exhausted and Capacity. `entry` distinguishes registry commands, native bindings, unsupported native sequences and missing targets, translating each candidate disposition into text. Capacity is not evidence of an unbound action. The displayed epoch is monotonic within one mounted configuration, not a promise about future key delivery.

A later `let%arr` constructs command `gallery.live-bindings` with Primary+K or Primary+L and Override priority. `V.command_scope` registers it only when requested; observation itself does not register shortcuts. `Presentation.Kbd.create` renders registry shortcuts, while `of_native_stroke` renders native display data that may not be valid registration input. Linux labels change formatting only, not the running backend.

Focus the draft, inspect Copy, then cycle to Editor and change the shortcut. Unregister the command to see Missing_command; disable it to see its registry status. Hide/show the observer: the editor remains mounted and retains text while the query retires/remounts. In Native mode, invalid facts demonstrate native parsing failure. “Run preview action” directly increments the counter even when registry invocation is disabled; it is a separate ordinary button in the current source.

Native editor state and observer sampling belong to GPUIO; application state/effects belong to Bonsai. The adapter retires leases/observers on branch removal; this file creates no asset scope or background worker. To adapt it, retain an editor outside query branches, handle every observation state, and use availability observations as diagnostics rather than permission to assume a later shortcut will execute.

## Run and review

From the repository root, use the isolated repository wrapper:

```sh
./scripts/gpuio build examples/gallery/main.exe
./_build/default/examples/gallery/main.exe
```

These are instructions, not validation performed for this documentation change. This component has no standalone executable or self-test. Interactive behavior requires the native gallery; compilation alone does not establish keyboard, focus, accessibility or platform acceptance. See the [gallery README](README.md) and [development environment](../../docs/development.md).
