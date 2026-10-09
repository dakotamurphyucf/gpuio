# How `main.ml` shares commands between a palette and controls

[README](README.md) · [Source](main.ml) · [Palette contract](../../lib/core/command_palette.mli)

The palette is a native searchable chooser over an enclosing command registry.
It does not own the Run count or draft document. Validated command IDs `run`,
`copy`, and `open-palette` distinguish commands from their displayed labels;
`Palette.Config.create` references only run/copy for chooser results.

`B.state` owns count initially 0 and `open_` initially false in the component graph.
`Text_input.create` supplies a native single-line Draft editor with autofocus.
Its text, selection, undo, and IME state are native-owned, not a Bonsai string.
An external `B.Expert.Var` controls diagnostic phase; `B.Edge.on_change` records
it through a thunk using typed integer equality.

`B` abbreviates `Bonsai.Cont` and `E` means `Bonsai.Effect`. Opening
`B.Let_syntax` enables `let%arr`, which reads state, setters, input, and phase to
derive the command registry and view. `and` binds dependencies, not concurrent
work. Setters create effects run on event delivery, not during registry creation.

Run's label includes current count; its callback sets count plus one. Phase 2
disables it. Copy is `Command.native ... Copy`: Rust routes the edit action to
an eligible document selection instead of round-tripping its text through OCaml.
Open commands has Primary+Shift+P and a callback setting `open_` true.
`View.command_scope` binds the registry, `command_button` invokes Open, and the
native palette borrows matching metadata/callbacks from that scope.

Click Open: native delivery runs the setter; Bonsai includes `command_palette`;
the runtime mounts its modal chooser and owns query/navigation/focus. Choosing
Run invokes its effect, count changes, and metadata/text update. Dismissal sends
`on_dismiss`, setting `open_` false so the application removes the chooser.
Native search uses the configured policy; no application filtering or I/O runs
in `let%arr`. A command registry is not a second draft editor.

The diagnostic deliberately forces mounting when phase is 1 or 2, independently
of `open_`. Therefore dismissing during those phases does not remove it until the
script changes phase. This is fixture control, not ordinary visibility policy.

`App.run` opens a 660 × 480 window and owns shutdown. With `--self-test`, an
app-scoped Eio task inside a 15-second timeout changes phases 0/1/2/3, waits in
5 ms intervals for Bonsai observation, and awaits a native frame promise after
each. Increasing revisions check initial view, mount, command disable, and unmount.
Completion closes the window and prints `GPUIO_PALETTE_PUBLIC_OK`. The sequence
does not type a query, invoke Run, copy text, or validate real keyboard/IME input.
Frame acknowledgement is not physical presentation or Linux desktop acceptance.

From the repository root with [development setup](../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/palette/main.exe
_build/default/examples/palette/main.exe
_build/default/examples/palette/main.exe --self-test
```

These require a graphical session/native backend. [Dune](dune) enables
`ppx_jane` and `bonsai.ppx_bonsai`. To adapt, keep ordinary visibility entirely
in application state, handle dismiss requests through effects, and let native
edit actions preserve editor ownership. Start external search in explicit owned
tasks only when needed; this example uses local native search and no network work.
