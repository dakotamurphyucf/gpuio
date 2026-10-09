# How `main.ml` owns application link intake and window routing

[README](README.md) · [Source](main.ml) · [Desktop API](../../lib/eio/desktop.mli)

Desktop Lab keeps one link receiver alive independently of its windows.
`identity ~self_test` validates identifier/name and private URL schemes.
Scheme construction alone does not register a default handler. Startup arguments
after `--open-uris` become raw link strings; all later arguments are links, so
place other flags before that marker.

`App.run_desktop ~startup_links ~exit_on_last_window:false` preflights instance
ownership. Linux secondary launches forward the batch or request reopen and
return `Forwarded` without initializing model state. macOS packaging uses Launch
Services. An unavailable session bus is not permission to start a duplicate UI.
The primary runtime owns GPUI on the OS thread and one Eio UI domain.

## Model, graph, and receiver lifetimes

`B` abbreviates `Bonsai.Cont`, `E` is `Bonsai.Effect` for deferred operations,
and `V` is `Gpuio_bonsai.View`, the view adapter accepting effects.

The status is external reactive `B.Expert.Var`; window/receiver refs hold exact
UI-domain application handles. `component` uses `let%arr` after opening
`B.Let_syntax` to read status and derive text/buttons. It has no separate
`B.state` reducer. Effects such as `E.of_thunk` defer mutations until scheduled
native event or producer completion delivery. View derivation opens no document
or filesystem resource.

`Desktop.attach ~on_event` creates the exclusive application subscription.
The intentional second attachment must return `Busy`. No link callback runs
until `Desktop.ready`. An app-scoped task calls ready after 1.5 seconds in test
mode, or after a zero-second yield ordinarily. `--self-test` also installs a
35-second shutdown deadline; it does not generate acceptance actions by itself.
FIFO delivery waits for each handler effect and yields between callbacks.

`ensure_window` reuses a live window or opens a 680 × 340 window. `App.on_reopen`
creates/activates a document window explicitly. Closing the last window does
not shut down this application; receiver and status survive until shutdown.
Exact window generations matter for delayed metadata commands.

## Link → effect → application state → view

A `document` link logs its original text, ensures a window, and sets status to
its path. Bonsai derives updated text and GPUIO submits it. The path is display
content, not a decoded filesystem authority or shell command.
A `close` route retires the current window; `replace` closes the receiver, attaches
and readies a replacement, then closes the old receiver again to check idempotent
retirement. Old callbacks cannot restart delivery into the new subscription.
`quit` calls `App.shutdown`; unknown routes only log.

Metadata routes construct a fixed represented path `/tmp/GPUIO résumé.txt` and
edited flag, then call `Desktop.set_document`. This changes native metadata,
not file contents or unsaved-work policy. The stale route intentionally uses the
retired handle and expects `Closed`; pinned Linux backends return `Unsupported`
for live document metadata. Link parsing is pure and bounded; Rejected_link and
Overflow are logged separately, while Failed raises in this fixture. See
[deep-link contract](../../lib/core/deep_link.mli).

Service routes use a fixed `--service-path=` argument, never a path extracted from
URI data. They request application activation, open/reveal through OS services,
or explicit private-scheme registration. Missing startup service path is a fixture
error. Registration can change default routing and is intentionally driven by
`--services`, not ordinary startup. Successful admission is not proof an external
application displayed/consumed a file; backend capabilities differ.

## Commands and harness requirements

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/desktop/main.exe
_build/default/examples/desktop/main.exe --open-uris gpuio-desktop-lab://document/example
_build/default/examples/desktop/main.exe --print-info-plist
_build/default/examples/desktop/main.exe --print-desktop-entry /opt/gpuio/gpuio-desktop
_build/default/examples/desktop/main.exe --check-invalid-launch
python3 scripts/test_desktop_links_macos.py
python3 scripts/test_desktop_links_macos.py --services
```

Run from the repository root in the [configured environment](../../docs/development.md).
Metadata branches match exact argv shapes, print package content via explicit
`Gpuio_eio.Output`, and open no window. Invalid-launch mode makes 12 rejected NUL
input preflights, verifies initialization never runs, and checks handle cleanup.
The ordinary link launch still needs a native graphical session and platform
instance prerequisites; it does not install a package.

The macOS harness requires the built binary and Accessibility permission. It
creates a disposable matching `.app`, exercises cold/warm Launch Services links,
readiness/routing/reopen/metadata/receiver replacement, and cleans child processes.
The services variant adds a disposable file consumer, Finder interaction, and
private default registration with cleanup. Read the owning README for the Linux
private-bus and informational graphical scripts and exact packaging prerequisites.
These are local fixtures, not signed/notarized release artifacts or
[Linux desktop qualification](../../docs/platform-release-policy.md).

For a real document app, validate routes/decoded parameters, use explicit file
capabilities in owned tasks, and reuse ordinary close/quit decisions for unsaved
work. Keep readiness after routing initialization, handle overflow/failures, and
retain receiver lifetime separately from document windows.
