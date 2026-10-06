# How the preference controls work

[main.ml](main.ml) opens an “Agent preferences” form with a master checkbox,
streaming switch, notification checkbox, radio choices, dropdown and editable
combobox. These are demo preferences: choosing “Detailed reasoning” does not start
an agent or send a request. See the [README](README.md) for the exact build/run
commands. Setup uses the checkout's isolated toolchain from the
[development guide](../../docs/development.md), with stock OCaml 5.3, Core and
Bonsai v0.17. Launch requires a native desktop; no external assets are required.
The [platform policy](../../docs/platform-release-policy.md) distinguishes the
macOS-first release from required Linux builds and deferred desktop qualification.

Read `choice_id`, `modes` and `choice_appearance` first, then `component`, then the
entry point. The single-file split keeps immutable configuration outside the
reactive graph and allocates window-specific state inside it. The
aliases `B = Bonsai.Cont`, `View = Gpuio_bonsai.View` and `App = Gpuio_eio.App`
identify graph construction, effect-bearing presentation and runtime ownership.
The [Bonsai view interface](../../lib/bonsai/gpuio_bonsai.mli) gives the actual
checkbox, switch and choice callback types used here. The
[dune stanza](dune) links Core, GPUIO, the Bonsai/Eio adapters, Bonsai and `eio_main`;
`ppx_jane` and `bonsai.ppx_bonsai` enable the syntax used below.

`choice_id` validates a string with `Gpuio.Choice.Id.of_string`. `modes` constructs
two `Gpuio.Choice.t` values, then a validated `Choice.Collection.t`. IDs `fast`
and `deep` identify choices independently of their displayed labels or positions;
duplicate IDs are rejected. `Or_error.ok_exn` treats invalid hard-coded demo
configuration as a programming error. For user-provided options, handle the
validation result instead. Read the [choice contract](../../lib/core/choice.mli)
for ID, label and collection limits.

`choice_appearance` defines 40-logical-pixel rows, empty text, popup theme colors
and an option `Focused` style. Here `Focused` describes the native highlighted
option. `Style.with_state_exn` adds that state-specific styling; the style does
not implement selection behavior. Token colors such as `accent` resolve through
the window theme. Appearance validation restricts popup styles to supported
presentation properties.

`component window graph` builds a Bonsai graph: a persistent network of state and
computations. `B.toggle` returns a reactive Boolean and an effect that toggles
its current value. `enabled` and `streaming` start true, `notify` false.
`B.state (Some (choice_id "fast"))` returns a reactive `Choice.Id.t option` plus
its setter. `None` would mean no selected choice, though this demo starts selected.
State is allocated during graph construction, rather than recreated each render.

```ocaml
let combo_config =
  let%arr enabled = enabled
  and mode = mode in
  Gpuio.Combobox.Config.create
    ~label:"Search reasoning mode"
    ~options:modes
    ~selected:mode
    ~disabled:(not enabled)
    ~placeholder:"Type to filter"
    ()
  |> Or_error.ok_exn
```

`let%arr` reads the current ordinary values from reactive inputs and derives a
reactive result. `and` lists additional dependencies; it does not start threads.
`let open B.Let_syntax in` brings this reactive syntax into scope. Before the
binding, `mode` is a reactive value; inside the body it is an ordinary
`Gpuio.Choice.Id.t option`. Similarly, inside `on_select`, `set_mode` is a
function from that option to `unit Bonsai.Effect.t`, so the callback returns
work for the runtime to execute rather than changing state during configuration.
This configuration changes when the master checkbox or selected mode changes.
The separate `on_select` computation derives a callback that extracts the
`Combobox.Selection.id` and returns a setter effect. Creating that effect is not
executing it. `Gpuio_eio.Combobox.create window ~config ~on_select graph` allocates
one controller for one editable native placement. The
[controller interface](../../lib/eio/combobox.mli) explains its ownership.

The final `let%arr` reads all state, actions and the controller and returns a
`Gpuio_bonsai.View.t`. `View.column` lays out the children with validated logical
pixel padding and gaps. `Check_state.of_bool` converts the checkbox Boolean to
the widget's state type. `View.switch` and `View.checkbox` receive toggle effects;
`~disabled:(not enabled)` blocks native interaction while retaining their models.
The streaming description depends on `streaming`, so disabling preferences does
not erase that choice. `View.radio_group` and `View.select` both build
`Choice.Config.t` from the same `mode` and `modes`; both selection callbacks return
`set_mode (Some id)`.

`Combobox.view` places the controller once and shares the dropdown appearance.
Rust owns its query, caret, composition and popup interaction; Bonsai owns the
selected application ID. `Combobox.snapshot` is an optional latest native text
observation, initially absent. `Option.value_map` displays an empty string until
it exists, then `Text_input.Snapshot.text` supplies the search-query line.
Observing that text does not replace the draft or move the caret. The demo issues
no explicit editor commands and does not clear the query after a selection.

For a concrete trace, select “Detailed reasoning” in the dropdown. Native code
handles popup navigation and sends the chosen stable ID asynchronously to the
OCaml UI domain. Its `on_select` runs `set_mode (Some deep_id)`. Bonsai updates
`mode`; the dependent radio configuration, dropdown configuration and combobox
configuration now all carry that ID. GPUIO submits the derived view to native
code, where each control displays the shared selection. Transaction submission
is separate from physical frame presentation. Typing a query follows a different
path: native editor state changes, a snapshot reaches the controller, and only
the observed search-query text needs to change in the derived view.

`App.run` keeps GPUI on the OS main thread and owns one OCaml Eio UI domain for
initialization, graph computation and effects. `App.open_window` mounts this
component in a 460×600 logical-pixel window. The unused environment argument means
this form starts no application file/network/timer task and has no application
cancellation or stale-result workflow. Runtime/controller lifecycle owns native
subscriptions and window resources. The Close effect uses `App.Window.close`,
which force-closes and cancels the window scope; see the
[application contract](../../lib/eio/app.mli). For an application with unsaved
preferences and close confirmation, use `request_close` and a close handler.

To add a “Balanced” mode, extend `modes` with a new unique ID/label pair. All three
choice presentations then share it automatically. Keep the selected ID a member
of the collection when changing options. For saved preferences, store application
IDs/Booleans separately from native query and popup state, perform I/O through
explicit Eio capabilities, and apply the loaded result on the UI domain. Avoid
I/O inside `let%arr`: it is view derivation and may run repeatedly. There are only
two options here, no virtual list or resource upload, and no private diagnostics
or self-test in this application. Existing library tests can be run through
`./scripts/gpuio test`; those tests do not establish desktop interaction evidence.
