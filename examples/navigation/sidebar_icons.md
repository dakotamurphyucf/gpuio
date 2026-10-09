# Publishing the sidebar's in-memory SVG

[sidebar_icons.ml](sidebar_icons.ml) contains one support function, `load env app
icon`. The [lab entry point](main.md) invokes it once with the runtime environment,
app and a `B.Expert.Var` initially holding `None`. The function publishes an
encoded folder SVG and later sets that variable to `Some Icon.Decoration.t`.
The component reads it reactively and decorates Archive; it does not paint or
decode SVG in OCaml. Read the source from `Scope.start` through `attempt` and
`on_result`. This support module has no separate executable, interface or CLI;
[dune](dune) includes it in `main.exe`.

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/navigation/main.exe
_build/default/examples/navigation/main.exe
```

The SVG bytes are embedded, so no file path, network service or external asset
is needed. Toolchain and platform prerequisites are in the
[development guide](../../docs/development.md) and
[platform policy](../../docs/platform-release-policy.md). The whole-lab self-test
waits for icon publication; no independent icon self-test exists.

`Scope.start (App.scope app)` runs a producer Eio fiber in application lifetime.
The explicit environment supplies `Eio.Stdenv.clock`; `Eio.Time.with_timeout_exn`
bounds the entire load/retry process to 20 seconds. `Asset.Source.of_bytes
~format:Svg` validates an immutable encoded source, not decoded pixels. It is a
24×24 folder path in white; choosing presentation belongs to the sidebar.
`Or_error.ok_exn` treats invalid fixed demo data as a programming failure.

`attempt` creates a fresh `Eio.Promise` for each registration request. It uses
`Scope.Expert.enqueue` to schedule an owning-UI-domain job, where
`E.Expert.handle` evaluates `Asset.register app ~scope source`. `E.map` turns the
registration result into promise resolution. Constructing a Bonsai effect is
separate from running it; `handle` here is low-level runtime integration used
inside this example helper. The producer waits on the promise without blocking
the native OS thread. No Bonsai graph/state is created in this module.

`Not_ready` means app negotiation has not completed. The producer sleeps 5 ms
and retries within the timeout; it does not blindly retry arbitrary errors.
Other typed registration errors are raised with sexp diagnostics and become the
scope producer's `Or_error` result. On success `Icon.Decoration.create
~asset:(Asset.handle asset)` builds validated metadata. The scope's `on_result`
returns `E.of_thunk`, delivered on the UI loop, to set the external reactive
variable. Thus a successful publication leads to `icon` changing, `main.ml`'s
`let%arr` deriving new `Sidebar.Decoration` metadata, and GPUIO submitting the
updated native view. Encoded publication is not decoding, native-ready pixels
or physical presentation.

Application scope owns the registration. The helper does not retain an explicit
asset object for early release; its handle does not extend registration lifetime.
Scope end cancels the task, suppresses queued user completion and retires assets,
including late allocation cleanup handled by the asset adapter. Hiding/collapsing
the sidebar does not cancel this application-scoped work. External Eio
cancellation stays cancellation; it is not converted into a retry. Read
[Scope](../../lib/eio/scope.mli), [Asset](../../lib/eio/asset.mli) and
[Icon](../../lib/core/icon.mli) for ownership and limits.

This is a small startup integration example using Expert enqueue/effect APIs;
it is not a general worker-domain pattern. It mutates Bonsai only on the UI loop.
For an ordinary component, use a lifecycle effect and scoped `Asset.register`
instead of copying the explicit promise bridge. To load a file, acquire bytes
through an explicit Eio filesystem capability in producer work and then construct
a source; keep decoding native and never perform file I/O in `let%arr`. Choose a
window scope if the icon should retire with one window, and retain the registration
if you need deliberate early `Asset.release`. Do not persist the native handle or
bind a released registration as though it were durable identity.
