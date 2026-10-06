# Examples

Start with [getting_started](getting_started/README.md): one small counter with
explicit GPUIO view, Bonsai state and application startup functions. Then use the
[gallery](gallery/README.md#reading-the-code) for component recipes,
[Agent Workspace](agent_chat/README.md#implementation-map) for a larger chat app,
and [Signal Studio](signal_studio/README.md#reading-the-code) for graphics and
desktop integration. Their entry points link to modules by responsibility;
optional acceptance runners are separate from normal app code.

The examples below include historical bootstrap and low-level integration checks.
They are useful for framework work, but are not the recommended first-app template.

`foundation/` is the bootstrap Bonsai/Eio/GPUI window and input example. Run it with
`./scripts/gpuio smoke`; `--self-test` exercises the bridge/lifecycle/input handlers
and cleanup, while `--two-windows` exercises native window identity and independent
editor lifetime. These tests open actual windows, but input probes invoke native
handlers; they do not prove real OS IME or accessibility behavior.

The bootstrap protocol and single-window application host are private to this
example. The production per-window Bonsai/Eio API is demonstrated in `runtime/`.
The integrated `agent_chat/` workspace is documented in its [README](agent_chat/README.md). Signal Studio is the current graphics reference.

`view_api/` demonstrates the public typed view/style/theme vocabulary, reusable
components, a compiled Bonsai.Cont component and an explicit Eio bridge runner.
See its [README](view_api/README.md) for interactive and automated commands.
`bridge/` separately exercises production transaction rollback and ownership.

`text_input/` demonstrates a single-line input and multiline composer, stable
Bonsai controllers, submit observations and explicit native commands. See its
[README](text_input/README.md). Local foreground GUI checks are authorized for fast iteration; avoid activation
where a test permits background execution. CI supplies the final platform gates.

- `combobox`: editable choices with native query ownership and a public controller
  self-test for guarded replacement, undo and stale unmount. See its README.

- [`presentation`](presentation/README.md): public stateless presentation helpers,
  settings/form semantics and reusable chat cards, with light/dark appearances
  and native keyboard/accessibility checks. OCH-33's stateful families remain pending.

- [Positioned menus](menu_controller/README.md): open/close a context menu from
  an OCaml controller, with validated coordinates and stale-definition rejection.
  [Component walkthrough](menu_controller/component.md) separates state, views
  and asynchronous commands from the small window launcher.
- [Menus](menus/README.md): shared command registries, Bonsai effects, dropdowns
  and optional native OS context popups. The adjacent [walkthrough](menus/main.md)
  traces the complete interaction and explains the optional self-test.
