# Subtree highlighting — OCH-41 foundation

The current checkpoint implements validated configuration, a windowless native
query kernel, range/run projections, an owned worker pool, retained scope
declarations and observation routing, bounded retained-tree collection and a GPUI
worker service, a shaped-text paint adapter, and mounted ordinary/selectable text
with real GPU painting, cached sources and queued observations. Native document
providers/painting, virtual-list and multiwindow highlighting acceptance, application
performance and the public gallery remain pending. No highlighting capability is
advertised.
OCH-41 remains In Progress. The sections below distinguish each implementation
checkpoint; constructed projections and injected observations do not establish
mounted rendering behavior.

The [design](../design/subtree-highlighting.md) records the pinned GPUIX comparison,
UTF-8 byte/range convention, independent match indices and virtualization offsets,
nested override policy, theme resolution and remaining ownership work.

Implemented:

- Public `Highlight.Query/Range/Appearance/Spec/Config` constructors with abstract
  OCaml types and matching protocol validation. Maximum 16 specs, 4096 aggregate
  ranges, 4096 bytes/query and 256 KiB encoded configuration. Indices preserve
  nonnegative signed 64-bit values; active-index comparison cannot overflow.
- Independent Rust/OCaml configuration bytes, malformed UTF-8/option/Boolean tags,
  every truncation, trailing bytes, semantic bounds and maximum aggregate size.
  Theme defaults multiply accent alpha; explicit colors preserve their alpha.
  Cosmetic changes leave matcher identity unchanged.
- Reusable native compiled queries with a streaming KMP matcher. It recovers
  original UTF-8 positions after scalar lowercasing, supports whole-word checks,
  adjacent chunks and line barriers, and retains only bounded match records.
  It allocates no lowercase source copy or source-sized origin map.
- Shared source-byte, group, chunk and stored-match budgets. Cancelled or
  work-limited requests return a typed failure; storage limits retain exact counts
  and explicitly indicate missing ranges. Empty/infinite empty-chunk iterators
  cannot bypass work limits. These checks do not substitute for scheduler-wide
  memory admission or mounted-scope resource accounting, which remain pending.

Local verification on macOS 14.5 arm64, using the repository-isolated toolchain:

| Command | Result / boundary |
| --- | --- |
| `./scripts/gpuio exec dune runtest test/highlight` | Four expect tests pass. |
| `./scripts/gpuio exec cargo test -p gpuio-protocol --test highlight --locked -j2` | Four configuration/codec tests pass. |
| `./scripts/gpuio exec cargo test -p gpuio-native --lib highlight_search --locked -j2` | Final eight matcher tests pass, including 123244 comparisons against an independent allocating oracle, all scalar chunk splits, 100000 matches with bounded storage, reuse, empty groups/chunks and cancellation. |
| `./scripts/gpuio exec dune runtest` | Full suite passes after configuration/kernel addition. |
| `./scripts/gpuio exec cargo test -p gpuio-native -p gpuio-protocol --locked -j2` | 693 tests pass before the final compiled-query/empty-chunk refinements; those refinements pass the eight-test focused matcher rerun above. |
| `./scripts/gpuio exec dune build @fmt` | Passes. |
| `./scripts/gpuio exec cargo clippy -p gpuio-native -p gpuio-protocol --features native-tests --all-targets --locked -j2 -- -D warnings` | Passes; the range-vector fixture module has a documented narrow allowance for Clippy's single-range-array ambiguity lint. |
| `./scripts/gpuio exec cargo fmt --all --check` | Passes. |

Two initial expectation errors were corrected deliberately: Core's S-expression
printer escapes the non-ASCII bytes in `café`, and a hand-counted range after a
Greek letter used the wrong byte position. Cross-language bytes and matcher
reference comparisons pass; neither failure involved a GUI or user interaction.

At the configuration/kernel checkpoint, next acceptance was ordinary-text grouping and explicit-range validation/projection,
bounded worker admission/cancellation and typed pending/ready/invalid/capacity
observations, retained View/reconciliation/session integration, text/selection and
Markdown/code/diff paints, nested scopes, virtualization offsets, independent
windows and a public gallery find bar. No native interaction, hosted CI or Linux
desktop acceptance is claimed by this windowless checkpoint.

## Immutable projections and worker ownership

`highlight_projection` now maps ordered groups/runs to bounded paint spans with
generational node/fragment keys. It validates source/range boundaries without
flattening rope-backed document snapshots. Native document groups support queries
and are excluded from ordinary explicit-range offsets. Query and range matches
retain distinct ordinals; a match crossing runs counts once. Invalid ranges return
their first spec/range index and typed reason, with no partial result. Output
limits preserve a prefix of complete matches, never a partially painted match.

`highlight_jobs` adds application-pool ownership: at most 128 scope entries,
two running workers and 64 MiB of conservative admission units. It reserves
queued/running/ready/retired sources and potential output, then releases unused
scratch/output credit on worker completion. Charges remain through the last
reader of retired results. Same-source cosmetic changes reuse the same ready
result/epoch; matcher or source changes retire it immediately and cancel old work.
Admission failure also removes old paint. Pool/task/scope-epoch checks reject late
or foreign results; close/drop and abandoned tickets release worker ownership.
The tests execute actual Rust threads, without GPUI or OCaml callbacks.

Final local command for this continuation:

```
./scripts/gpuio exec cargo test -p gpuio-native --lib \
  --test highlight_jobs --test highlight_projection --locked -j2
```

Result: 369 tests pass (350 library, 11 worker integration, eight projection),
with two pre-existing notification tests ignored because they require a separate
isolated `dbus-run-session`; their required release validation is unchanged.
New coverage includes 100000 matches,
32768 paint spans with whole-match truncation, UTF-8 boundaries and separator-only
ranges, rope snapshot retirement, real thread delivery, sparse-result quota
recovery, rapid replacement, invalid/admission-failed updates, foreign completion,
fair dispatch, 128 scopes/two workers, epoch exhaustion and pool/handle shutdown.
Strict all-target native/protocol Clippy with `native-tests` enabled and Rust
formatting both pass on the final continuation. No lint suppressions were added.

No GUI windows were opened. At this worker-pool checkpoint, the OCaml configuration/
protocol was unchanged from the previous checkpoint. Remaining work was the versioned observation envelope,
public retained scope, tree/native-document projection adapters, GPUI executor
and shutdown service, actual painting/selection/nested-scope behavior and gallery
acceptance. The earlier full-suite evidence remains tied to its stated checkpoint.

## Retained declarations and observation routing

Core/Bonsai now expose `View.highlight_scope ~config ?on_update children`.
Reconciliation emits configuration updates, preserves child identities, rotates an
existing observer on config changes and dispatches through the latest accepted
closure. An empty config is an explicit nested override; a missing configuration
is invalid. Native tree transactions validate and account for retained config
atomically. The scope cannot emit semantic Press events.

The bridge appends Kind50, Op57 and Event64 without renumbering existing variants.
Observations carry a positive scope-local epoch and Pending, Ready per-spec
counts, Invalid_range, Capacity or Failed. Both sides validate the payload;
callback admission also validates count/range shape against the current config.
The OCaml bin_prot list reader bounds allocation before consuming counts in wire
order, and malformed observations return a typed decode error. Native session
checks reject stale handlers, closed windows and future tree revisions. The
ordered mailbox includes observation payloads in its existing capacity and window
retirement accounting. No new capability is advertised.

Local macOS 14.5 arm64 verification for this continuation:

| Command | Result / boundary |
| --- | --- |
| `./scripts/gpuio exec cargo test -p gpuio-native -p gpuio-protocol --locked -j2` | 722 tests pass; the same two isolated-D-Bus notification tests remain ignored by this command. Includes three new protocol and three native scope tests. |
| `./scripts/gpuio exec dune build -j2 @all @runtest @fmt` | Full build, tests and formatting pass. Includes eight highlight expect tests, four newly added for observations/reconciliation. |
| `./scripts/gpuio exec cargo clippy -p gpuio-native -p gpuio-protocol --features native-tests --all-targets --locked -j2 -- -D warnings` | Passes. |
| `./scripts/gpuio exec cargo fmt --all --check` | Passes. |

Independent fixtures cover every observation state and the complete scope request
and event envelope. Tests cover all truncations/trailing bytes, hostile counts,
invalid epochs/counts/range indices, aggregate storage bounds, atomic rollback,
optional observers, latest closures, handler retirement, unchanged children,
ordered mailbox overflow and window-close reclamation.

The tests caught and fixed a sequential-decoder bug: Core's `List.init` evaluates
in descending index order, reversing heterogeneous wire counts. The reader now
uses an explicit sequential loop; its dedicated malformed-observation exception
is caught by the event decoder. A hand-written request fixture's SetRoot tag was
corrected against the existing schema. The full build also found two low-level
examples missing explicit cases for Input_observed and Highlight_observed; both
now compile exhaustively. None of these failures involved user desktop activity.

This declaration/routing checkpoint used injected observations. Its remaining
work was retained source collection, bounded GPUI worker dispatch/shutdown, real
observation production, ordinary/selectable/native-document painting and public
gallery acceptance. No GUI windows, hosted CI or Linux desktop qualification were run here.

## Retained collection and native worker service

The retained-tree collector preserves adjacent text groups, empty-text
transparency, nested declaration boundaries and native group source ordering.
It excludes widget metadata/editor values, accepts the mounted adapter's
visibility and installed document text, and rejects Pending/Unavailable document
sources without publishing partial counts. Empty scopes and range-only scopes
avoid unnecessary document work. It bounds all visited nodes, including empty
ones, and checks group/run/source limits before appending. Source comparison
preserves reuse across cosmetic updates while checking group order, generational
keys and document snapshot identity.

The GPUI service now dispatches the owned pool on the real background executor,
wakes the owning window on completion, and cancels a scope on drop/window close.
Application abort, protocol shutdown and platform quit drain workers. Reentrant
cleanup retains worker destruction signals; completion delivery cannot block on
a UI callback. Abandoned tickets preserve explicit Closed state. No idle polling
or synchronous OCaml callback is introduced.

Local macOS 14.5 arm64 checks:

- `./scripts/gpuio exec cargo test -p gpuio-native --lib --test highlight_collect --test highlight_projection --test highlight_jobs --locked -j2`:
  377 tests pass (350 library, six collector, eight projection, 13 job tests).
  Two existing notification tests still require isolated private-bus validation.
- Built `native_highlight_host` using `cargo test -p gpuio-native --test native_highlight_host --features native-tests --no-run --locked -j2` through
  `./scripts/gpuio exec`, then ran Cargo's reported binary under a 35-second
  subprocess timeout. It passes with `GPUIO_NATIVE_HIGHLIGHT_HOST_OK`.
  After one forced Pending draw, completion must repaint the window without
  further manual draws. It verifies cosmetic result identity, latest-request
  epochs/counts, window-close cancellation while a native handle survives, and
  repeated shutdown with two actual dispatched workers. All worker slots,
  reserved bytes, completion buffers and window routes return to zero/empty.
  The windows request no focus, close sequentially and are reaped.
- Strict native/protocol all-target Clippy with `native-tests`, Rust formatting,
  shell syntax and diff checks pass. No OCaml/protocol representation changes in
  this continuation; prior Dune evidence remains tied to its own checkpoint.

The native test is included in required macOS CI and informational Linux GUI
scripts, with required cross-platform test compilation. Hosted execution remains
pending the consolidated milestone run. This local native evidence verifies
worker lifecycle, not actual text highlighting, keyboard/IME or Linux desktop
behavior. The collector's document fixtures supply prepared strings; they do not
prove Markdown/code/diff adapters. Connecting those adapters, mounted cache and
observations, selection-aware/radius-aware paints and the public gallery remains
required before advertising the capability or completing OCH-41.

## Shaped-text paint adapter

`highlight_paint` now resolves prepared matches to colors/radii and paints an
underlay using GPUI's shaped layout. It follows actual soft wraps and alignment,
maps original UTF-8 offsets through truncation, excludes synthetic ellipses, and
leaves native selection decoration on top. Geometry cache and Paint readers retain
the Ready lease and original worker reservation. Query matching never runs in
paint; shape/source changes replace cached geometry.

Local macOS 14.5 arm64 evidence:

- `./scripts/gpuio exec cargo test -p gpuio-native --lib --locked -j2`: all 355
  library tests pass; two private-bus notification tests remain excluded from this
  command. Five new tests cover truncation mapping (including equal-length
  changed display strings), visual-row alignment/wrap boundaries/clipping,
  UTF-8 cluster indices, active offsets near `i64::MAX`, and reservation retention
  until the last retired Paint/geometry reader drops.
- Built `native_highlight_paint` with `cargo test -p gpuio-native --test native_highlight_paint --features native-image-tests --no-run --locked -j2` through
  the isolated wrapper, then executed Cargo's reported binary under a 35-second
  subprocess timeout. Final `GPUIO_NATIVE_HIGHLIGHT_PAINT_OK` passes: real native
  GPU pixels show both wrapped rows, rounded corners, selection above the wash,
  centered text and the highlighted original suffix after start ellipsis. The
  ellipsis itself retains the background. The background window closes/reaps.
- Strict all-target native/protocol Clippy with `native-image-tests`, Rust
  formatting and shell syntax/diff checks pass. New test compilation/macOS
  execution is wired into CI; Linux GUI invocation remains informational.

After adding reservation ownership, one native fixture failed because GPUI drew
its scene before the asynchronous test task prepared its Paint. Preparation now
runs in the scene constructor. The final rerun passes with leased paint data;
this was fixture initialization, unrelated to user desktop activity.

This is real GPU evidence for the independent paint adapter, not public
`View.highlight_scope` acceptance. Mounted state/visibility/document providers,
source invalidation, actual callback production, ordinary/selectable/document
view wiring, stress workloads and the public gallery are still required. No
highlight capability, full script/bidi coverage, hosted pass or Linux desktop
qualification is claimed. Prior full Dune/bridge results remain tied to their
stated checkpoints; no OCaml API or protocol representation changed here.


## Mounted ordinary/selectable text

The production View now owns native scope state and connects retained-tree
collection, worker wakeups, source/config invalidation, shaped paints and queued
HighlightObserved events. Empty nested configurations block outer highlights.
Base/structural visibility is independent of input inertness or modal input gates;
actual visibility changes invalidate collection without needing an OCaml tree
transaction. Cleanup runs after the complete paint cycle rather than immediately
after root render, allowing children created lazily during layout to participate.

Local macOS 14.5 arm64, repository-isolated toolchain:

- `./scripts/gpuio exec cargo test -p gpuio-native --lib --test highlight_collect --test highlight_projection --test highlight_jobs --test highlight_scope --locked -j2`:
  **387 tests pass** (357 library, six collector, eight projection, 13 jobs,
  three scope/observation). Two existing private-bus notification tests are
  excluded from this command. New pure tests verify last-field visibility
  overrides, inert text eligibility and stable structural visibility identities.
- Built `native_highlight_view` using `cargo test -p gpuio-native --features native-image-tests --test native_highlight_view --no-run --locked -j2`
  through the wrapper, then executed Cargo's reported binary with a 35-second
  subprocess timeout. `GPUIO_NATIVE_HIGHLIGHT_VIEW_OK` passes through the real
  production Session/View/Transport path: ordinary/selectable GPU washes, selection
  on top, empty nested barrier, queued exact counts, cosmetic result/epoch reuse,
  no callback on unrelated layout updates, immediate retirement on source change,
  latest rapid update, base and native structural visibility changes, typed invalid
  UTF-8 range failure/recovery, retired result disposal and unmount cleanup.
- Strict native/protocol all-target Clippy with `native-image-tests` passes. The
  test requests no focus, closes its own window on success or assertion failure,
  and drains the highlight service before shutdown. Required macOS CI compiles/
  invokes it; Linux graphical invocation remains informational. Hosted execution
  is still deferred to the consolidated milestone run.

One test teardown initially removed the root without removing its retained child
nodes. The transaction correctly failed validation; explicitly removing those
nodes fixes the fixture and the rerun passes. User keyboard/mouse activity was
not involved. This test uses internal selection state and GPU readback, not
physical keyboard, clipboard or IME validation.

Native documents currently report SourceUnavailable for query collection instead
of publishing counts over raw Markdown. Exact installed fragment providers and
painting, dynamic style visibility, virtual-list/independent-window acceptance,
large workloads and the public gallery remain required before capability
advertisement. No OCaml/protocol representations changed in this checkpoint;
prior Dune/bridge evidence remains tied to its recorded revisions.
