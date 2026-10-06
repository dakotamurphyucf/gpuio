# How `main.ml` checks canvas publication and resource leases

[README](README.md) · [Source](main.ml) · [Canvas adapter](../../lib/eio/canvas.mli)

This finite native integration check opens no window and creates no Bonsai view.
It registers scenes, exercises protocol failures, and verifies cleanup. Scene
publication and image leasing are separate from canvas layout, hit-testing, paint,
input, or graphical acceptance.

`Wire` is the internal Canvas protocol; `Scene`/`Geometry` are wire schemas;
`Canvas` is the public scoped adapter; `Typed_scene`/`Resource` are validated Core
values. `Effect` means `Bonsai.Effect`. `App.run ~exit_on_last_window:false` owns
the backend/Eio UI domain. An app-scoped producer has a 60-second timeout.
`on_ui` schedules effects through `Scope.Expert.enqueue` and resolves promises.
`ui` wraps synchronous ownership mutations in a thunk. No `let%arr`, reactive
model, or event-driven view is present.

`scene ()` constructs 20,000 raw rectangle items whose bin_prot encoding exceeds
one message. After readiness retry, Create supplies an ID; Begin names accepted
base, next revision, resource generation, and encoded byte count. Ordered Chunk
calls supply bounded slices. Publishing only the first slice must return
Incomplete; finishing all slices publishes revision 1. A stale base is rejected.
A malformed successor returns Invalid_scene; Abort clears it without changing
accepted revision. An empty scene then publishes revision 2 in generation 2.
Release and slot reuse check stale resource IDs.

`typed_text` creates validated reusable text resource/item IDs, origin, color,
and scene description. Public `Canvas.create` publishes that scene. Replacing
text under the same resource generation via `Canvas.set` must report
`Native Stale_resource`, keeping the registration live. `Canvas.reset` starts a
new scene resource generation and recovers while keeping its handle stable.
One thousand requested resets coalesce to one publication; a raw Begin/Abort
probe checks exact accepted revision/generation. See
[resource contract](../../lib/core/canvas_resource.mli).

A child scope owns a valid one-pixel PNM asset and an image scene borrowing it.
Publishing the scene acquires a native asset lease. Releasing the asset retires
new bindings, while the existing canvas can republish its retained image lease.
A second raw asset Release is an acknowledgement barrier, not a stale-ID probe.
Creating a new canvas from that retired image must return Unavailable_image.
Cancelling the child releases its canvas as well.

The remaining stages cycle 270 registrations/releases, then occupy 63 raw lanes
while a scoped create progresses through its reserved lane. The final callback
checks zero commits/rendered frames, issues eight pending creates, shuts down,
and issues one more. All nine complete Closed exactly as asserted. Success prints
`GPUIO_CANVAS_UPLOAD_OK` with stage logs useful for failure location.

From the repository root using [development setup](../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/canvas_upload/main.exe
_build/default/examples/canvas_upload/main.exe
```

Every launch runs this check; it has no ordinary UI or self-test flag. The native
backend still starts, so windowless does not imply arbitrary headless support.
There are no external files/network credentials. Build/native-worker success
cannot establish rendered canvas or Linux GUI acceptance.

For an application, use scoped typed scenes, bump resource generations when
changing reusable content or reset deliberately, and handle asynchronous adapter
errors. Keep borrowed asset and scene lifetimes explicit. Raw protocol IDs,
manual revision probes, and lane saturation belong to this diagnostic, not
production rendering logic. See [canvas UI](../canvas/main.md).
