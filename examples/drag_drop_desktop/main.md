# A desktop drag acceptance fixture with two windows

[main.ml](main.ml) is a diagnostic fixture, not a standalone newcomer app. It
requires an existing absolute file path, release-status file and optional scenario.
Use [drag_drop](../drag_drop/main.md) first for ordinary application structure.
[README](README.md) gives build/harness commands; [dune](dune) uses the standard
Core/GPUIO/Bonsai/Eio PPX. Read Scenario, source_component/target_component,
argument/config setup, source_callback and task assertions. Setup/platform limits are
in [development](../../docs/development.md) and [release
policy](../../docs/platform-release-policy.md).

Scenario is Drop by default, or one of `--reenter`, `--cancel`, `--remove-source`,
`--close-source`, `--shutdown`, `--close-internal`, `--shutdown-internal`. Exact argv
shape is enforced; no `--self-test` mode exists. The macOS driver creates a regular
file plus held/released status file, posts a real AppKit gesture and releases that
synchronization after driving. Do not launch with arbitrary nonexistent paths or
without status coordination and expect automatic completion.

File_path validates native path bytes; `File.create` supplies known is_directory
false, and `Payload.files` makes immutable bounded path metadata. No file lookup or
content read is performed by these constructors. `Source.create` explicitly allows
desktop files; this requires known metadata and does not authorize copying,
moving/deleting or promise OS completion. Targets accept Files only. See
[Drag_and_drop](../../lib/core/drag_and_drop.mli).

source_component constructs persistent Bonsai status instruction, a presence flag
initially true and a received flag initially false. target_component owns independent
a received flag initially false. `B.state` returns reactive values/setters; `let%arr`
reads current ordinary values to derive views. `and` lists dependencies, not threads.
`Edge.on_change` records present/ received using `Bool.equal` and deferred thunks.
`E.Many` combines observation effects, source removal after Desktop_offered and
status changes. Source's drop target wraps its drag source to support reentry;
receiver has its own native target and renders green/received text. Source removal
destroys handler placement, not the OS's immutable offered payload. These refs are
test observations rather than application persistence or duplicated native drag
ownership.

`App.run` owns GPUI OS thread and one OCaml Eio UI domain, opening two 320 × 280
windows. source_callback records typed source events then force-closes/shuts down on
Started for internal scenarios or Desktop_offered for OS scenarios. Native ownership
prevents callbacks after retired source/window; surviving target can still receive
immutable OS offer. For Drop, native source offer crosses OS and second-window
incoming path uses Desktop origin, new Gesture_id and unknown is_directory metadata.
Source ends Unconfirmed: it does not assert OS copy/move success. Reentry restores
original gesture and known metadata, and can end Internal_drop. Target event effect
sets received; Bonsai derives result and GPUIO submits the update. Rust handles
native gesture/preview and current acceptance; OCaml does not synchronously decide OS
drop permission.

Non-shutdown scenarios start one application-scoped Eio task with explicit clock,
30-second timeout and 5 ms waits. It awaits expected received/source-end or removal/
close, then reads release-status through explicit Eio filesystem capability until
text released. This is the only fixture file I/O; dragged content is never opened.
Assertions inspect Starts/Offers/Ends, gesture identity, exact path and metadata,
Desktop/Internal origins and source event counts. Cancel/close permit no drop;
removal suppresses late source Ended while offer remains receivable. A native render
callback for source (reentry) or target is awaited via `Eio.Promise`, then completion
effect closes both windows. Shutdown scenarios complete from source callback instead,
asserting retained event counts after runner exit. Each scenario prints its
GPUIO_DRAG_DROP_*_OK marker.

The driver requires macOS Accessibility, targets owned child windows, verifies
fixture content unchanged and reaps child on failure/success. It is actual AppKit
gesture automation; it does not imply VoiceOver, physical pixel/frame or Linux GUI
qualification. Scope/window cleanup and cancellation follow
[App](../../lib/eio/app.mli)/[Scope](../../lib/eio/scope.mli); OS drag lifetime is
not an Eio task or proof of file operation success.

For application file export, construct validated bounded payload metadata with
explicit prior Eio work, keep read/write authorization separate and handle
Unconfirmed honestly. Do not copy the fixture status polling into product UI, keep
I/O outside `let%arr` and choose operation scopes independent of transient native
source placement. If adapting multiple files, preserve order and limits, and do not
invent directory metadata for incoming paths that report None.