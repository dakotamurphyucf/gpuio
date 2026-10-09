# Picker state: application open intent and native visibility

[picker_state.ml](picker_state.ml) and its [interface](picker_state.mli) define the
controlled export-destination chooser plus immutable workspace fixtures. They use
public Choice/Choice_picker types without constructing a Bonsai graph, native popup,
Eio controller or I/O task. [choice_picker_cases.ml](../choice_picker_cases.ml)
provides their three consuming preview graphs.

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
./scripts/gpuio exec _build/default/examples/gallery/main.exe
```

Choose **Dates & colors**, find **A destination you control**, choose Downloads,
pause destination changes and reset. Search the separate 4,096-workspace fixture.
There are no actual downloads, workspace directories or export writes here: names
are mock choices. No independent executable, asset or model-specific diagnostic
flag exists. [Development](../../../docs/development.md) covers toolchain setup;
this review adds no popup/keyboard/platform acceptance.

Read `collection`, `destinations`, `t`, `config`/`apply`, then workspace fixtures.
`collection` creates typed string `Choice.Id`s and labeled/disabled choices, validates
a `Choice.Collection`, then wraps it as a flat picker collection. Destination IDs
are workspace/downloads/archive; archive is deliberately disabled. Abstract model
`t` derives typed equality and stores selected, can_open, requested_open and visible.
Initial selection is Single None, permission true, requested intent false and actual
visibility false.

`config` creates a clearable single selection with label Export destination,
Controlled requested_open and disabled=(not can_open). `requested_open` is the
application preference; `visible` is an observation of whether native placement
currently has a visible popup. Clipping/unavailability can make them differ.
The [choice-picker contract](../../../lib/core/choice_picker.mli) distinguishes
selection requests, controlled opening and native Visibility observations.

`Request` applies `P.Config.apply_request` to the current config only while permission
is true, rejecting disabled/missing IDs through that validated library policy.
`Request_open` gates intent by current permission. `Observe` updates only visible
for Snapshot/Changed, never erasing accepted open intent. Toggle_permission flips
permission and clears intent when disabled. Reset clears selected/intent but retains
permission and last observed visibility until the next native observation arrives.
`selected_label` relies on this fixed selectable catalog: no selection is Not chosen,
workspace is Current workspace and its other legal selection is Downloads.

The caller's `controlled` graph uses `B.state_machine0 ~equal:State.equal`, with
`B.map state ~f:State.config` feeding `Gpuio_eio.Choice_picker.create`. Bonsai owns
the model/injection effect; native code owns search draft, placement, scrolling and
visibility. `Selection_requested` injects Request, Open_requested injects Request_open,
and Visibility injects Observe; Query_changed is ignored by this application model.
`let%arr` derives a card, controller view and selected/status text from current state.
No helper here executes a popup during view construction.

For a concrete trace, Open_requested true sets intent true. If clipping reports
Changed (false,Unavailable), requested_open stays true while visible becomes false.
A later visible observation updates that readout. Selecting Downloads injects a typed
request and derives selected Downloads. Now permission becomes false before another
queued selection request reduces: the request is ignored, current destination retained
and open intent cleared. Native reconciliation subsequently reports visibility false.
Reset remains an explicit application action and clears selection even while paused.

`workspaces` contains 4,096 stable IDs workspace-0001 through workspace-4096;
`first_workspace` is a one-item fixture and `empty` has no choices. These are data,
not 4,096 mounted Views. The directory preview uses a separate selection model with
native substring query/virtual rows; it does not reuse the destination's t or label
function. The [existing tests](../../../test/gallery/picker_state_test.ml) check
open/visible separation, permissions, disabled/missing requests and valid large/empty
public descriptions. They were read, not run for this guide.

The retained model needs no disposal; caller/controller/window own popup query/native
lifetimes and stale event handling. To add another destination, give it a stable
unique ID and update `selected_label` to resolve that new legal selection explicitly
rather than incorrectly labeling every non-workspace ID Downloads. Preserve the
intent/observation split; deriving intent from visible would close a clipped popup
that the application still wants open.
