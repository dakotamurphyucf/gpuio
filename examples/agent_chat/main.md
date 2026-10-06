# CLI and macOS package metadata

[main.ml](main.ml) is a small executable entry point. It parses demo switches,
then calls [Application.run](application.md); it has no Bonsai state or GPUIO
view construction. Read `main`, the final `let ()`, then
[application.mli](application.mli) to see the handoff.

After the [repository-local setup](../../docs/development.md), run from the root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/agent_chat/main.exe -j 2
_build/default/examples/agent_chat/main.exe
```

The ordinary launch uses synthetic responses, bundled icon/image data and
in-memory conversations. No credentials or network connection are needed.
macOS is the v1 desktop target; Linux builds and graphical coverage have
[separate requirements](../../docs/platform-release-policy.md).

## Parse options and launch

`flag value` uses `Array.exists (Sys.get_argv ()) ~f:(String.equal value)` to check
for an exact switch. `~f` labels the predicate argument in Core's array API;
`String.equal` compares string values without polymorphic comparison. This is
a deliberately small parser, not a full CLI framework: it does not reject every
unknown option or offer a help command.

| Option | What `main` passes to the application |
| --- | --- |
| `--directory /absolute/path` | A validated `Gpuio.File_path.t` starting-folder hint for the attachment picker |
| `--reduced-motion` | `Gpuio.Animation.Preference.Reduce` |
| `--full-motion` | `Gpuio.Animation.Preference.Full` |
| Neither motion switch | `Gpuio.Animation.Preference.System` |
| `--self-test` | Enable the controller-driven native acceptance runner |
| `--native-test` | Lengthen send acceptance and print a shutdown marker for external scripts |
| `--workload-metrics` | Enable the long stream fixture and diagnostic stdout sampling |

`Array.findi` finds the first `--directory`, including its index, so the next
argument can be passed to `Gpuio.File_path.of_string`. A missing argument raises
the explicit diagnostic; an invalid path is rejected through `Or_error.ok_exn`.
This is startup validation, not asynchronous picker failure recovery. Motion
options use an exhaustive match over the pair of booleans and reject supplying
both. The setting applies to the app, without changing system preferences.

Try these distinct modes:

```sh
_build/default/examples/agent_chat/main.exe --reduced-motion
_build/default/examples/agent_chat/main.exe --directory /tmp
_build/default/examples/agent_chat/main.exe --self-test
```

The self-test opens real windows; it is optional and does not simulate external
OS typing. The [README](README.md#validation) distinguishes it from the macOS
Accessibility scripts. This prose review is not a new GUI validation result.

## The metadata branch

The final `let ()` checks `--print-info-plist` before calling `main`. That branch
constructs `Gpuio.Desktop.Identity` with identifier `com.gpuio.agent-chat`, name
`GPUIO Agent Workspace` and no URL schemes. It asks
`Gpuio.Desktop_package.macos_info_plist` for version `0.1.0`, build `1` and bundle
executable `gpuio-agent-chat`, then prints `Desktop_package.contents` using
`Eio_main.run` and the local `output` helper. It exits without opening the GUI:

```sh
_build/default/examples/agent_chat/main.exe --print-info-plist
```

The bundle executable name describes a packaged app, while the development Dune
executable is `main.exe`. This option emits text only; it does not make a bundle,
sign it, install it or register a URL handler. If supplied alongside ordinary
switches, the metadata branch takes precedence. See the
[desktop identity interface](../../lib/core/desktop.mli),
[package interface](../../lib/core/desktop_package.mli) and
[distribution guide](../../docs/distribution.md) for the wider contract.

A small adaptation is to rename the application for a separate demo. Change the
identity name/identifier and packaged executable together with the actual
packaging configuration; changing the plist text does not rename the built
binary. For a new runtime flag, parse it in `main` and add a typed parameter to
`Application.run` and its interface, keeping native resource creation in the
application rather than this CLI module. Update the option table and owning
README whenever a switch changes.
