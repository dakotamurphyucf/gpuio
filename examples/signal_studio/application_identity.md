# One identity for desktop startup and package metadata

[application_identity.ml](application_identity.ml) and
[application_identity.mli](application_identity.mli) construct Signal Studio's
immutable desktop identity once: identifier com.gpuio.signal-studio, display
name GPUIO Signal Studio and the pure model's gpuio-signal scheme. This is pure
validated configuration, not OS handler installation, native resource ownership,
Bonsai state or Eio I/O.

From the repository root after [isolated setup](../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/signal_studio/main.exe -j 2
./scripts/gpuio exec dune exec examples/signal_studio/main.exe
```

The ordinary workbench is a local simulation with no credential/service. macOS
is the v1 desktop target; Linux nongraphical and
[informational GUI](../../docs/platform-release-policy.md) checks are separate.

Read `value`, then its two callers [main.ml](main.ml) and
[application.ml](application.ml). `Gpuio_eio.Desktop.Identity` exposes the public
[Desktop.Identity](../../lib/core/desktop.mli) pure constructor: a validated
reverse-DNS lowercase identifier, nonblank name and unique normalized schemes.
Or_error.ok_exn treats these checked-in values as startup invariants. The scheme
comes from [Workspace.scheme](model/workspace.md), avoiding separate drifting
URL declarations in package/runtime code.

Application calls `App.run_desktop Application_identity.value`, configuring the
process identity before desktop requests and attaching the incoming-link receiver.
Startup links/native events are held until Desktop.ready after model/resources/
window readiness; a first chart paint is not required for an occluded app to
receive the activating link. Workspace.route then accepts only known sample
links and selects a sample. Declaring a scheme alone does not register/reassign
an OS handler or grant filesystem capabilities; runtime readiness and packaged
metadata have separate responsibilities.

Main's metadata modes reuse the same identity:

```sh
./scripts/gpuio exec dune exec examples/signal_studio/main.exe -- --print-info-plist
./scripts/gpuio exec dune exec examples/signal_studio/main.exe -- --print-desktop-entry /absolute/path/to/gpuio-signal
```

Use an actual intended installed executable path for the desktop entry. The
macOS plist uses bundle executable gpuio-signal/version0.1.0/build1, while the
Dune development executable remains main.exe. These modes emit text without
opening a workspace, creating a bundle, signing it or installing URL handlers.
The [package interface](../../lib/core/desktop_package.mli) and
[distribution guide](../../docs/distribution.md) explain that wider workflow.

For a concrete route trace, a packaged gpuio-signal://sample/2 event reaches the
ready desktop receiver, Workspace.route selects Sage and the application ensures/
activates a window plus a native selection command. It never uses /2 as a file
path; [Documents](documents.md) obtains file destinations through a separate
picker/explicit capability. Successful metadata construction/native activation
submission is not evidence that an OS cold/warm link launch passed.

A small adaptation is renaming a forked demo: change identifier/name and packaging
configuration together, preserving matching scheme declarations and validators.
A new route needs a pure Workspace.route contract before an OS scheme declaration.
The [README](README.md) links the optional desktop/notification walkthroughs and
actual evidence; this source review performs no OS registration or launch test.
