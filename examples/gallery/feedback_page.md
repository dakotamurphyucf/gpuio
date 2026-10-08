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
their registry across buttons, drawn menus, native context menu, native menu bar and chooser. Advance uses
Primary+K and the model’s enabled flag; Notify injects a mock save notification; Choose opens
the palette; Copy is an explicit native Copy action. Native Advance activation runs its injected
action, updates the stage in the latest model, and `let%arr` derives
`State.Stage.label` and progress for native reconciliation: “Ready to begin”,
“Finding the right pieces”, “Halfway there” and “Everything is in place”. It does not start real work. Menu availability
therefore comes from the same registry rather than independent button booleans.

`register_menu_icon` acquires decorative SVG artwork through [Preview_scope](preview_scope.md).
Pending/failure omits artwork while leaving menus usable. Drawn menu content uses item paths,
while platform context menus and the menu bar receive icons through their supported API. The scope owns
registration; borrowed icons are not independent owners. Separate native editors hold the
command draft and palette header note.

The platform context popup uses the original `menu`, including section labels.
Its icon paths are `[0; 1]` for Advance, `[0; 4]` for Editing and `[0; 4; 1]`
for Copy inside Editing. Labels and separators count toward these positions;
the nested Selection actions label is child zero. The Copy decoration is passive:
Copy still uses native edit availability and is disabled without an eligible
editing target. AppKit dims disabled artwork along with its text.

`native_bar` builds separate application and Workspace menus. AppKit presents
the first top-level menu under the application's name. Workspace's Preview actions
submenu repeats Advance and Choose from the same command registry. `V.menu_bar
~platform:true` installs the active window's application menu on macOS; Linux
uses the drawn fallback. `V.with_menu_item_icons` attaches passive SVGs by item
path: `[1; 0]` is Workspace's first command, `[1; 3]` its submenu, and
`[1; 3; 0]` the submenu's first command. Separators count toward positions.
Paths identify occurrences, so two Advance rows can have independent artwork.
The Detailed menu items toggle clears or restores these decorations without changing
command behavior. The SVG supplies the silhouette; AppKit chooses its template
tint. Top-level menu titles remain text.

For example, choosing Workspace → Preview actions → Advance dispatches the
existing `advance` command, injects its Bonsai state-machine action and derives
the next stage/progress through `let%arr`. Rust owns menu tracking, pixels and
native event delivery; the example needs no Rust code. Pending artwork does not
disable the command. Leaving Feedback removes its bar with the page's native
owners, and another active window supplies its own menu.

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
a batch of typed sample identities; it substitutes these for the ordinary notification view. The “Notification
visible” text still reports State.notification, so it does not describe whether a substituted
layered card is painted. Placement cycles eight anchors; insets, layering and motion are native
presentation policies, not toast identity resets.

For a concrete layered interaction, `sample_toasts` begins with `Samples.initial`: sample numbers
1, 2 and 3 in batch zero. `Samples.items` supplies the values mapped into `V.toast`;
`Samples.Item.key` includes both batch and number, and each dismissal effect
captures the complete item identity. Tab from the native Notifications group reaches the oldest
source card's close button; Escape accepts that card's dismissal. With motion
enabled, native input eligibility ends before the exit finishes. Rust then sends
the terminal observation, `on_dismiss` injects `Dismiss item`, the Bonsai reducer
filters that exact batch/item identity, and the next `let%arr` omits its view. Show three sample
notifications injects `Show`, which starts a fresh batch with three new keys.
This also works during an exit: its old callback cannot remove a restored item.
The pure reducer and its race test are explained in [feedback_state.md](model/feedback_state.md). Layered paint places the newest card at the anchor;
that visual stacking does not reverse the source-order keyboard traversal.

Changing `toast_anchor` updates `Toast.Placement`; the `toast_config` passed to
`V.toast_stack` changes while the child keys remain stable. This is why moving
notifications among anchors or changing margins does not reconstruct their
models. `Toast.Stack.Layering.default` supplies measured expansion geometry and
`Toast.Stack.Motion.default` supplies native entry/exit and reflow. Neither option
adds an OCaml animation loop. While collapsed, older painted cards are decorative
and hidden from accessibility traversal; focusing the Notifications group expands
them for interaction.

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
python3 scripts/test_gallery.py --section notifications --images scratch/notification-walkthrough
```

The wrapper uses the repository toolchain. These commands are instructions, not checks run for
this documentation change. The last command runs the native notification fixture
against the built gallery; it opens and closes an actual macOS window and requires
Accessibility access. The page has no standalone executable. Compilation does
not establish native keyboard, focus, IME or platform acceptance. See
[gallery instructions](README.md) and [development](../../docs/development.md).
