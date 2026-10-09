# Native canvas interaction and scoped scene publication

[main.ml](main.ml) registers the typed scene from [Plot](plot.md), mounts a canvas,
and handles native selection/move/viewport observations. Read Snapshot and the
startup helpers update/command/publish/on_event, then component, registration task
and optional self-test. [README](README.md) gives exact build/run/diagnostic
commands; [dune](dune) links Core/GPUIO/Bonsai/Eio and PPX. Setup and platform
limits are in [development](../../docs/development.md) and
[release policy](../../docs/platform-release-policy.md).

`--large` selects 19,000 decorative points rather than 160; both have four interactive
samples and use deterministic local data. `--trace-events` logs Canvas.Event sexps;
self-test enables it too. Ordinary launch opens focused 1040×840 with fixed 700×470
canvas and scrolling containing page. No assets/service/credentials are required.
Close lab force-shuts down the app with App.shutdown rather than a close decision.

Snapshot.t owns Plot, optional borrowed scene handle, optional Canvas.Command,
selected item ID, policy flags, status, latest command ack and observed
(scene_revision,scene_generation). It starts without handle/command/selection,
flags false, ack/epoch 0 and preparing status. `B.Expert.Var` stores application
state outside the component. `update` reads latest state and applies a functional
change. Native renderer owns viewport, interaction preview and drawing resources;
Plot owns final sample positions. `registered` retains scoped Scene controller;
`sequence` is a monotonic command counter in application UI-domain ownership.

`command` increments sequence, validates Canvas.Command.create and stores it.
Repeated renders of the same sequence are not new requests. `publish ~reset`
uses Scene.set or reset on registered scene, then updates Plot/status locally.
Set preserves generation and coalesces not-yet-started uploads; reset starts a new
resource generation on accepted publication. Immediate successful set/reset
means local acceptance, not native publication or paint. `Scene.is_published`
observes native acceptance; asynchronous errors can retain prior published scene.
Read [Canvas adapter](../../lib/eio/canvas.mli).

`on_event` returns E.of_thunk to update epoch and optionally trace. Selection_changed
stores ID; Activated formats status; Moved calls Plot.move then publishes one final
scene; Viewport_changed formats zoom status; Command_completed stores ack; Failed
raises a typed diagnostic. Native drag-preview frames stay in Rust. `name` looks
up an ID with Unknown fallback; details assumes a selected ID remains in Plot.

`component` reads reactive Snapshot through let%arr and derives a view. A reactive
value changes over persistent graph lifetime; let%arr reads its ordinary value,
not executing I/O. Button callbacks return deferred thunks. Before registration
there is only status text. After handle exists, validated Canvas.Config carries
scene, label, disabled flag, selection color and optional command. V.canvas uses
stable key plot and on_event; Display Hidden retains placement/resources while
blocking interaction. Selection sidebar sends explicit Select commands. Reset
viewport and dataset have distinct native command versus scene-generation paths.
Style helpers provide validated logical-pixel geometry/colors; no custom Rust
painting or native callback into OCaml layout is required.

Select a sample then drag it: native input owns hit testing, preview and viewport;
selection observation reaches OCaml and updates inspector. On drag completion,
Moved contains final transform; its effect updates Plot and calls Scene.set once.
Bonsai derives inspector/status and GPUIO submits the view, while adapter publishes
scene bytes asynchronously. Native acceptance updates epochs/observations.
No implicit per-preview position replacement is fed back. Transaction/command
acknowledgement is separate from physical frame presentation.

`App.run` owns GPUI OS thread and one OCaml Eio UI domain. Startup opens window
then starts a window-scoped task, bounded to 60 seconds for registration/test.
Local on_ui enqueues effects through Scope.Expert.enqueue/E.Expert.handle and
bridges results via Eio.Promise; ui wraps ordinary mutations in E.of_thunk.
Scene.create is evaluated on owning UI loop; scope cancellation suppresses late
completion and retires allocation. Registration alone does not mount a widget;
its success sets handle, triggering view derivation. Borrowed handle does not
extend registration lifetime. See [Scope](../../lib/eio/scope.mli),
[App](../../lib/eio/app.mli), [Canvas](../../lib/core/canvas.mli).

## Optional diagnostic

After building from repository root:

```sh
_build/default/examples/canvas/main.exe --self-test
_build/default/examples/canvas/main.exe --self-test --large
python3 scripts/test_canvas.py --screenshot scratch/canvas-owned.png
```

Self-test awaits Select ack and selected ID, Set_viewport ack, moved same-generation
scene publication, reset publication and Reset_viewport ack. It checks revision ≥ 3
and generation 2, requests native render via promise, waits idle intervals and
asserts unchanged OCaml commit count. Then it releases registration, sets handle
None to remove canvas, awaits render and checks released flag/completion. Success
prints GPUIO_CANVAS_PUBLIC_OK with item/byte counts, then completion effect shuts
down app. GPUIO_CANVAS_APP_RETURNED prints after ordinary/test runner returns.
The 5 ms until loop reads predicates through UI effects; this polling and stats
assertion are diagnostic plumbing, not production scene scheduling.

Callbacks attest native rendering/admission, not GPU pixels or physical display.
The separate foreground macOS script uses actual AX/keyboard and requires
Accessibility access; screenshot option captures owned window. These tests are
separate from VoiceOver, Linux GUI and broad performance qualification. Asset/
scene releases retire scoped resources, not instantaneous physical-memory proofs.

To load a new dataset, validate stable IDs and clear/reconcile selection if an ID
is removed, then choose set for related snapshots or reset for new generation.
Handle publication errors explicitly rather than assuming Plot/status means
native accepted data. Preserve monotonic command sequence and stable view key;
use typed scene APIs rather than raw resource handles. Keep external I/O in
explicit Eio scopes and never inside let%arr. Bound update work if extending large
mode; existing complete-scene reconstruction is not a universal chart engine.
