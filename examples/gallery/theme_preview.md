# Loading and reloading a window's theme

The **Styling details → A theme from your workspace** card loads a small palette
file, reports failures and keeps a native draft editor mounted while colors
change. Start with the [interface](theme_preview.mli), then
[`component`](theme_preview.ml). The [Styles page](styles_page.ml) supplies the
loader and window-owned theme selection. [Application](application.md) implements
the loader using an explicit Eio filesystem capability.

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
./scripts/gpuio exec _build/default/examples/gallery/main.exe
```

Open the card and choose a copy of [Aurora](themes/aurora.sexp). Type into
**Theme preview draft**, edit a color in the file, save it and press **Reload
file**. Loading is explicit: there is no directory watcher. The file picker is
native; current macOS evidence does not establish Linux desktop qualification.
The [existing behavior report](../../docs/evidence/gallery-theme-files-och41.md)
separates model tests from its later physical picker/reload checks.

## Model and native ownership

The draft controller is created once in the Bonsai graph. Updating its style
does not replace its initial text or construct a new editor for each palette.
Busy state, the last chosen path and status text are reactive `B.Expert.Var`s.
The applied [Theme_selection](model/theme_selection.md) comes from the window,
so it outlives this card's page-scoped loader. The local path/status and the
window's selected palette are different pieces of state.

[`Preview_scope.acquire`](preview_scope.md) creates the loader's child scope on
activation. Its cancellation hook clears busy state and reports cancellation
when needed. `let%arr` reads the current values to derive button availability,
profile/status labels and the styled editor. File operations never run inside
this view derivation.

## Follow a request

`request ~reload:false` starts only while the scope is active and not busy. It
captures the current selection's opaque token and opens a native file dialog.
Reload uses the remembered path instead. The card admits one request at a time;
it does not queue an unbounded sequence of reads.

Picker failure/cancellation preserves the current theme. A result must contain
exactly one path. Before reading, the component compares the captured token
with the current window selection. If the user chose Light/Dark or Follow system
while the picker was open, the old request is abandoned even if the colors match.

The selected path is retained, and `Scope.start` runs the supplied `load` function
in an Eio task. Its completion is delivered back as an effect. That effect checks
scope activity and the selection token again. This second fence matters because
the user can make another choice while file I/O is in progress. `Or_error.join`
combines task failure and loader failure before `Selection.complete` is called.

A successful profile updates the window model; [Component](component.md) derives
new palette values and native theme tokens. A failure leaves the last working
profile intact. Status messages use a valid UTF-8 prefix of at most 512 bytes;
invalid UTF-8 diagnostics use a generic message. This bounds displayed errors,
not the size of arbitrary application logs.

Leaving Styles cancels the task scope and prevents late publication. It does
not undo an already applied window palette. Returning acquires a new child scope.
Changing a built-in appearance clears the custom profile; changing logical size
does not. Other windows have independent selections and loader scopes.

To support another palette format, keep the public `load` result type and adapt
the file/parser layer, preserving bounds and errors. To add automatic reload,
design a scoped watcher with coalescing and the same stale-selection fences;
do not trigger file reads on every `let%arr`. This example's explicit reload and
single pending request are intentional simplifications.

Read the [pure profile decoder](model/theme_profile.md),
[selection identity](model/theme_selection.md) and
[bounded Eio adapter](files/theme_file.md) for the independent parts. None needs
an application-authored Rust callback.
