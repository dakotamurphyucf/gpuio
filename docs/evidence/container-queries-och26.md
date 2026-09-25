# Container query evidence (OCH-26)

These are chronological checkpoints. Mounted bridge and local macOS acceptance
are complete; consolidated hosted macOS/Linux gates and merge remain pending.

## Typed rule and codec foundation

Local macOS arm64, 2026-09-25. This checkpoint defines public abstract Branch_id,
Range, Predicate, Rule and Config modules, a pure reference selector, and a bounded
native configuration decoder. It does **not** mount responsive presentations,
change native focus/visibility, deliver selection events or advertise a capability.
Those remain required OCH-26 implementation work.

Half-open width/height ranges select the first matching rule, otherwise the
explicit default. Configurations allow 32 rules and 16 distinct branches with
128-byte UTF-8 IDs. The native decoder limits total standalone configuration bytes
to 4,096 and bounds nested lists and names before allocation. It validates positive
generations, finite/nonnegative/ordered bounds, unique IDs and branch indices.
Public construction rejects invalid inputs before conversion to the wire shape.

OCaml and Rust independently construct the same `container-query.hex` fixture:
default compact, wide at width >=480.25 and height >=200, then tall at height >=600.
Tests verify exact fractional lower/exclusive upper boundaries, height/width AND,
first-match overlap, rule-order changes, default and zero assigned size, invalid
nonfinite/negative sizes, UTF-8/NUL/name limits, branch/rule saturation and generation
validation. Rust also rejects every truncated fixture prefix, trailing data,
malformed UTF-8, oversized declarations before allocation, invalid indices/ranges
and configurations exceeding the total byte bound. Maximum admitted counts decode.

Targeted checks pass through the isolated jobs=2 toolchain:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune runtest -j 2 test/view_api
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -p gpuio-protocol --test container_query -j 2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -p gpuio-protocol --all-targets -j 2 -- -D warnings
```

Three OCaml expect scenarios and three new Rust tests pass. No graphical or Linux
execution is claimed. [The implementation contract](../design/container-queries.md)
describes the planned native layout, identity, observation and lifecycle behavior.


Final foundation checks also pass:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @all @runtest @fmt
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -p gpuio-protocol -j 2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all --check
```

Formatting changes were reviewed; no expect values were promoted. No application
windows were opened by these foundation checks. Hosted and mounted acceptance
remain pending.


## Retained bridge, native selection and public Bonsai example

The next local checkpoint appends kind 33 / operation 38 / event 42, preserving
older wire tags. Tree validation rejects malformed configs, stale generations,
duplicate updates and mismatched branch counts atomically. Config payloads count
against native retained-tree budgets. Session and OCaml event fences reject stale
handles/revisions/configs, invalid sizes/indices, branch/size inconsistencies and
duplicate/out-of-order selection sequences.

`View.container_query` and the Bonsai specialization accept exact supplied
presentations and retain them under stable branch keys. New expect tests prove
identity across rule reorder, callback-only updates without a native transaction,
latest-closure delivery, unaccepted prepare isolation, config fencing and disposal.
Independent full request and selection-envelope fixtures agree in OCaml and Rust.
Native session tests prove tree rollback, branch count checks, event fencing and
zero retained payload after removal/close.

The actual macOS `native_container_query` executable passes:

- Window resize selects without changing the retained tree revision. Below/equal/
  above native thresholds choose the expected branch; resizes within the same
  branch produce no repeated selection observations.
- Buttons retain native identity while hidden. Losing visibility removes focus
  and denies input; the new branch accepts native pointer activation. Hidden
  queries settle to idle, and removal releases controls and query state.
- Nested queries select from their own assigned inner size, not the window size.
  Their controls retain identity through outer-branch switches.
- An animation inside a hidden alternative pauses active time, stays idle and
  resumes its previous geometry. Query selection also works in a deferred modal
  dialog; hiding its outer branch removes its modal eligibility.

The public `examples/container_query/` program uses ordinary Core/Bonsai/Eio APIs.
Its `--self-test` passes real FFI resizing, retention of a hidden Unicode editor
draft, focus denial for the hidden editor, silent same-branch resize with no new
tree commits, two Bonsai branches remaining active through native selection, and
both lifecycle deactivations on scoped window shutdown.

The first public run requested a background window and timed out. Diagnostics
showed both editors initialized and one accepted commit, but zero rendered
revisions and zero selection events. On this macOS desktop the occluded window
had not painted. The self-test now requests focus; it passes and closes itself.
This is actual foreground paint/command validation, not a claim of background
rendering or physical keyboard/IME coverage.

The existing native controls suite and baseline/advanced animation executables
also pass locally after adding query visibility. Their focus, overlay, tooltip,
menu, palette, progress, toast, pointer and animation regression markers remain
passing. The full Dune build/expect/format check and combined native/image/canvas
all-target Clippy pass at the initial mounted checkpoint. Final validation of
subsequent initialization refinements is recorded below.

Native/public query commands are included in macOS CI and the informational
X11/Wayland scripts. No hosted or Linux execution is claimed yet. No capability
is advertised. Virtualized retention, deeper accessibility/composition/capture,
scale/fractional and observer/lifetime cases, aggregate workloads and final gates
remain OCH-26 work.


Final mounted-checkpoint validation passed on local macOS after the initial-hidden
animation refinement. Before first query layout, inactive branches now suspend
legacy and advanced motion; the native test delays the first paint by 500 ms and
proves the animation still starts at its initial value. It subsequently advances
only during visible time.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @all @runtest @fmt
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --workspace --locked -j 2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -p gpuio-native --all-targets --features native-tests,native-image-tests,native-canvas-tests -j 2 -- -D warnings
./scripts/gpuio exec cargo fmt --all --check
./scripts/gpuio exec _build/default/examples/container_query/main.exe --self-test
```

All commands exited zero. The final native query/nested checks and both legacy and
advanced animation executables passed; the rebuilt public self-test passed again
and closed its window. This checkpoint does not advertise a query capability or
claim the broader remaining acceptance, hosted CI, or Linux GUI validation.


## Expanded local native acceptance

The native query executable now also exercises the following actual macOS paths:

- A retained 32-row list: actual scrolling to row 20 hides the first query's input
  eligibility without destroying its button; returning does not replay an unchanged
  selection. Offscreen config/observer changes wait for paint. Reordered branch
  IDs preserve native identity, a detached observer receives no events, and a new
  observer receives future selections only. All list/query/native state disposes.
- Fractional assigned widths below/equal/above 300.5, with GPUI scale overrides of
  1, 1.5, 2 and restoration of the display scale. Predicates use assigned logical
  sizes after device-pixel snapping: a declared 300.5 at scale 1 assigns 300. The
  test checks exact observed size and branch, not a pre-snapping expectation.
  This is a real GPUI layout test using its scale override, not physical movement
  between monitors. The macOS CI command enables the existing native-image-tests
  feature to include upstream test-support and these scale cases.
- AppKit accessibility traversal exposes only the selected native editor. Real
  NSTextInputClient marked/committed text stays with its original retained editor
  through resize, hidden focus is rejected, fallback input modifies neither draft,
  and explicit return/focus permits editing the original owner. This does not
  claim a walkthrough of every OS input method or a VoiceOver usability review.
- A held native pointer capture ends with Hidden cancellation when its branch
  disappears; a late mouse release does not complete the cancelled gesture.

Two test-harness corrections were necessary: the transaction helper must dispatch
accepted list actions, as the production host already does; AX traversal must
start at the window's content view to activate/access the native accessibility
subtree. Neither required a production adapter change. The first-paint motion
clock test now advances its clock in the same host update as admission, preventing
an intervening paint from making the test nondeterministic.

The debug workload lays out 256 visible queries with 32 rules each and 512 retained
alternatives, changes all branches through four real window resizes, dispatches
a native button click, verifies unchanged tree revisions/payload, confirms no
selection events for unobserved queries, settles idle and returns retained payload
to zero on removal. One measured run used 573,664 retained payload bytes:
admission 87.2 ms; initial frame barrier 61.4 ms; resize barriers 91.4–95.9 ms;
native injected input dispatch 14.0 ms; disposal 3.4 ms. These are debug wall times
including scheduling, not frame CPU time, OS input latency, total RSS, or universal
performance guarantees. Changed selections currently synchronize shared managers
and can revisit the tree; this cost remains explicit for the integrated showcase.

Capability bit `8589934592` is now enabled (aggregate `17179869183`). Independent
OCaml/Rust Hello expectations pin `0001fcffffffff03000000`. Historical evidence
above describes earlier checkpoints and is not rewritten to imply this bit existed
then. Hosted macOS/Linux gates and merge remain pending.


## Final local capability validation

All of the following exited zero on macOS arm64 with the capability enabled:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @all @runtest @fmt
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --workspace --locked -j 2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -p gpuio-native --all-targets --features native-tests,native-image-tests,native-canvas-tests -j 2 -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -p gpuio-native --features native-tests,native-image-tests --test native_container_query --no-run -j 2
./scripts/gpuio exec _build/default/examples/container_query/main.exe --self-test
./scripts/gpuio exec cargo fmt --all --check
```

The rebuilt native query executable was run with a 60-second subprocess deadline
and passed all seven marker families, including scale, lifecycle, AX/IME and
workload scenarios. Its final debug resize barriers were 91.2–100.1 ms and input
dispatch 13.7 ms, consistent with the measurement limitations above. The public
self-test passed and closed; no owned native test windows/processes remain.
No hosted run or Linux GUI acceptance is claimed. The accepted local scope is
complete; the ticket remains in progress pending consolidated CI and merge.
