# Entry point and exact metadata modes

[main.ml](main.ml) dispatches command-line modes; it owns no Bonsai state or native
window. Read its single top-level match, then [Application](application.ml),
[Component](component.md) and [Ui](ui.md) for the runtime/reactive/layout layers.
After [repository setup](../../docs/development.md), run from the root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/signal_studio/main.exe -j 2
./scripts/gpuio exec dune exec examples/signal_studio/main.exe
./scripts/gpuio exec dune exec examples/signal_studio/main.exe -- --print-info-plist
./scripts/gpuio exec dune exec examples/signal_studio/main.exe -- --print-desktop-entry /absolute/path/to/gpuio-signal
```

Array.to_list exposes the executable plus arguments. Only the exact two-element
plist invocation and exact three-element desktop-entry invocation match metadata
modes. Extra arguments fall through to Application.run; this is not a general
flag parser for metadata. The application handles ordinary flags such as
--background, --self-test and --directory. The model is a local simulation with
no service credentials; ordinary launch needs no saved document.

Both metadata branches use [Application_identity.value](application_identity.md)
and [Desktop_package](../../lib/core/desktop_package.mli). macos_info_plist declares
bundle executable gpuio-signal, version 0.1.0 and build 1. linux_entry first validates
the supplied executable using File_path.of_string. Or_error.ok_exn treats invalid
configuration as a failure, not a fallback entry. Desktop_package.contents extracts
the generated text. Eio_main.run supplies an environment only to write that text
through Gpuio_eio.Output.write to its stdout capability; no GUI starts in these
matched modes. Emitting metadata neither installs handlers nor signs/packages an
app. [Distribution](../../docs/distribution.md) covers those separate operations.

A concrete ordinary trace is argv → Application.run → application-scoped resource
startup/window → Component observes snapshot → Ui describes views. A concrete
metadata trace ends at stdout without that startup. To adapt package branding,
change the shared identity and these executable/version declarations coherently;
do not confuse the Dune main.exe name with the packaged executable name.
The [README](README.md#local-checks) lists actual native harnesses and macOS/Linux
scope. These commands were inspected, not executed by this documentation review;
metadata generation alone provides no OS launch or GUI acceptance evidence.
