# Signal Studio — milestone 6 acceptance application

Status: OCH-29 in progress. The pure workspace model and combined canvas/chart/
component application pass local macOS checks. Packaged links/reopen and Eio
documents, packaged OS notifications and full/reduced motion now pass combined-app
checks. Repeated lifecycle/workload acceptance and consolidated hosted gates remain pending. The existing
agent-chat example stays intact. This is a focused model-evaluation plotting
workbench, not a complete diagramming product or scientific environment.

## Application and ownership

The main window combines an OCaml-authored interactive canvas, a native latency
chart and an inspector. Four named simulated model samples have stable canvas
identities; moving a sample changes its latency/quality coordinates and the
corresponding chart. A separately packaged SDK counter controls run number
(0–100), changing the deterministic signal without changing sample identities.
Native selection, keyboard motion and drag return semantic observations; OCaml
updates the authoritative immutable workspace and republishes the scene/chart.
Pan/zoom remain native view state and are not written into the data document.

`examples/signal_studio/model/Workspace` owns pure data, validated movement,
selection, run changes, scene/chart construction and a version-1 document codec.
It uses public GPUIO types only. Sample movement is clamped inside the visible
plot; untrusted documents reject out-of-range/NaN values instead of clamping.
Documents are at most 16 KiB and include exactly the four known IDs. Decoding
never recreates a native handle or grants I/O authority. Deep links accept only
`gpuio-signal://sample/{1..4}`, without query/fragment, to select an existing item.

Application-scoped Eio resources own the canvas scene and chart registration.
Window views borrow handles; hiding an inspector or switching a responsive
presentation must not cancel data tasks. Files are read/written only through
explicit Eio filesystem capabilities. Save/load results update represented-file
and edited metadata separately; unsupported native metadata does not imply the
file operation failed. A failed load preserves the current workspace.

## Native component and consumer boundary

Reuse `examples/extension_package`: its counter has public OCaml properties,
commands/events and a separately consumable Rust crate importing only
`gpuio-extension-sdk`. Application code imports the typed OCaml library and
selects its factory through the documented composition manifest. It must not
import private host modules or link a second GPUI version. Counter styling,
keyboard/AX activation, event routing, disable/hide/remount and scope disposal
must be checked in the combined app, not inferred from standalone unit tests.

A staged-prefix consumer build will copy this application/model and the component
into a fresh Dune project, compose its backend with the pinned Cargo.lock and
compile against installed public GPUIO libraries. It must not install into or
mutate the current opam switch. macOS runs the interactive acceptance; Linux clean
build/unit checks are required, with GUI evidence deferred separately to OCH-17.

## Desktop and motion integration

Use `App.run_desktop` with a dedicated application identity and declared
`gpuio-signal` scheme. Incoming links wait for model/window readiness before
selecting/focusing a sample. Reopen creates a live window when necessary; actions
route through current window handles. An optional explicit file path for tests
provides a temporary document; ordinary UI uses a native file picker and Eio I/O.
File reveal/open and represented-document metadata use the accepted desktop API.
No incoming link is interpreted as a filesystem path or arbitrary command.

A user action requests notification permission explicitly. Posting a completed
run uses a scoped receipt; clicking its action focuses the current workspace.
Unbundled macOS, missing Linux services, denied permissions and unsupported
capabilities remain visible typed outcomes, with an in-app status fallback.
Packaged tests exercise real OS delivery; deterministic tests cover unavailable
paths. Do not treat a returned post receipt as proof of visible presentation.

Responsive container variants change the inspector presentation around a chosen
width breakpoint. A spring animates inspector expansion, a short sequence marks
run changes, and two indicators share a native animation clock. Reduced-motion
policy must settle appropriately; no per-frame OCaml animation callback is added.

## Required capability-to-test matrix for OCH-17

| Capability | Required evidence | Current OCH-29 state |
| -- | -- | -- |
| Public OCaml canvas and native chart | Rendered app; selection, real drag, pan/zoom, keyboard and chart callbacks | Local AppKit script passes |
| Pure workspace and document invariants | Expect tests, all valid run values, malformed/oversized/version/ID/bounds checks | Five passing expect tests, including all 101 valid run values |
| Independent component | Combined-app style, pointer/key/AX, commands/events, hide/remount and disposal | Combined style, AX/key, disabled pointer, events and hide/remount pass; explicit package command/repeated disposal checks remain |
| Clean consumer | Staged installed libraries, separate component, locked generated backend; macOS run and Linux build | Fresh macOS consumer build/self-test passes; Linux hosted build pending |
| Responsive layout | Real resize across breakpoint; retained state and only active branch exposed to input/AX | Wide/compact/wide resize retains run and moved sample; explicit inactive-branch/motion checks remain |
| Springs, sequences, synchronized indicators | Native geometry/phase and reduced-motion checks in combined app | Full/Reduce actual spring heights/interruption, ordered native stages, synchronized painted intensity and paused paint pass locally |
| Documents and files | Eio save/load round trip; dirty metadata; actual OS file actions and typed unsupported behavior | Real macOS Save/Open panels and Finder reveal, unsaved reveal fallback, Eio round trip, native dirty/file metadata and concurrency/invalid/missing-file preservation pass; Linux unsupported metadata/private-bus evidence remains separate |
| Deep links/reopen | Packaged cold/warm OS delivery and current-window selection/focus | Local packaged startup/cold/warm/invalid links and same-process reopen pass |
| OS notifications | Packaged action/receipt routing; denied/unavailable fallback; stale-window handling | Real macOS named/default actions, same-process reopen, replacement, stream completion, dismissal and quit cleanup pass; five pure expect tests and real unbundled fallback pass; first-time permission on a fresh hosted runner pending |
| Workloads and lifetime | Named hardware, native resources/queue charges, repeated window/resource cleanup | Pending |
| Cross-platform release | Required macOS/Linux CI; report X11/Wayland GUI separately | Pending; full Linux GUI under OCH-17 |

Existing chart, canvas, extension, desktop, notification and motion test suites
provide supporting evidence. They do not replace this application-level matrix.
Record any API issue discovered while composing these features and resolve it or
document an accepted contract before calling OCH-29 complete.

## Initial model validation

Local isolated OCaml 5.3/Core toolchain passes:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build @examples/signal_studio/model/runtest @fmt -j2
```

Five expect tests cover document round trips, clamped movement, invalid documents
(oversize/version/run/duplicate ID/NaN/out-of-bounds/unknown selection), exact route
allowlisting and all 101 valid run values. Each valid run constructs a validated
canvas scene and a chart that round-trips through the production data codec.
These model checks are separate from the combined-app acceptance below.

## Combined-app local checkpoint

The executable and generated backend live in `examples/signal_studio`. The backend
selects the separate counter package through `native.json`, with a reviewed lock
file and no private host import. The workspace is authoritative: a native move
republishes canvas and chart data, while canceled streams invalidate already
queued UI work using an epoch. Run values are bounded to 0–100; late counter events
outside the domain are rejected without terminating the app. The inspector and
chart display the same latency/quality units and sample colors as the canvas.

`python3 scripts/test_signal_studio.py` exercises actual macOS AX/keyboard/pointer
input, including sample selection/movement, drag, wheel pan/zoom, chart selection,
component disable/hide/show, twelve streamed updates, responsive resize with
retained data, reset/remount and OS close. Screenshots cover wide, streaming and
compact views. A discovered test-helper issue was resolved: synthetic mouse events
now set explicit modifier flags, avoiding inherited Control converting a macOS
left drag to a right-click. No canvas production change was needed.

The public `--self-test` checks resource publication, semantic command completion,
render callback and zero OCaml canvas/chart registration charges after release.
This is not yet the repeated lifecycle/performance workload. Dedicated native
phase/geometry/reduced-motion evidence is now recorded below; declarations alone
do not satisfy that acceptance row.

## Document and desktop checkpoint

`Documents` owns one picker/I/O admission and a saved snapshot. It compares the
current workspace to that snapshot for the edited state. Saving an older snapshot
does not mark newer edits saved. Load captures the workspace before picker/read
and abandons replacement if it changes; reset invalidates older completions.
Native metadata has a separate, revision-fenced result, so an unsupported badge
cannot turn a successful file write into a file failure. The main process retains
the model/resources across native window close/reopen and reapplies metadata to
the new window handle. The example has no autosave or quit-confirmation flow;
users explicitly save before quitting.

`files/Document_file` receives explicit Eio path/random capabilities. Reads use a
16 KiB+1 bounded buffer before the validated codec. Saves hold the parent directory,
create a private exclusive temporary sibling, write/sync, then atomically rename
it over the selected destination. Cleanup is cancellation-protected. Existing
permissions are not preserved, destination symlinks are replaced rather than
followed, and directory crash durability is not promised. Three expect tests cover
round trips/overwrites, the exact size boundary, oversized/malformed/missing input,
failed replacement cleanup and symlink replacement.

`--self-test --document-path=/absolute/fixture.signal` checks an actual Eio file,
exact native represented-file/edited observations, save-versus-edit consistency,
load-versus-edit preservation, reset fencing, busy rejection and invalid-load
preservation. The delayed race uses controlled Eio promises rather than a slow disk.
This flag is a test operation that overwrites the caller-provided fixture; it is
not interpreted from a deep link.

`python3 scripts/test_signal_desktop.py` runs that test in a disposable fixture,
then packages the public app and checks startup/cold/warm link ordering after
readiness, unknown-sample rejection, same-process close/reopen, native Save/Open
panels, dirty-state open guard and restored run/component state. The script owns
its launch proxy, cleans up only the exact bundle's processes, unregisters the
bundle, and retains logs/screenshots in the specified output directory. The
same walkthrough now checks real Finder visibility for the uniquely named
disposable directory and closes only that window. Unsaved reveal keeps an in-app
message; missing-file load preserves the model and reports failure. An attempted
missing-file reveal check confirmed the documented macOS contract: AppKit has no
completion result, so submission succeeds even when the path has disappeared.
The test does not misreport that as visible selection or unavailable detection.
Linux build and desktop-entry generation are separate
from Linux GUI acceptance, which remains under OCH-17.

## Run notifications checkpoint

`notifications/Run_alerts` owns explicit opt-in, one receipt, one in-flight
operation and one latest pending run on the OCaml UI domain. A nonprompting probe
does not opt in. Only `Enable alerts` requests permission and retries initial
service availability. Native notification mutations remain asynchronous through
`Gpuio_eio.Notification`. Content respects body/action capabilities, replaces an
existing receipt when supported, or dismisses before posting again. Terminal
service errors disable automatic alerts; run results remain visible in the app.

Serial event delivery waits asynchronously if an action arrives before its post
reply. Only the current receipt and declared action activate the workspace; the
activation callback resolves the live window at invocation time. Scope cleanup
closes admission, clears pending state and fences late replies/events without
calling view/log callbacks. Five controlled expect tests cover explicit opt-in,
permission denial, unavailable services, coalescing, capability fallbacks,
early/stale events, terminal errors and cleanup races.

`python3 scripts/test_signal_notifications.py` now passes locally on macOS with
an ad hoc signed bundle: real named/default Notification Center actions,
same-process reopen after window close, replacement, streamed-run completion,
explicit dismissal and disappearance on quit. Replacement is checked in
Notification Center rather than assumed to produce a new banner. It also runs
the unbundled executable and verifies typed `Unavailable` with an in-app result.
The ordinary graphics walkthrough checks the unavailable popover and still passes
input/resize/streaming. Current retained packaged evidence starts with previously
authorized permission; first-time permission on a fresh hosted runner remains
unverified. The required macOS workflow includes both this combined-app check and
the separate notification-service walkthrough. Hosted results remain pending.

## Combined motion checkpoint

The app defaults to `Animation.Preference.System`; explicit `--full-motion` and
`--reduced-motion` options override only this application. `--motion-check` logs
bounded stage/completion observations by source, without per-frame OCaml callbacks.
The inspector region and two activity labels have named accessibility metadata.

`python3 scripts/test_signal_motion.py` passes on local macOS in both policies.
The inspector region changes by exactly 172 logical pixels; Full traverses
intermediate heights and settles after reversal, while Reduce uses only accepted
endpoints. A queued AX press may briefly expose the previous layout before the
application processes it. Hidden inspector controls disappear from accessibility.
The keyed run sequence reports ordered stages 0 and 1 plus completion, with
`Played` versus `Reduced_motion` outcomes from the actual native painter.

Eight owned-window captures compare both labels in each frame. Full-mode green
intensity changes together (local retained sample spans 99–218, equal pairs);
Reduce stays at 221 for both labels in all eight captures. After streaming ends,
two captures show identical paused-label pixels. These measurements check shared
painted phase, not frame rate or input latency. Wide/compact/wide resize retains
run 13 and inspector controls. Logs, captured images and raw height/intensity
samples are retained by the script; required macOS CI now includes it. Resource
workloads and explicit repeated extension disposal remain separate acceptance.
