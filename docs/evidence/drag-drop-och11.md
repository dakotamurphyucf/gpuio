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
