# Retain one scoped canvas while native views come and go

[diagram.ml](diagram.ml) and [diagram.mli](diagram.mli) turn the
[pure run geometry](run_diagram.md) into a window-owned GPUIO canvas. Native input
handles intermediate drag/pan/zoom frames; queued observations update the model.
The application retains one lazily registered scene across inspector/page hides,
while native canvas widgets may unmount.

Use [isolated setup](../../../docs/development.md), then from the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/agent_chat/main.exe -j 2
_build/default/examples/agent_chat/main.exe
```

Open Explore run diagram, select/drag a stage, Shift+arrows to move, Ctrl+scroll
or +/- to zoom, pan with scrolling/Alt+arrows, then open details with Enter.
The stage list offers equivalent names/positions/select/open actions without
requiring the graphic. No real source files or job runner are involved. macOS
is the v1 target; Linux graphical qualification is
[informational](../../../docs/platform-release-policy.md).

## Three identities and initial state

Read Registration, `create`, `snapshot`, `publish`, `command`, `initialize`,
`on_event` and `component`. Registration is Absent | Pending | Ready Scene.t |
Failed. The controller owns observable geometry, registration, selected stage,
viewport, pending command and status. Initially geometry is the three fixture
positions, selection None, viewport default (100%), no command, sequence/resource
generation zero, annotation Empty and dark true.

Stable item IDs identify stages. `snapshot` increments resource generation and
creates the complete Run_diagram.scene; path resources require a fresh generation
when their content changes. `command` increments a separate sequence and stores
Canvas.Command.create. The [canvas contract](../../../lib/core/canvas.mli)
requires positive monotonic command sequences; repeating a sequence must not
replay it. Neither counter is a navigation visit ID or scene-registration handle.

`publish` only operates on Ready. `Scene.set` locally accepts a coalesced desired
snapshot or returns an immediate error, which changes status. Native publication
can subsequently reject a snapshot; the controller does not poll Scene.error or
automatically retry that rejection here. Read the
[scoped canvas interface](../../../lib/eio/canvas.mli) for desired versus published
state and explicit retry. Native canvas Failed events also produce a generic
interaction-unavailable status.

## Lazy asynchronous initialization

`initialize ~app ~window` returns a Bonsai effect. Its first deferred thunk
ignores Pending/Ready; Absent/Failed becomes Pending and captures snapshot/theme/
annotation. `let%bind` sequences Scene.create under App.Window.scope window;
this is Effect syntax, not graph-building let%arr. Registration completes after
first encoded/native publication, not after mounting a widget or physical paint.
Scope cancellation suppresses late completion and cleans up late allocations.

Failure sets Failed and exposes Retry diagram. Success stores the registration;
if captured theme/annotation changed while awaiting creation, it publishes the
latest scene before the ready status. Geometry cannot be dragged through this
component before a canvas is ready. This one-start guard prevents duplicate
registrations, while the window scope eventually releases the retained resource.

`component` derives `(active,dark,annotation)` with B.map3 and installs
Bonsai.Edge.on_change. The callback updates cached theme/annotation and publishes
changed colors even when the page is hidden. When active, it issues a Select
command for any remembered selection and starts initialization if needed.
The inspector constructs this graph once and supplies active only for an opened
Diagram route; hiding native content does not dispose the controller/model.

## Native observations and derived view

`let%arr` combines current observable state into views. Ready builds
Canvas.Config from Scene.handle, saved initial viewport, selection accent and
optional pending command; View.canvas has stable key run-diagram and a bounded
300-pixel-high surface. Initial viewport seeds mount/reset, rather than replacing
native pan/zoom on every ordinary view update.

`on_event` dispatches the typed observation:

| Observation | Controller effect |
| --- | --- |
| Activated stage ID | Call supplied on_open effect for a known stage |
| Selection_changed | Store known selected stage or None |
| Moved | Validate pure translation, store geometry and publish; invalid move commands Reset_positions |
| Viewport_changed | Store observed native viewport for labels/remount |
| Command_completed | Clear pending command only for the latest sequence |
| Failed | Set generic interaction-unavailable status |

Events also carry scene revision/generation; this helper dispatches their
observation field, relying on the public mounted canvas/native delivery boundary
rather than adding its own per-event generation comparison. Intermediate native
frames do not synchronously call OCaml. The properties/events/commands form
separate ownership channels; an observation is not an implicit request to remount.

For a drag, native canvas manipulates the stage, queues Moved with resulting
local-to-world transform, and the effect accepts it through Run_diagram.move.
Publication recalculates connectors; the public canvas contract lets OCaml adopt
the moved transform without double-applying the native override. On a boundary
rejection, Reset_positions discards overrides and the model stays unchanged.
Select buttons issue explicit commands and wait for selection observation;
Open buttons invoke route navigation directly. Reset view commands the default
viewport; it does not reset stage geometry. Command completion/scene publication
is distinct from physical painting or an already-delivered viewport snapshot.

Window closure releases the scene; page change/inspector close unmount native
content while positions, viewport and selected stage stay window-local. Another
window has its own scene/model. Annotation applies connector color from the
accepted settings value and does not initiate a provider operation.

A small adaptation is another canvas tool command: use the monotonic sequence,
keep pending state until matching acknowledgement and distinguish view/reset
from resource publication. Add geometry through Run_diagram and retain stable
item IDs; do not perform I/O during scene/view construction. The optional
`python3 scripts/test_agent_chat_diagram.py`/motion checks and their actual
[README](../README.md)/[evidence](../../../docs/evidence/agent-chat-m5.md) limits
remain separate. This source review starts no canvas/native validation run.
