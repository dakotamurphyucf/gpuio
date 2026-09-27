# Desktop Lab

An application-scoped link receiver through `Gpuio_eio.Desktop`, with explicit
readiness and application-selected document windows. Model state survives
closing a window. The receiver is independent of window scopes.

Build and run the macOS packaged integration test:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/desktop/main.exe
python3 scripts/test_desktop_links_macos.py
```

The test creates a disposable `.app` with matching identity and URL declarations,
then invokes actual Launch Services cold and warm links. It checks malformed
input, readiness order, same-process routing, native window closure/reopening,
displayed document text, represented/edited metadata, stale-window rejection,
receiver replacement and final process exit. It requires
macOS accessibility access, briefly activates its own window, and saves the app
log to `scratch/desktop-links-macos.log` by default. It never changes the default
handler for a common scheme. Run these GUI tests sequentially.

The example scheme is `gpuio-desktop-lab`. Routes are deliberately small:
`document/name`, `close/window`, `replace/receiver`, and `quit/application`.
The `metadata/edited`, `metadata/clear`, and `metadata/stale` routes exercise
document metadata on the current or retired window, using a fixed example path.
Unknown routes have no effect. URI contents are never executed or interpreted as
filesystem paths. A real document application must validate its own routes and
reuse its close/quit decisions for unsaved work.

The packaging here is a local acceptance fixture, not a signed/notarized release
artifact. File reveal/open, runtime scheme registration and Linux incoming-link
forwarding are still being implemented under OCH-27.
