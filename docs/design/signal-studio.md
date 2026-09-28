# Signal Studio — milestone 6 acceptance application

Status: OCH-29 in progress. The pure workspace model passes local expect/format checks;
application composition and platform acceptance below remain planned. The existing
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
| Public OCaml canvas and native chart | Rendered app; selection, real drag, pan/zoom, keyboard and chart callbacks | Pending |
| Pure workspace and document invariants | Expect tests, all valid run values, malformed/oversized/version/ID/bounds checks | Four passing expect tests, including all 101 valid run values |
| Independent component | Combined-app style, pointer/key/AX, commands/events, hide/remount and disposal | Pending |
| Clean consumer | Staged installed libraries, separate component, locked generated backend; macOS run and Linux build | Pending |
| Responsive layout | Real resize across breakpoint; retained state and only active branch exposed to input/AX | Pending |
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

Four expect tests cover document round trips, clamped movement, invalid documents
(oversize/version/run/duplicate ID/NaN/out-of-bounds/unknown selection), exact route
allowlisting and all 101 valid run values. Each valid run constructs a validated
canvas scene and a chart that round-trips through the production data codec.
These are pure model checks; no combined-app graphical or platform acceptance
is claimed yet.
