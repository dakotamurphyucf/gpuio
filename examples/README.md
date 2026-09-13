# Examples

`foundation/` is the bootstrap Bonsai/Eio/GPUI window and input example. Run it with
`./scripts/gpuio smoke`; `--self-test` exercises the bridge/lifecycle/input handlers
and cleanup, while `--two-windows` exercises native window identity and independent
editor lifetime. These tests open actual windows, but input probes invoke native
handlers; they do not prove real OS IME or accessibility behavior.

The bootstrap protocol and single-window application host are private to this
example. The production per-window Bonsai/Eio API is demonstrated in `runtime/`.
Agent chat and graphics examples grow with subsequent milestones.

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
