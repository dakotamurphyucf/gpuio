# How `main.ml` turns pointer samples into panel width

[README](README.md) · [Source](main.ml) · [Pointer contract](../../lib/core/pointer.mli)

This example combines a raw drag region with ordinary Narrower/Wider buttons.
The buttons provide alternatives to a gesture primitive that does not invent
keyboard slider semantics.

## Application state and Bonsai syntax

`Action.t` is `Set of float` or `Adjust of float`. `B.state_machine0` creates a
reactive width model, initially 180 logical pixels, and an `inject` effect
constructor. Its reducer replaces or adds to the width, then clamps it to the
80–360 range with `Float.clamp_exn`. All input paths therefore share the same
application invariant.

`B` abbreviates `Bonsai.Cont`, `E` abbreviates `Bonsai.Effect`, and `graph` owns
the state computation. An external `B.Expert.Var` supplies the diagnostic phase.
`B.Edge.on_change` observes it using `Int.equal`; its callback returns a thunk
that records the phase after effect delivery. That reference is test bookkeeping,
not the width model.

Opening `B.Let_syntax` enables `let%arr`. The bindings read current width,
`inject`, and phase, then derive a new view when dependencies change. `and`
collects reactive dependencies; it does not start concurrent operations. Calling
`inject` builds an effect attached to an event, rather than changing width during
view construction.

## Event, effect, model, view, native region

`Pointer.Config.create` validates the accessible label and disables the region
only in phase 1. The default initiating button is Left; native default prevention
and propagation policy run synchronously in Rust. `View.pointer_area` has stable
key `width-handle`, a 360 × 48 logical-pixel surface, and a `Pressed` foreground
style resolved by the native runtime.

A native initiating press captures the gesture. Its `Started` sample reaches
`on_event`, which injects `Set event.local_position.x`. `Moved` and `Released`
use the same path. The reducer clamps the coordinate; Bonsai rebuilds the width
label and content style; GPUIO submits the updated native view. Coordinates are
relative to the region bounds at that sample and may lie outside the region.
Consecutive movement samples may coalesce, so this is a position-based update,
not a reliable count of every physical motion sample.

`Cancelled _` returns `E.Ignore`: this example keeps the last accepted width.
Native cancellation can follow Escape, disabling, removal, focus loss, or other
capture changes. A transactional editor could retain an initial width and restore
it on cancellation. Phase 2 omits the handle entirely; the content model remains.

## Runtime and diagnostic limits

`App.run` owns GPUI on the OS thread and an OCaml Eio UI domain. It opens a
640 × 360 window. In ordinary mode, users supply the events. With `--self-test`,
`Scope.start (App.scope app)` runs a producer inside a 15-second timeout. Its
`render` helper updates phase, waits in 5 ms intervals for Bonsai observation,
and awaits an Eio promise resolved by the `App.Window.request_frame` effect.
It checks increasing revisions for mount, disable, enable, and keyed removal,
then closes the window and prints `GPUIO_POINTER_PUBLIC_OK`.

This sequence generates no drag samples. It checks native region/view lifecycle
and rendering acknowledgements, not capture delivery, real pointer gestures,
keyboard access, accessibility, or physical screen presentation. Frame delivery
can be deferred for occluded windows. Diagnostic orchestration and application
state run on the owning UI domain, not from an arbitrary worker thread.

## Commands and adaptation

From the repository root in the [repository environment](../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/pointer/main.exe
_build/default/examples/pointer/main.exe
_build/default/examples/pointer/main.exe --self-test
```

These launches require a graphical session/native backend. [Dune](dune) enables
`ppx_jane` and `bonsai.ppx_bonsai`. To resize from a starting width instead of
using an absolute coordinate, store gesture-start width and position in the
model and calculate deltas. Keep keys stable, handle cancellation deliberately,
and offer ordinary controls for equivalent actions.
