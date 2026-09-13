# OCH-11 drag/drop local integration evidence

Platform: macOS, repository-isolated OCaml 5.3/Core+Bonsai v0.17/Eio environment,
pinned Rust toolchain and GPUI `a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b`.
The initial data checkpoint is `900e17a`; the integration checkpoint follows it.
No hosted CI, Linux GUI or completed OS file-transfer result is claimed here.

## Native window dispatch

`cargo test -p gpuio-native --features native-tests --test native_drag_drop`
opens a real local GPUI window and injects GPUI platform events. Verified:

- A rejecting nested target leaves the offer for its accepting parent; an
  accepting child receives the drop once.
- Source payload changes while dragging preserve the original offer. The next
  gesture receives the new payload. Changing target acceptance ends its prior
  accepted hover immediately, without waiting for another mouse move.
- Escape, disabling and hiding cancel; releasing outside targets yields one
  Unconfirmed end. Later mouse release does not invent a drop.
- Incoming file paths preserve non-UTF-8 Unix bytes and unknown directory metadata.
  An oversized file offer rejects as a whole, never delivering a prefix.
- Mounting a trapping focus scope cancels an outside drag and prevents restart
  behind the trap. Activating a second native window cancels an ordinary internal
  drag, clears manager state, ignores a late release, and permits a fresh drag
  after returning to the source window.
- Removing a source during an active gesture releases GPUI active drag state and
  suppresses later callbacks for that removed handler.
- A weak reference to the removed source configuration expires after redraw;
  the preview and manager do not retain it. Gesture/hover state is empty.

The success marker is `GPUIO_DRAG_DROP_NATIVE_OK`. Injection directly into GPUI
is **not** an actual AppKit mouse-down or an OS cross-application drag test. The
remaining OS export/reentry and multi-window checks are listed in the design.

## Public and protocol checks

`./scripts/gpuio exec dune exec examples/drag_drop/main.exe -- --self-test`
passes with `GPUIO_DRAG_DROP_PUBLIC_OK`. This checks the full OCaml/Bonsai/Eio/native
path for mount, disabled/enabled updates, keyed removal and shutdown. It does not
inject gestures or verify callback effects from an actual drag.

Core expect tests cover current callback routing, stale handler/revision rejection,
malformed samples, bounded snapshot/drop decoding and independent binary fixtures.
Rust session tests cover atomic invalid-tree rejection, payload accounting, live
owners and terminal delivery after disable. Mailbox tests cover consecutive motion
coalescing, lifecycle barriers, byte-budget overflow and bounded drain batches.

Independent request and event fixtures agree in OCaml and Rust. All 19 source/
target event cases and every fixture truncation are checked; existing data fixtures
retain raw custom bytes and Unix paths. An uncaught custom-reader exception found
during development was fixed by explicitly converting it in `Wire.Event.decode`.

Full local validation passed:

```sh
cargo clippy --workspace --all-targets --features gpuio-native/native-tests -- -D warnings
cargo test --workspace
./scripts/gpuio exec dune build @runtest @all @fmt
cargo fmt --all -- --check
git diff --check
```

After the stationary-hover policy fix, native-crate Clippy, the expanded native
suite, full Dune build/tests/format and the public lifecycle test passed again.
The existing `native_pointer` suite also passed, covering shared Escape dispatch,
modal gating, window deactivation, captured styling and nested pointer ownership.

Three existing system-libwayland tests remain explicitly ignored on macOS and
require the consolidated Linux gate. Logs and per-ticket recovery notes are under
`scratch/agents/root-20260912-milestones/`; they are local artifacts, not build inputs.

## Actual AppKit gesture to public Bonsai callbacks

The separate-process macOS driver now posts one system mouse drag into the public
`examples/drag_drop/main.exe --gesture-self-test` application. AX lookup finds the
named groups; system-wide AX hit-testing verifies the child PID before each move
or press. Release is guaranteed during driver unwinding. The Python harness owns
and cleans up only its spawned child. This foreground test requires accessibility
permission and briefly moves the pointer.

```sh
./scripts/gpuio exec cargo test -p gpuio-native --features native-tests --test native_drag_drop --no-run
python3 scripts/test_drag_drop.py --driver target/debug/deps/native_drag_drop-<cargo-reported-hash>
```

Local macOS execution passes `GPUIO_DRAG_DROP_GESTURE_OK`: exactly one source
Started, target Dropped and source Internal_drop share the expected gesture ID
and text payload. Accepted hover reaches Bonsai. An observed result-model change
followed by a painted-frame acknowledgement precedes clean application shutdown.
The ordinary lifecycle self-test also passes after this addition.

Process-targeted mouse posting did not produce a complete AppKit gesture here;
the driver uses the system event stream. A CGWindow bounding-box check falsely
reported a Wispr Flow overlay as an obstruction; AX hit-testing correctly checks
the interactive owner. Waiting for the rendered AX tree also avoids activation
before the child's accessibility server starts. No production GPUI/GPUIO behavior
patch or debug input listeners are needed for this test.

Native Clippy (all targets/native-tests), full Dune build/tests/format, the expanded
native drag suite and both public tests pass locally. Logs are
`drag-appkit-lifecycle-clippy.log`, `drag-appkit-final-dune.log`,
`drag-appkit-lifecycle-native.log`, `drag-appkit-public-v6.log` and
`drag-appkit-final-lifecycle.log` in the personal scratch directory.
Actual OS file export/reentry and live-window close/shutdown coverage remain.

## Actual macOS file sessions

`examples/drag_drop_desktop` uses only the public Core/Bonsai/Eio API. The Python
harness creates a regular temporary file, supplies its absolute path, then checks
that the source file still exists with unchanged contents. The driver arranges two
320-pixel child windows side by side and posts a continuous system mouse gesture.
It acts only on windows owned by that child and checks the AX hit owner throughout.

The following scenarios have passed locally:

- `--desktop`: source Started and Desktop_offered, then a second-window drop with
  Desktop origin, exact path and unknown directory metadata. The receiver's
  gesture ID is distinct from the original source ID. The source ends Unconfirmed;
  receipt is not treated as a confirmed external copy/move operation.
- `--reenter`: drag out to the second window and back into the original source
  window. The OS offer is observed, and reentry restores the original gesture ID,
  Internal origin and caller-supplied directory metadata. The source ends once
  with Internal_drop.
- `--cancel`: while the OS drag is over the receiver, post Escape. The receiver
  has observed Desktop hover but gets no drop; the source ends Unconfirmed.
  AppKit consumes cancellation, so this does not claim a framework Escape reason
  for an OS-owned session.
- `--remove-source`: Bonsai unmounts the source upon Desktop_offered, and its Edge
  observer confirms the removal. The immutable OS offer still reaches the second
  window. The removed source gets no late terminal callback. A rendered frame and
  the expected source-event count precede clean application shutdown.

Build the example and driver, then select a scenario:

```sh
./scripts/gpuio exec dune build examples/drag_drop_desktop/main.exe
./scripts/gpuio exec cargo test -p gpuio-native --features native-tests --test native_drag_drop --no-run
python3 scripts/test_drag_drop.py --desktop --driver target/debug/deps/native_drag_drop-<cargo-reported-hash>
python3 scripts/test_drag_drop.py --reenter --driver target/debug/deps/native_drag_drop-<cargo-reported-hash>
python3 scripts/test_drag_drop.py --cancel --driver target/debug/deps/native_drag_drop-<cargo-reported-hash>
python3 scripts/test_drag_drop.py --remove-source --driver target/debug/deps/native_drag_drop-<cargo-reported-hash>
```

These tests exercise real AppKit file sessions between windows of the test process.
They do not claim transfer into Finder or another application, Linux graphical
behavior, or confirmed external filesystem operations. The successful file has a
UTF-8 name containing a space. This local filesystem rejected creating a filename
containing byte 0xff with errno 92 (Illegal byte sequence); actual OS transfer of
non-UTF-8 names remains unverified. Existing byte-preservation coverage uses codec
fixtures and GPUI-level injected paths. Pinned GPUI's incoming macOS adapter reads
legacy filename pasteboard strings; do not extrapolate raw-byte OS support from
the injected-path tests.

Live window close and application shutdown *during* a held gesture, plus the
remaining aggregate lifetime review, still require coverage. No production runtime
patch was needed for these scenarios. Final native all-target Clippy, full Dune
build/tests/format, all four OS scenarios and the original AppKit text regression
pass locally. Logs use `drag-os-final-*` in the personal scratch directory.
