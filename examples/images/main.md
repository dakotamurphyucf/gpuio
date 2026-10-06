# Scoped encoded data, native readers and retirement

[main.ml](main.ml) publishes an embedded 4 × 4 PNM, full-color SVG or monochrome icon.
Read component, source construction, register/retry and self-test. The
[README](README.md) gives exact launch variants; [dune](dune) links Core/GPUIO/
protocol/Bonsai/Eio and PPX. No filesystem/network acquisition occurs; use the
isolated [toolchain](../../docs/development.md) and current
[platform scope](../../docs/platform-release-policy.md).

`component` derives a view from external `B.Expert.Var` values: optional asset,
click count initially 0 and phase initially 0. A Bonsai graph persists; `let%arr`
reads ordinary current values, with `and` declaring dependencies, not threads.
`B.Edge.on_change` records phase via typed equality and deferred thunk for tests.
No per-image decode or application I/O runs inside derivation.

Without a handle, the view displays registering text. With one, description
Generated preview is explicitly meaningful for accessibility. Phase 0 uses Contain,
others Cover. Key preview remains stable through phase 1; phase 2 changes to
remounted deliberately. Style is 192 × 192 logical pixels and blue foreground;
source metadata is four **pixels**, a different unit. `View.image` uses pure
`Image.Config`; `View.icon` uses validated `Icon.Config` and monochrome foreground.
See [Image](../../lib/core/image.mli) and [Icon](../../lib/core/icon.mli).

Icon mode also builds decoration metadata for leading/trailing Send, labelled
icon-only button and a command button. `invoke` returns a thunk incrementing the
reactive click value; registry label derives from it. Phase 1 removes one trailing
decoration while retaining other native readers. These callbacks are events,
not decode work. `on_change` records Loading/Ready/Failed state asynchronously.
Native source decoding/rasterization/layout/pixels stay in Rust.

`App.run` owns GPUI on OS thread and OCaml Eio UI domain, opening a 480 × 300 window.
An application-scoped Eio producer with explicit clock/20-second timeout constructs
encoded source and publishes via `Asset.register`. Local `on_ui` enqueues owning-
UI-domain effect through Expert enqueue/handle and bridges typed result with
`Eio.Promise`. Not_ready retries after 5 ms; other errors fail diagnostic.
Successful source publication sets asset handle, triggering view creation, then
waits for native Ready with 4 × 4 metadata. Encoded publication does not prove decoding.
The producer is an Eio fiber on UI domain, not a worker mutating Bonsai.

Click an icon button: native button event reaches OCaml effect, thunk changes
count, Bonsai derives updated command label and GPUIO submits native view. Source
and readers remain independent of this count. Ordinary launch retains registration
until application scope ends; borrowed handle does not extend lifetime.

`--self-test` releases registration after Ready, changes phase 1 and awaits native
render callback through promise, requiring existing reader still Ready. Phase 2
changes key and requires Failed Released for a fresh reader; old readers retain
their leases until removed. Success prints `GPUIO_IMAGES_PUBLIC_OK` with SVG/icon
flags then force-closes. `--svg`/`--icon` combine with this test; icon implies SVG.
It checks source/reader lifetime and FFI events, not real clicks, GPU pixels,
VoiceOver or Linux GUI acceptance. [Asset](../../lib/eio/asset.mli),
[Scope](../../lib/eio/scope.mli) and [App](../../lib/eio/app.mli) define cleanup.

To load external images, acquire bytes with explicit Eio capabilities, validate
source and register in intended lifetime scope. Store registration if early release
is needed and remove decorations rather than making new bindings to a retired
handle. Preserve stable view key for restyling, choose a new key deliberately for
new reader identity, and handle Image.State failure separately from publication.
Never perform I/O in `let%arr` or treat Ready as physical presentation evidence.

From the repository root, build and launch with the configured toolchain:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/images/main.exe
_build/default/examples/images/main.exe
```

Run `_build/default/examples/images/main.exe --self-test` for the diagnostic
sequence described above. This requires a graphical session and native backend.
