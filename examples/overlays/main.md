# Controlled dialog removal versus a nonmodal popover

[main.ml](main.ml) shows Workspace editor, modal Agent settings dialog and Help
popover. Read config helper, component state/controllers, final view, then optional
diagnostic. [README](README.md) links this walkthrough; [dune](dune) declares Core/GPUIO/Bonsai/Eio
PPX dependencies; [development](../../docs/development.md) and
[platform policy](../../docs/platform-release-policy.md) cover prerequisites.
No assets or services are needed.

`B.state` owns `dialog_open` initially equal to `self_test` and `popover_open` false in a
persistent graph. Ordinary dialog starts closed; diagnostic starts open so inner
editor mounts. `Text_input.create` builds separate Agent name and Workspace
controllers with constant `B.return` config. Rust owns live drafts/caret/IME/undo.
`let%arr` reads state/setters/controllers and derives a view; `and` declares
dependencies, not threads. Setter effects are deferred, not executed by rendering.

`config` validates label/desired width/outside dismissal. Help uses key help,
width 280, anchor button and optional content; dialog key settings, width 360,
outside dismissal false. `View.dialog` is modal, `View.popover` nonmodal.
`on_dismiss` returns setter false, and only accepted application view removal
closes content; native focus policy remains until that update. Escape requests
closure; Help permits outside pointer too. Done removes dialog content. The
inner controller remains in graph but its native editor is destroyed on close;
this demo does not persist it across reopening. Outside editor remains mounted.
See [Overlay](../../lib/core/overlay.mli) and
[Text_input](../../lib/eio/text_input.mli).

Click Edit agent: native action reaches setter effect, Bonsai changes `dialog_open`,
derives `Some` panel content, GPUIO submits modal view and Rust mounts editor/focus
policy. Workspace focus is blocked until accepted close. Done reverses model/view
and retires inner editor, while external draft survives. Help open leaves Workspace
focus eligible. Admission/command replies are distinct from physical display.

`App.run` owns GPUI OS thread and one OCaml Eio UI domain, opening a 520 × 400 window. Ordinary
launch creates no I/O producer; Quit force-closes/cancels window. `--self-test`
uses mapped optional inner snapshots and B.Edge.on_change with typed equality,
starting once observation exists. `B.Clock.sleep` supplies 100 ms settling effects;
`E.Let_syntax`/`let%bind` await native commands. It replaces Unicode inner text,
checks external Focus_blocked while modal, closes and checks old inner Stale_editor,
focuses outside, opens/closes Help and confirms nonmodal outside focus. It then
marks completion and closes; `GPUIO_OVERLAYS_PUBLIC_OK` indicates success.

No test here generates real keyboard/backdrop clicks, IME, VoiceOver or GPU/physical
presentation. Timer/assertion machinery is diagnostic; application starts no file/
network/scoped producer. [App](../../lib/eio/app.mli) supplies runtime cleanup.
To retain unsaved dialog draft, store it under application ownership and seed
new mounts deliberately; do not treat graph controller retention as native
persistence. Add typed application dismissal decisions before removing content,
keep callbacks as effects and I/O outside `let%arr`.

From the repository root, build and launch with the configured toolchain:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/overlays/main.exe
_build/default/examples/overlays/main.exe
```

Run `_build/default/examples/overlays/main.exe --self-test` for the diagnostic
sequence described above. This requires a graphical session and native backend.
