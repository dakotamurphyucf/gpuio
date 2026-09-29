# Subtree highlighting — OCH-41 foundation

The current checkpoint implements validated configuration, a windowless native
query kernel, range/run projections, an owned worker pool, retained scope
declarations and observation routing, bounded retained-tree collection and a GPUI
worker service, a shaped-text paint adapter, and mounted ordinary/selectable text
with real GPU painting, cached sources and queued observations. Installed native
code/diff/source-mode pages now have rounded background painting and revision/page
fences. Prepared Markdown text fragments now have focused native GPU evidence.
Declared custom text and dynamic image placeholders are also implemented.
Virtual-list and multiwindow highlighting acceptance,
application performance and the public gallery remain pending. No highlighting capability is
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


## Immutable installed-page source primitive

`Source::document_slice` validates an immutable document snapshot interval and
streams its rope chunks directly. Match offsets are local to that interval;
NativeDocument groups remain excluded from ordinary explicit-range offsets.
Changing either page or installed snapshot identity invalidates source equivalence,
even when a newly published revision has identical bytes. The original document
store charge survives retirement until the final slice reader is dropped.

Local macOS 14.5 arm64: the isolated Cargo command
`cargo test -p gpuio-native --lib --test highlight_projection --test highlight_jobs --test highlight_collect --locked -j2`
passes **386 tests** (357 library, ten projection, 13 jobs, six collector); two
existing private-bus notification tests are excluded. The two new projection tests
cover page-local matching across rope content, outside-page exclusion, page and
same-content revision identity, invalid/reversed/out-of-bounds/UTF-8 endpoints,
empty pages and release of the original store reservation. Strict all-target
native/protocol Clippy with native-image-tests and Rust format/diff checks pass.

No native window test is claimed for this source primitive. Installed document
collection, rounded editor backgrounds and Markdown fragment/paint adapters remain
pending; the mounted provider still reports SourceUnavailable for nonempty native
document queries. No protocol, capability or OCaml API changes.


## Installed code/diff/source-mode highlighting

The production document presenter now supplies its exact installed page to the
scope collector. Initial preparation reports Pending, collapsed content contributes
no text, and streaming continues matching the still-displayed installed revision
until its replacement is installed. A shared native document identity invalidates
source stamps and queued observations on page/content changes. Document painters
verify their fragment key and exact snapshot interval against the prepared result.

The small GPUI Base adaptation adds immutable, bounded read-only range-background
owners. It validates bytes/radii, clears them on text edits, retains the owner
through frame painting and uses shaped visible buffer-line offsets. Rounded washes
paint above syntax backgrounds and below native search/selection. Prepared native
Paint readers retain the original Ready/store reservations. No synchronous OCaml
callback or query work runs in paint.

Local macOS 14.5 arm64, isolated repository toolchain:

- `native_highlight_document`, built with `native-image-tests` and executed under
  a 40-second subprocess timeout, passes `GPUIO_NATIVE_HIGHLIGHT_DOCUMENT_OK`.
  Actual GPU pixels verify code and diff washes, reduced painted area for radius8
  versus radius0, selection precedence, unchanged cosmetic epoch, collapse/expand,
  native page changes, old installed revision during pending replacement, new
  streamed counts/pixels, unmount and final prepared-background owner disposal.
- The first background-only run passed rendering/lifecycle checks. Adding selection
  exposed the native editor's existing active-window-and-focus guard. The final
  fixture briefly activates/focuses its window for that check and passes. Selection
  is set through the native editor bridge, not physical keyboard/IME input. All
  windows close and processes are reaped.
- Existing `native_highlight_view` and `native_document` binaries rebuilt against
  this integration also pass. The latter retains code/Markdown/table/fence/image,
  diff, Unicode selection/streaming and lease teardown coverage. Its diagnostic
  layout/paint sample is 16914 microseconds and peak process RSS 69500928 bytes;
  these are observations, not the milestone's application performance budgets.
- `cargo test -p gpuio-native --lib --test highlight_projection --test highlight_collect --test highlight_jobs --test highlight_scope --locked -j2`
  through the wrapper passes **389 tests** (357 library, ten projection, six
  collector, 13 jobs, three scope/observation); two existing private-bus notification
  tests are excluded from this command. Strict all-target native/protocol Clippy
  with `native-image-tests`, Rust format, shell syntax and diff checks pass.
- The updated Base patch and checksum reconstruct exactly from the pinned archive,
  comparing all committed vendor files and excluding generated Cargo.lock. The
  patch attribute allows the single-space blank context line used by unified diffs;
  source whitespace/formatting checks remain unchanged. New native test compilation
  and macOS execution are in CI; Linux graphical invocation is informational.

No hosted or Linux GUI acceptance is claimed. Rendered Markdown still reports
SourceUnavailable until its immutable fragment/paint adapter is implemented.
Virtual-list/independent-window and broad wrap/tab/fold/scroll behavior, application
performance/resource budgets and public gallery acceptance remain open. No new
protocol, OCaml API or capability advertisement in this checkpoint.


## Prepared Markdown text integration

The bounded parser now creates an immutable structural fragment table before
layout or painting, with 4,096-fragment/64-KiB bounds. Paragraph/heading formatting
stays contiguous; code blocks and table cells are separate groups. Inline-flow
splits carry the original fragment's byte range. Matching uses `DocumentText`,
fenced to both the installed document snapshot and prepared text allocation.
Original document-store reservations and prepared-match owners stay retained by
their readers. Pending parsing keeps the old AST and its displayed fragments.

Base's `TextBackgrounds` validates the complete layer count, UTF-8 endpoints,
radius and aggregate 32,768-range limit. Installation checks the prepared table
identity; replacement clears the prior owner. Inline painting verifies the exact
fragment slice, then paints formatting backgrounds, rounded query washes, glyphs
and native selection in order. It runs no queries or OCaml callbacks. Cosmetic
updates do not reset measured list layout, scrolling or selection.

Local macOS evidence through the isolated toolchain:

- `native_highlight_document` now checks actual GPU pixels and counts for headings,
  matches crossing bold styling and inline-code font/size splits, fenced code,
  table cells and wrapped paragraphs. It also checks rounded corners, selection
  precedence/restoration, old installed fragments during streamed parsing,
  replacement identity, rejection of an explicitly reinstalled old-AST provider,
  collapse/expand and final Markdown background-owner disposal, alongside the
  earlier source/code/diff cases. Each final binary ran under a 60-second watchdog;
  all windows closed and processes were reaped.
- The first expanded run revealed a fixture error: rebinding an observer while a
  replacement parse is pending may correctly report the old installed document.
  The fixture now waits for the requested revision and binds a fresh observer
  after draining acknowledged older samples. The corrected run passes. Selection
  is driven through native state APIs; this is not physical keyboard or IME evidence.
- Rebuilt `native_highlight_view` and `native_document` also pass. The latter
  retains existing safe-image, table/fence, Unicode selection/streaming and lease
  teardown coverage. Its diagnostic layout/paint sample is 17278 microseconds
  and peak process RSS 68517888 bytes; these are not application budget acceptance.
- The full library/projection/collector/jobs/scope command passes **394 tests**:
  361 library, 11 projection, six collector, 13 jobs and three scope. Two existing
  private-bus notification tests remain excluded from this command. New checks
  cover ordered pre-paint text, Unicode/code/table runs, all 200 unpainted blocks,
  ownership after prepared-AST disposal, distinct AST/revision identities,
  original store retention, opaque boundaries and malformed background metadata.
- Strict all-target native/protocol Clippy with `native-image-tests`, workspace and
  changed-vendor formatting, and diff checks pass. The Base patch reconstructs
  exactly from the pinned archive, excluding generated Cargo.lock. Its SHA-256 is
  `04d4d8b3444bd90fc8de184125d81bffb18b5d6e3726b61288ad9f8a829b6038`.

Custom/image renderers still require an explicit displayed-text contract. Their
copy/AX labels are not reliable glyph sources; the prepared table reports opaque
nodes and the mounted adapter returns SourceUnavailable for such documents,
without publishing partial Ready counts. This remains required follow-up work.
Broad tabs/folds/scroll/bidi, virtual-row and independent-window behavior, public
gallery and application resource/performance acceptance remain open. No hosted,
Linux GUI, new capability advertisement or full highlighting acceptance is implied.


## Declared custom text and image-resource transitions

`MarkdownPresentation` now distinguishes framework-painted text, explicitly
non-text native content and an unspecified opaque renderer. Prepared custom
occurrences have separate identities, structural match positions and immutable
projection data. Text uses passive glyph elements inside the existing object
selection/link wrapper. Atomic selection now paints above child backgrounds.
GPUIO uses this contract for literal HTML and image placeholders; loaded images
contribute no glyphs. Their copy/AX alternatives remain separate from visible text.

Image-resource changes rebuild the bounded installed AST's displayed projection
without parsing or matching, preserve the document revision and invalidate the
scope. Cached extension handles prevent a new projection on every frame. Opaque
third-party renderers still report SourceUnavailable until they declare their
presentation; no glyph representation is guessed from an accessibility label.
The native Markdown paint cache has no additional projection owner when no washes
exist, while a zero-match fragment does not suppress other fragments' highlights.

Local macOS 14.5 arm64 evidence:

- The extended `native_highlight_document` passes actual GPU checks for literal
  HTML blocks and inline attributes, image placeholders, selection precedence and
  restoration, and two identical placeholders with separate normal/active colors.
  Registering a real PNM asset through the session changes the count from two to
  zero and paints the decoded blue image. Removing the resource restores two
  matches. The installed document revision stays unchanged; the displayed table
  changes. Select-all copy remains `aaa aaa`, independent of the visible placeholder
  syntax. Empty-scope cleanup and explicit asset release/image-worker shutdown pass.
- Rebuilt `native_highlight_view` and `native_document` also pass. Final binaries
  ran sequentially under 60-second watchdogs; all windows closed and processes were
  reaped. Existing document diagnostics report 17338 microseconds layout/paint and
  70123520 bytes peak RSS; these are not application performance-budget acceptance.
- The full native library/projection/collector/jobs/scope command passes **395
  tests** (362 library, 11 projection, six collector, 13 jobs, three scope), with two
  existing private-bus notification tests excluded. New unit checks distinguish
  declared text from copy text, verify non-text/opaque behavior and reject oversized
  renderer text. Strict all-target native/protocol Clippy with native-image-tests,
  workspace/changed-vendor format and diff checks pass.
- The Base patch reconstructs exactly from the pinned archive, excluding generated
  Cargo.lock. SHA-256:
  `b4d09f1b3bda02575b7db9beb63d339babb87c4951745e814427166bd4e4c559`.

The expanded run caught and fixed an overly strict cleanup condition: a fragment
with no matches has no paint, which must not discard other fragments' valid washes.
A subsequent fixture correction adds the required handler rebinding when changing
to an empty scope configuration. The corrected final runs pass.

Selection checks use native APIs, not physical keyboard/IME input. Complete partial
selection of custom blocks, rich-object accessibility, broad script/bidi/scroll,
virtual-row and multiwindow acceptance, public gallery, performance/resource budgets
and release gates remain required. No hosted CI, Linux GUI or new capability claim.

## Virtual rows and independent windows

The extended `native_highlight_view` runs two background windows through the
production retained View, shared transport and native matching service. Both use
the same node and handler IDs. Actual GPU colors, queued window-tagged counts,
result identity and epochs verify that a source update in one window leaves the
other window's result unchanged.

One window then installs a managed list with 100,000 logical rows and 12 supplied
row trees. Jumping to row 50,001 shows unloaded placeholders, clears highlight
pixels and reclaims the previously painted row scopes. Rebinding the same native
nodes to the distant logical page gives new counts and normal/active colors using
the updated match offsets. The other window retains its prepared result throughout.
Closing the first window immediately after a large text update releases its scope
handles; no late observation from it appears during the subsequent survivor draws.
The surviving window then accepts another source update and paints the new result.

Local macOS 14.5 arm64 validation:

- Build: `./scripts/gpuio exec cargo test -p gpuio-native --features native-image-tests --test native_highlight_view --no-run --locked -j2`.
- The resulting native binary passes the existing ordinary/selectable suite and
  the new lifecycle suite under a 60-second watchdog. Both windows close and the
  process is reaped. Checks use native APIs; no OS keyboard/IME interaction is claimed.
- Strict native/protocol all-target Clippy with `native-image-tests` passes.
  Workspace formatting and diff checks pass. Production code and the Base fork
  patch are unchanged by this fixture extension.

Initial failures were fixture errors: computing window bounds from AsyncApp
instead of App, then attempting sparse node allocation. The corrected fixture uses
the production dense, generation-checked node allocation contract. Neither failure
provides evidence of interference from the owner's desktop interaction.

This is functional lifecycle evidence, not a 100k-row performance budget or a
deterministic worker-cancellation race test. Broad scrolling/tab/fold/bidi,
application resource budgets, public gallery and release acceptance remain open.

## Public OCaml gallery

Component Studio's **Find & highlight** page uses only public Core/Bonsai/Eio APIs.
The native editor supplies the query snapshot; case/whole-word/enabled controls
rebuild a validated configuration. A displayed observation is paired with its exact
configuration, so the prior query's Ready count is not presented as the new query's
answer. Selecting a match changes appearance metadata; changing the query or its
case/word semantics resets the cursor. When collapse removes matches, the app
clamps the cursor to the remaining count. Selection does not automatically reveal
an offscreen match.

The scope contains ordinary/selectable text, a match crossing styled leaves, and
a scoped Markdown document with bold/inline-code/fenced-code content. An empty
nested scope excludes a note. A separate explicit UTF-8 range highlights `世界`.
One idempotent append grows the notebook; page departure clears observations/cursor
and releases document/editor resources. Toggle preferences survive page visits;
the query and notebook are fresh on return.

Local macOS 14.5 arm64 evidence:

- `./scripts/gpuio build examples/gallery/main.exe` passes using the isolated switch.
- `python3 scripts/test_gallery.py --section highlighting --images scratch/agents/root-20260928-m7/highlight-gallery-images`
  passes actual AX query edits, case/whole-word controls, empty/oversized query
  recovery, pause/resume, next/previous wrap, bounded append, native document
  collapse/expand, theme/editor retention and page remount. Counts change 7→9→4;
  collapsing at selected match 9 clamps to 4 and subsequent stepping remains valid.
  Final app closes and is reaped. AX editing is not physical keyboard/IME evidence.
- Dark/alternate-theme screenshots were generated; inspection of the dark image
  found an existing inline-code contrast defect. The native Markdown adapter now
  sets inline-code background explicitly to its document palette. Changing only
  Base's general code background left the default inline style's light color intact.
  The corrected dark screenshot shows readable code and the query wash.
- Rebuilt `native_highlight_document` passes actual code/diff/Markdown GPU washes,
  selection precedence, native paging/collapse, streaming replacement and ownership
  disposal under a 60-second watchdog. Its window closes and process is reaped.
- Gallery/highlight expect targets, strict all-target native/protocol Clippy with
  `native-image-tests`, formatting, catalog structure and diff checks pass.

The focused gallery check is integrated into the combined `--section all` command;
the new combined 23-section run is still pending. Broad script/bidi/scroll/tab/fold,
application performance/resource, installed consumers and consolidated release
gates remain open. No highlighting capability or full release claim is added.

## Reordered shaped text

The native painter test reproduced an RTL defect: CoreText shaped `אבג` with
source indices 4, 2, 0, and the existing endpoint lookup painted no wash for byte
range 2..4. The same endpoint assumption existed in ordinary, Markdown and source
document background painting.

The shared Base `ReorderedTextGeometry` now maps source-cluster intervals to visual
glyph cells. Matches can emit disjoint spans, which are clipped to visual wrap rows
and then aligned. A logical-index binary search avoids rescanning preceding glyphs
for every match. Ordinary text and source editors retain the index with their
shaped layout; Markdown builds it once per line per paint pass, outside the range
loop. Monotonic text retains its previous path. Native text shaping, matching,
count ordinals, caret/selection and accessibility behavior are unchanged.

Local macOS 14.5 arm64 evidence:

- Extended `native_highlight_paint` passes actual GPU cell checks for the middle
  Hebrew scalar, a whole Hebrew word, a mixed English/Hebrew logical range with
  an unselected visual gap, an Arabic middle scalar, wrapped Hebrew and center/right
  alignment. Existing wrapping, rounded corners, selection precedence and original
  source-byte start-ellipsis checks also pass.
- Extended `native_highlight_document` passes the middle Hebrew query in read-only
  code and Markdown adapters, plus its existing code/diff/Markdown, custom/image,
  selection, paging/collapse, streaming and owner-disposal regression. Native test
  binaries ran sequentially under 60-second watchdogs; windows closed and processes
  were reaped. These are native API/GPU checks, not physical keyboard or IME evidence.
- The native library and highlight projection/collector/jobs/scope/geometry command
  passes **398 tests** (362 library, 11 projection, six collector, 13 jobs, three
  scope, three geometry). Two private-bus notification tests remain excluded.
  New geometry fixtures cover visual clipping, disconnected ranges, whole-cluster
  treatment and the monotonic path without an additional per-glyph index.
- The Base patch reconstructs exactly from the pinned archive, excluding generated
  Cargo.lock. SHA-256:
  `0c2cc2e45384a01e33e9b66da9eea2d1faec77d95988c74805e9a7915a013d9e`.
- Strict all-target native/protocol Clippy with `native-image-tests`, workspace and
  changed-vendor formatting, and diff checks pass.

The tested scripts/cases do not imply exhaustive typography, bidi caret/selection,
screen-reader or Linux desktop acceptance. Broader scrolling/tab/fold integration,
application-scale CPU/memory budgets, combined gallery/consumer checks and the
remaining release gates stay open. No new capability advertisement.

## Viewports and retained structural visibility

The expanded `native_highlight_view` now checks a production scroll container,
retained TabPanel/disclosure content and a responsive ContainerQuery under one
highlight scope. Native APIs drive layout/visibility; these fixtures do not send
OS keyboard or pointer input.

- A viewport starts on one highlighted row, scrolls through a middle region with
  no matches, and then reveals the distant highlighted row. Actual GPU pixels
  verify clipping and alignment. The prepared matching result stays identical,
  and scrolling publishes no new count/epoch observation.
- Editing offscreen text changes the logical count while keeping its wash clipped
  and preserving scroll position. Scrolling back reveals the updated matches.
- Switching retained tab visibility changes the count. Editing the hidden tab
  reuses the visible result and emits no highlight observation; switching back
  exposes its updated text. A disclosure's closed body contributes no matches or
  painted washes, and reopening restores them.
- Resizing the real native window across a responsive threshold changes the count
  and epoch while the retained tree revision stays unchanged. Resizing within the
  same branch preserves the result and emits no highlight observation.

The expanded `native_highlight_document` also checks source-editor scrolling with
tab/Unicode-prefixed text and two distant matches. Vertical scrolling hides both
washes in the middle; at the bottom, a horizontally distant match remains clipped
until horizontal scrolling reveals it. The prepared background owner is reused,
the installed-page count stays constant, and returning to the origin restores
the first wash. Replacing Markdown releases its old provider, and unmounting the
final scrolled source editor releases the current background owner.

Local macOS 14.5 arm64 validation:

- `./scripts/gpuio exec cargo test -p gpuio-native --features native-image-tests --test native_highlight_view --test native_highlight_document --no-run --locked -j2`
  builds both suites. Their final binaries pass sequentially under 60-second
  watchdogs, including the earlier multiwindow/100k-row and document regression
  checks. All windows close and processes are reaped.
- Strict native/protocol all-target Clippy with `native-image-tests`, workspace
  formatting and diff checks pass. This checkpoint changes test coverage only;
  the production code and Base fork patch remain unchanged from the RTL fix.
- The first run passed scrolling/tabs/disclosures but rejected the responsive
  fixture because ContainerQuery nodes require an empty text payload. Correcting
  that fixture produced the final passing run; no bridge validation was relaxed.

These are functional clipping/visibility checks, not application performance
budgets or physical wheel/keyboard/IME acceptance. Animated navigation transitions,
native diff hunk-fold byte mapping, broader script/input matrices, combined gallery,
installed consumers and consolidated release gates remain open.

## Animated navigation

`native_highlight_view` now also mounts a two-page NavigationStack under the
production highlight scope. A forward slide changes the logical count from one
to two. GPU readback confirms that both colored pages are on screen and every
highlight pixel lies over the selected page, with no wash on the outgoing page.
Editing the outgoing page preserves the matching result and emits no observation.
Reversing the slide before completion exposes that page's three updated matches,
advances the epoch, and removes the other page's washes while both pages still
paint. Reduced motion settles directly to the selected page. Unmounting during
another active transition releases both scope and presenter owners, clears
highlight pixels and emits no later highlight observations.

Local macOS 14.5 arm64: the feature-enabled `native_highlight_view` target builds
and its binary passes under a 60-second watchdog, including the existing
ordinary/selectable, multiwindow, virtual-row and visibility cases. All windows
close and the process exits. Strict native/protocol all-target Clippy with
`native-image-tests`, workspace formatting and diff checks pass.

The first run passed forward/reverse/reduced-motion checks, then rejected an
invalid fixture operation: a nonempty NavigationStack cannot select `None`.
That step was removed; the successful final fixture obeys the existing selection
contract. No production code or native validation changed. This is background
rendering evidence, not physical input, accessibility or application performance
acceptance. Dynamic state-style visibility, native diff hunk-fold mapping and the
remaining catalog/consumer/CI/release gates stay open.
