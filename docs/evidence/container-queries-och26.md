# Container query evidence (OCH-26)

These are chronological checkpoints. The mounted bridge and initial local
macOS scenarios now work; broader acceptance and hosted gates remain pending.

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
