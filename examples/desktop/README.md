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
then invokes actual Launch Services cold and warm links alongside explicit
startup arguments. It first checks repeated invalid-launch cleanup without a GUI. It checks malformed
input, readiness order, same-process routing, native window closure/reopening,
displayed document text, represented/edited metadata, stale-window rejection,
receiver replacement, empty application reopen and final process exit. It requires
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
artifact. `App.run_desktop` now implements Linux session-bus instance forwarding,
with private protocol tests; packaged Linux invocation remains to be validated. Portal file
services are implemented with local protocol/worker tests; actual Linux desktop
presentation has not been validated yet.

`python3 scripts/test_desktop_links_macos.py --services` additionally builds a tiny
disposable native file consumer, opens a fixture through the OS-selected handler,
reveals it in Finder, verifies public activation from behind Finder, and checks
default routing for the private demo scheme.
It checks missing-file and undeclared/unpackaged registration errors. The test
briefly brings Finder forward, closes its fixture window and unregisters its test
bundles. Fixtures use ignored `scratch/desktop-os` because Launch Services on the
tested Mac did not discover default handlers in the system temporary directory.
`services/*` routes operate on a fixed test path supplied at startup, never a path
extracted from a URL.


`App.run_desktop` takes a validated identity and an explicit `startup_links` list.
This example collects that list after `--open-uris`; all following arguments are
raw links. Linux secondary launches return after atomically forwarding the batch,
without initializing application state. With no links, a secondary requests the
existing application's reopen handler. There is no fallback second UI if the
session bus or current owner is unavailable. See the [launch contract](../../docs/design/desktop-services.md#desktop-launch-preflight-and-linux-instance-ownership).
