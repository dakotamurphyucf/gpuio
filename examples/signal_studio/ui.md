# Describe views from a snapshot and deferred actions

[ui.ml](ui.ml) and [ui.mli](ui.mli) turn Ui.Snapshot.t and Ui.Actions.t into a
Gpuio_bonsai.View.t. This is a stateless layout function: it creates validated
view/configuration values, not a Bonsai graph, Eio tasks or native registrations.
Read Snapshot/Actions, style helpers, indicator/inspector, body/alerts_panel, then
view. [Component](component.md) makes it reactive; [Application](application.ml)
owns the observable snapshot, effects, scopes and native resources.

From the repository root after [setup](../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/signal_studio/main.exe -j 2
./scripts/gpuio exec dune exec examples/signal_studio/main.exe
```

Drag one of four fixture points, click a named sample, change the run control,
resize and toggle the inspector. All values are simulated; no provider or external
model dataset is read. The [model guide](model/workspace.md) explains their units.

## Snapshot and action types separate description from execution

Snapshot contains the immutable workspace plus optional canvas scene handle,
chart resource and canvas command; inspector/compact/running/status fields;
extension generation/optional sequenced command/disabled/visible fields; document
and alert state plus popover open state. Handles are borrowed descriptions of
application-owned registrations, not serializable domain data. None handles
produce Preparing labels while startup runs. Actions contains typed callbacks for
sample selection, Canvas.Event, Chart.Event, int Extension.Event, container-query
selection and animation events, alongside deferred stream/reset/document/alert/
quit effects. Merely putting an effect in on_click does not execute it.

The px/full/color/style/key helpers construct validated lengths, colors, styles
and keys. ok/checked constructors treat source literals as invariants. text adds
font/foreground defaults; button attaches its supplied effect and optional native
disabled state. [View's public contracts](../../lib/core/view.mli) explain native
controls and asynchronous observations. Layout recomputation remains an ordinary
OCaml function call, not an imperative native widget creation call.

## Canvas and chart presentations use shared registrations

body receives an explicit compact argument. Compact uses a 490×330 canvas with
initial zoom 0.7 and 490-wide chart; wide uses 700×470/zoom 1 and 310-wide chart. It
creates Canvas.Config from the supplied scene, initial origin (0, 0), optional
sequenced command and selection color. View.canvas forwards events to actions.canvas.
The native viewport owns live pan/zoom; rebuilding an initial_viewport descriptor
is not an instruction to reset it. Reset view instead uses an explicit application
command. Chart.Config uses the chart resource, label and sample-color palette;
View.chart forwards observations to actions.chart. No scene/chart data is created
by these view constructors; Application publishes Workspace.scene/chart separately.

A sample-button click supplies actions.select ID. Application issues a Canvas
Select command with increasing sequence rather than immediately changing the
model in the view. The native Selection_changed observation returns through the
handler's deferred effect, updates Workspace selection and document dirty state,
then Component's let%arr sees the new snapshot. Dragging instead yields Moved,
which Application converts to a point, validates/clamps through Workspace.move
and republishes scene/chart. Chart selection changes status; it does not select
a workspace sample. Native Command_completed and Ready observations support
checks separately from model publication.

## Responsive branches and native lifetimes

view defines compact/wide typed branch IDs and one [Container_query](../../lib/core/container_query.mli)
rule: assigned width ≥ 1080 logical pixels selects wide, otherwise compact. It
supplies both body presentations to View.container_query with a full-width,
explicit-height outer box (620 when snapshot.compact, 550 otherwise). Native layout
chooses a branch without calling OCaml to measure it; on_select reports the
painted branch asynchronously. Application then records compact for header copy/
font and outer height. snapshot.compact does not choose which body was supplied.

Both branches remain mounted in Bonsai; the hidden presentation retains native
state but does not paint/receive input. They refer to the same application scene/
chart registrations, with keys scoped to their branch presentation. Stable keys
preserve native reconciliation identity; they do not transfer resource ownership.
Window close disposes its mounted views while application handles remain available
for reopening. This function does not cancel application tasks when a branch hides.

## Inspector, shared clocks and run sequences

indicator constructs an alternating opacity program from 1 to 0.35 with a 700 ms
ease-out tween. canvas-live and chart-live have distinct keys but share native
clock group signal-activity. When not running, playback is Paused. Accessible
labels distinguish canvas/chart activity and Ready/Updating state.

inspector uses a spring (stiffness 220, damping 29, mass 1, epsilon 0.05, max 1.4 seconds)
to target height 172 or 0. The keyed animation wrapper receives actions.on_motion
named inspector. Its selected-model panel uses active:snapshot.inspector and
hidden:Unmount: inactive descendants are removed, while application selection/
extension flags remain stored outside it. The wrapper clips overflow during the
height animation; no OCaml timer drives it. The detail label derives selected
sample latency/quality, and Lock/Hide buttons dispatch independent control effects.

The run status sequence starts opacity 0.45, tweens 100 ms to 0.75 then 180 ms to 1.
Its key is the run integer, so a different run creates a different presentation
identity; unrelated snapshot changes at the same run preserve that identity.
[Animation contracts](../../lib/core/animation.mli) and View.animate_program define
native retargeting, hidden timing and reduced motion. System policy/CLI overrides
are owned by Application, not changed in this layout. Ordered native events and
actual painted frames are stronger evidence than constructing these descriptors.

## Extension, document status and alert controls

view creates [Counter.Properties](../extension_package/ocaml/gpuio_example_counter.mli)
from run and step 1, then Counter.instance with explicit generation and optional
(sequence,value) command. It is disabled when locked or run=100. Native Data
returns through actions.extension; Application validates 0–100 even if an already
queued click arrives after disabling. Generation increments on workspace reset
explicitly reset native counter state. The keyed extension's Display Hidden
hides its presentation; the function still supplies the instance descriptor.
Do not equate Hide control with resetting generation or dropping domain state.

Document buttons always dispatch controller effects; they are not disabled by
busy here. [Documents](documents.md) enforces serialized admission and preserves
edits across I/O. Status text prioritizes Working, Unsaved, Saved, then Untitled;
native metadata results are independent. Alerts uses an optional popover content:
None closes it; outside dismissal invokes close_alerts. Enable is disabled when
busy/already enabled, Notify when busy/not enabled, Dismiss when busy/no current
notification. Permission requests, notification ownership and asynchronous replies
belong to injected alert actions, not to popover construction.

For a small adaptation, change plot spacing or a pure detail label without adding
I/O. For another breakpoint, update validated named rules and supply exactly one
presentation per referenced ID, retaining a meaningful outer size. For another
native resource, create/register it in Application and pass only its handle in the
snapshot. [README checks](README.md#local-checks) cover native motion, responsive
layout, extensions and document/notification flows; source review does not newly
execute them or establish platform acceptance. Full Linux GUI remains deferred.
