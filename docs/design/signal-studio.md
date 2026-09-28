# Signal Studio — milestone 6 acceptance application

Status: OCH-29 in progress. The pure workspace model and combined canvas/chart/
component application pass local macOS checks. Desktop integration, dedicated
motion/workload acceptance and consolidated hosted gates remain pending. The existing
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
| Springs, sequences, synchronized indicators | Native geometry/phase and reduced-motion checks in combined app | Pending |
| Documents and files | Eio save/load round trip; dirty metadata; actual OS file actions and typed unsupported behavior | Pending |
| Deep links/reopen | Packaged cold/warm OS delivery and current-window selection/focus | Pending |
| OS notifications | Packaged action/receipt routing; denied/unavailable fallback; stale-window handling | Pending |
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
phase/geometry/reduced-motion checks also remain required; merely declaring the
spring, sequence and shared clock does not satisfy that acceptance row.
