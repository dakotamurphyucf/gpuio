# Subtree highlighting — OCH-41 foundation

The current checkpoint implements validated configuration, a windowless native
query kernel, range/run projections, an owned worker pool, retained scope
declarations and observation routing. It does **not** implement native source
collection, GPUI executor/wakeup integration, actual observation production,
painting or gallery acceptance. No highlighting capability is advertised.
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

This is declaration/routing evidence using injected observations. Retained source
collection, bounded GPUI worker dispatch/shutdown, real observation production,
ordinary/selectable/native-document painting and public gallery acceptance remain
pending. No GUI windows, hosted CI or Linux desktop qualification were run here.
