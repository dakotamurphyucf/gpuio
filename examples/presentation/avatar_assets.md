# Publishing valid and deliberately invalid avatar sources

[avatar_assets.ml](avatar_assets.ml) is startup support. Its record `t` carries
`image` and `invalid` scoped asset handles, not pixels or portable paths.
`load env app assets` starts one application-scoped Eio task to publish both
sources and put `Some { image; invalid }` in an external `B.Expert.Var` initially
None. It has no graph or separate executable. The owning
[main walkthrough](main.md) explains how this reactive value reaches avatars.

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/presentation/main.exe
_build/default/examples/presentation/main.exe
```

Both sources are embedded bytes: a 64×64 SVG drawing and invalid Pnm text. No
network fetch or file asset is required. The
[README](README.md) and [development guide](../../docs/development.md) describe
native launch/toolchain prerequisites; current platform limits are in the
[release policy](../../docs/platform-release-policy.md).

`Scope.start (App.scope app)` runs an Eio producer using the explicit environment
clock. A 20-second timeout bounds both registrations. Local `register format
bytes` validates `Asset.Source.of_bytes`, then `attempt` creates a promise and
uses `Scope.Expert.enqueue` to evaluate `Asset.register` on the owning UI loop.
`E.Expert.handle` runs the effect; `E.map` resolves its typed result into that
promise. The producer awaits without blocking the native OS thread. Not_ready
retries after a 5-ms Eio sleep, permitting startup negotiation; other errors
become typed diagnostics rather than infinite retries.

Success returns `Asset.handle`, and after both sources publish, the producer
calls `B.Expert.Var.set assets` directly. This producer is an Eio fiber on the
OCaml UI domain, not a worker-domain task; do not copy that mutation to another
domain. Its `on_result` effect checks `Or_error` and propagates failure.
The handles are application-scoped: no explicit early-release object is retained,
and scope end retires sources and cancels/suppresses pending task completions,
including adapter late-allocation cleanup. A handle does not extend its owner's
lifetime. See [Scope](../../lib/eio/scope.mli) and [Asset](../../lib/eio/asset.mli).

Source publication means encoded bytes are available; it does not validate image
decoding. Deliberately invalid Pnm publication succeeds so the avatar can report
`Failed Invalid_data` and display fallback. Choosing Use image makes the native
leaf acquire/decode valid source; its observation updates the local avatar state,
Bonsai derives status text, and GPUIO submits the updated view. Physical pixels
and retained editor checks are separate evidence.

This helper's Expert promise bridge is startup integration code. For component
registration prefer a lifecycle effect with scoped `Asset.register`. To load a
file, acquire bytes using explicit Eio filesystem capabilities, then publish on
the UI loop; do not read inside `let%arr` or decode in application OCaml. Retain
the registration if early release is needed and choose window scope when ownership
should end with one window. Never persist or newly bind a retired native handle.
