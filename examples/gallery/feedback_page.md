# Feedback page implementation

Read [feedback_page.ml](feedback_page.ml) and its [interface](feedback_page.mli).
[pages.ml](pages.ml) routes Feedback to `component ~search_palette app window palette graph`.
`search_palette` is a caller-supplied function used by the
[external palette](external_palette_preview.md); it is not a network capability implicitly
created here.

`B` is `Bonsai.Cont`, `V` `Gpuio_bonsai.View`, `State` the
[pure feedback model](model/feedback_state.md), and `Palette_controller` the Eio command-palette
adapter. `graph` hosts reactive models/controllers; `let%arr` reads their current contents to
derive UI. A state machine owns `State.initial`, returning current model plus action-effect
injector that reduces against latest model when executed. Ordinary state/toggles own chooser
visibility, palette controls, toast placement/layering/motion and artwork display. Constructing
callbacks/effects does not invoke a command.

`advance`, `notify`, `choose` and `copy` are validated command IDs. One `V.command_scope` shares
their registry across buttons, drawn menus, native context menu and chooser. Advance uses
Primary+K and the model’s enabled flag; Notify injects a mock save notification; Choose opens
the palette; Copy is an explicit native Copy action. Native Advance activation runs its injected
action, updates stage in the latest model, and `let%arr` derives Ready/Working/Halfway/Complete
text/progress for native reconciliation. It does not start real work. Menu availability
therefore comes from the same registry rather than independent button booleans.

`register_menu_icon` acquires decorative SVG artwork through [Preview_scope](preview_scope.md).
Pending/failure omits artwork while leaving menus usable. Drawn menu content uses item paths,
while platform context menus receive icons through their supported API. The scope owns
registration; borrowed icons are not independent owners. Separate native editors hold the
command draft and palette header note.

`Palette_controller.create` supplies stable query identity, observations and asynchronous
commands. Search mode cycles All_terms/Substring/Unfiltered; keywords such as store/persist map
to Save. Rich palette header/footer/empty/items contain independent controls and passive
command-row decoration. `palette_command` awaits Set_query/Highlight/Focus/Set_loading replies
with effect `let%bind`, then sets status for reactive derivation. Loading here is a native
palette flag, not a worker. Dismiss resets controller observation/status and chooser state;
mounting/configuration, observation and command completion are distinct stages. See
[embedded palette](embedded_palette_preview.md) for the separately mounted inline example.

Notifications use stable serial-derived keys from State. A new save replaces the previous timed
five-second toast; `on_dismiss` injects its captured serial so an older dismissal cannot remove
a newer notification. Layered mode instead displays three persistent mock cards controlled by
their ID list; it substitutes these for the ordinary notification view. The “Notification
visible” text still reports State.notification, so it does not describe whether a substituted
layered card is painted. Placement cycles eight anchors; insets, layering and motion are native
presentation policies, not toast identity resets.

Deactivation injects Leave and closes the chooser. Asset scope and native owners retire
separately; native toast timing/hover/focus pause remain in GPUIO, with no OCaml frame timer.
Page notes are native drafts, not durable storage. Adapt with real command effects and explicit
result states, serial-safe notification reducers and scoped search capabilities. This
demonstration confirmation saves no files; rich decorations do not change registry semantics or
prove platform acceptance.

## Run and review

From the repository root:

```sh
./scripts/gpuio build examples/gallery/main.exe
./_build/default/examples/gallery/main.exe
```

The wrapper uses the repository toolchain. These commands are instructions, not checks run for
this documentation change. The page has no standalone executable or self-test. Compilation does
not establish native keyboard, focus, IME or platform acceptance. See
[gallery instructions](README.md) and [development](../../docs/development.md).
