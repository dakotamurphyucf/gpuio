# Milestone 07 hosted validation follow-up

Latest result: [run 37236900248](https://github.com/dakotamurphyucf/gpuio/actions/runs/37236900248)
at `1cd5de9622a2b3841d63f1ef85e928b2373f78df` is terminal with both jobs failed.
The earlier missing TestAppContext guard is resolved. macOS passed unit tests and
workspace lint, then failed the independent table native-test build because its
optional observation wrapper changed the row type passed to `finish_row`. Linux
reached the feature-enabled native library tests and aborted on a sidebar fixture
stack overflow. Later gates remain unqualified.

## Current follow-up

The table now applies `.test_support()` after the delegate finishes its concrete
`Stateful<Div>` row, before accessibility decoration. That preserves the delegate
contract with and without the optional `Observed` wrapper. The exact independent
CI build and strict all-target table lint pass locally. The resulting native
`native_table` executable also exits zero under a 120-second process-group bound:
four sampled positions across 100,000 logical rows and 64 columns, keyed selection,
gesture/event identity, anchor preservation and entity release pass. These are
native fixture/layout/dispatch checks, not external OS keyboard or full-history
performance acceptance. Evidence: `table-observed-{build,clippy,native}-001.log`.
Hosted revalidation remains pending.

A subsequent local check of the plain `native-tests` configuration found four
TestPlatform modules guarded by that feature even though it does not enable GPUI
test support: editor frame, input content, menu observations and split-button
view tests. Their guards now use `native-image-tests`, consistent with the other
TestPlatform modules and the required feature-enabled library suite. The tests
remain required there. Plain, image-only and canvas-only strict all-target native
Clippy now pass locally. The final combined library run still reports **919
passed, two existing private-bus skips**; no tests were dropped from that required
suite. Dune/Rust formatting and the workflow's YAML/embedded Python syntax checks
also pass. Logs use `native-basic-clippy-002.log`, `native-image-clippy-001.log`,
`native-canvas-clippy-001.log` and `native-final-feature-tests-001.log` in the local
session directory.

The failing Linux fixture is
`host::sidebar_labels_view_test::public_sidebar_labels_retain_focus_paint_once_and_route_one_activation`.
It passes locally on macOS with an explicit `RUST_MIN_STACK=2097152`; this neither
reproduces nor fixes Linux. A conditional CI diagnostic captures a default-stack
GDB backtrace and then compares an 8 MiB run. It leaves the required suite and its
stack setting unchanged. Each probe has a 90-second process-group limit so a
debugger timeout cannot leave its inferior running. The original required failure
remains a failure regardless of the diagnostic outcome. Cause investigation is
open; no larger-stack production policy is inferred.

Logs: `ci-macos-002.log`, `ci-linux-002.log`, `sidebar-stack-2m-001.log` in
`scratch/agents/root-20261004-resumed/`.

Prior checkpoint: [run 37234888418](https://github.com/dakotamurphyucf/gpuio/actions/runs/37234888418)
at `4453cbb1573533288e0ba9af303f10efe00b904c` failed on both macOS 15 and
Ubuntu 24.04 during the OCaml/Rust tests step. Both platforms completed bootstrap,
formatting and the build. Later acceptance steps were skipped; this is not a
current green result.

## Default-feature compilation repair — 2026-10-04

Both jobs report the same compile error in `desktop_host.rs`: the scrollbar
correlation test refers to `gpui::TestAppContext`, which is unavailable in the
default-feature workspace test build. The test now uses the existing
`native-image-tests` feature guard, matching other TestPlatform tests. The next
command in `scripts/gpuio test` explicitly enables that feature and runs library
tests on both platforms without OS windows; the test remains part of required CI.
The pure scrollbar policy tests remain in the default build.

Local default-feature compilation also exposed unused carousel snapshot helpers
whose callers already require that feature. Their guards now match the callers,
without suppressing warnings or changing production code. Local macOS 14.5 arm64 validation:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --workspace --offline --locked -j 2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-image-tests,native-canvas-tests --lib --offline --locked -j 2
```

Both commands exit successfully: the default workspace reports 1,386 passed and
13 ignored across 260 reports; the feature-enabled native library reports 919
passed and two existing private-bus skips. The scrollbar correlation test is
explicitly present and passes in the second run. The default run began before
the helper-guard cleanup and reported two dead-code warnings; subsequent `cargo clippy --workspace --all-targets --offline --locked -j 2 --
-D warnings` through the repository wrapper passes and validates that cleanup
separately. Cargo retains an upstream `block 0.1.6` future-incompatibility notice;
this is not a first-party warning or a claim of future compiler compatibility.
No hosted rerun has yet validated this repair. Local logs are in `scratch/agents/root-20261004-resumed/` as
`default-tests-001.log` and `native-feature-tests-001.log`.

## Earlier hosted checkpoints

Run **36791905054** is terminal, with Linux success and macOS cancelled after two
failed steps. See its final-result section below. Later local grid/Form/avatar
changes remain outside that older run's coverage.

[Run 36781947346](https://github.com/dakotamurphyucf/gpuio/actions/runs/36781947346)
tested `afb4bba48892b25527db158d533f92eb1fdcaad6`. Linux completed successfully;
macOS failed seven steps. This run predates the Settings composition/reveal
repairs and subsequent catalog work. It does not validate those changes.

## Display-scale assertions — locally repaired

Two failures reproduce exactly on local macOS 14.5 arm64 when the native test
fixture overrides GPUI's scale to 1. This exercises real native layout and GPU
readback; it does not change or qualify the physical desktop's display mode.

- Passive link animation expected 112.5 logical pixels at 200ms, but native
  layout returned 112. The pinned layout snaps to device pixels, with half ties
  toward zero. The test now compares the analytic tween width after device-pixel
  snapping, retaining its 0.1 logical-pixel tolerance. At 2×, 112.5 remains exact.
- The dashed border test required a pure-black sample in every gap. On the
  rounded panel's one-device-pixel left edge, successive three-pixel samples
  had valleys of 61..119, shoulders of 192..133 and peaks of 255. The one-pixel
  gaps crossed sample centers and remained visibly antialiased. The check now
  accepts a midpoint-or-darker RGB sample only for one-device-pixel strokes;
  thicker gaps still require near-black. Every edge still needs multiple bright
  and gap samples; solid and absent edges retain their original strict checks.

Both failing-before cases and corrected suites ran locally. The 72-case border
matrix and complete composed-link suite pass with `--scale-one` and the normal
Retina scale. No production renderer change was needed for these two failures.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native --features native-image-tests --test native_border_style --test native_link -- --scale-one
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native --features native-image-tests --test native_border_style --test native_link
```

Each invocation used a 180-second process-group watchdog and closed/reaped
normally. Local diagnostics are in the ignored per-agent scratch directory;
the original hosted logs remain attached to the linked run.

## Document scroll geometry — locally repaired

The document failure reproduces locally. After a request for `(−70, −10000)`,
the stored editor offset was already clamped to `(0, −2138)`, with an 800 × 300
input viewport. However, the first frame had neither source bounds for the last
diff header nor any visible header actions. The deferred request was used for
row selection and painting before the stored scroll offset was clamped.

The Base adapter now clamps deferred caret/text offsets against the current
layout dimensions. Source-row selection also bounds the request against current
content and the configured empty bottom area; completion ghost rows can extend
final scroll height without excluding earlier source rows needed for shaping.
The stored-offset and layout paths share the same horizontal/vertical clamp.
Ordinary cursor following and auto-grow policy remain unchanged.

The native regression checks negative and positive overscroll on the first frame,
source/header alignment, absent offscreen actions, zero horizontal offset for a
short source, and identical geometry on the next frame. It retains the existing
stale-action, focus and disposal checks. The complete document, editor and table
host suites pass locally on macOS 14.5 arm64:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native --features native-tests --test native_document --test native_editor --test native_table_host
```

The invocation used a 180-second process-group watchdog. Document validation
builds native layout/paint scenes; it does not establish physical presentation.
Its background window can appear blank. This is separate from the unresolved
[public-gallery startup observation](window-startup-och17.md).

The cumulative Base patch SHA-256 is
`56d0526fccf8afb11c2510827d7cf76f39cae05d5efae56eb93a58dc79a39ee9`.
Reconstruction from the hash-verified pinned archive matches the vendored tree
byte-for-byte, excluding generated `Cargo.lock`.

## Retained navigation and worker diagnostics — locally repaired

The navigation failure also reproduces locally. Before/after snapshots retain
identical native window/node generations, editing revision, text, selection and
composition; hiding the route changes only focus from true to false. Retention
assertions now compare all ownership/editing fields independently of focus.
Hidden-editor focus commands must still fail, and returning to the route must
allow focusing the same editor. The public navigation self-test passes locally.

The picker investigation exposed a separate diagnostic defect: raising a worker
exception inside its OCaml domain caused `Domain.join` to replace the useful
origin with its join site. The Eio application now carries the exception and raw
backtrace as data across the join, then reraises with that trace. It also copies
the caller's backtrace-recording setting into the worker domain. A deliberate
worker failure first failed the new origin assertion and now passes. The
`examples/runtime/main.exe --worker-backtrace-test` regression is part of macOS
CI. The full Dune build/expect/format suite passes, as do runtime
`--self-test`, `--shutdown-test`, `--last-window-test` and
`--worker-backtrace-test`, followed by the picker/navigation self-tests against
the rebuilt backend. Runtime self-test checks include scoped cancellation,
Bonsai timers, idle commit stability, reopen and frame acknowledgements.

Full Rust workspace tests and strict all-target Clippy also pass:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --workspace --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --workspace --locked -j2 --all-targets --features gpuio-native/native-image-tests,gpuio-table-adapter/native-tests -- -D warnings
```

The native library's unit suite reports 422 passed and two existing ignored;
platform-specific skipped tests retain their own restrictions. Runtime and public
self-tests used bounded process-group wrappers and closed/reaped normally.

## Table keyboard readiness and traversal execution budget

Local instrumentation observed `is_window_active = false` immediately before the
first targeted OS key, even though activation had been requested and manual
layout frames had run. That local baseline happened to pass; it does not prove
that activation caused the hosted failure. The fixture now yields until AppKit
reports this window active, with a two-second bound, before posting the first
key. The corrected native table suite passes actual process-targeted arrows,
Return, Shift-F10, Copy and embedded-editor checks. Hosted confirmation remains
required; no synthetic replacement or key retry was introduced.

The hosted full-history test made steady progress to 171,936 of 200,000 visits
at 895.8 seconds: 33.8 seconds applying updates, 859.8 seconds in layout/paint,
and 0.8 seconds checking/yielding. Active rows/cells were bounded at 128/512,
retired cached payloads were zero and peak RSS was 153,665,536 bytes in that
partial log. These observations do not establish completion or final cleanup.

The runner now permits 1,800 seconds for the unchanged 100,000-row × two-pass
debug-build ownership workload, retaining group termination/reaping and the
separate deliberate-failure cleanup check. This accommodates the observed hosted
throughput; it is not a release latency or memory budget. Named-hardware
performance and a completed hosted traversal remain required.

## Completed local full-history rerun

`GPUIO_JOBS=2 python3 scripts/test_table_history.py` completed successfully on the
local M1 Max/macOS 14.5 machine. The full traversal executable was built at the
`e54d279` checkpoint, before subsequent grid-placement work began.

| Measurement | Result |
| --- | ---: |
| Logical rows / traversals / visits | 100,000 / 2 / 200,000 |
| Active rows / cells | at most 128 / 512 |
| Peak retired text payloads retained at batch checks | 0 |
| Retired text after unmount | 0 |
| Baseline / peak admission accounting | 19,200,689 / 19,659,441 bytes |
| Peak mounted accounting above baseline | 458,752 bytes |
| Initial / final process peak RSS | 80,150,528 / 249,806,848 bytes |
| Traversal duration | 809.3 seconds |

The window-release marker passed. The runner's separate deliberate-failure
invocation rebuilt while local grid-placement protocol work was in progress;
it produced the expected assertion, cleanup and verified-failure-exit markers,
and the parent runner exited zero. That short cleanup run is evidence for the
newer working tree's failure path; the complete traversal metrics above apply
to the pre-grid executable. Both child processes exited and were reaped.

This hidden-window debug workload exercises actual retained layout and ownership.
It does not establish physical input-to-paint latency, a named release performance
budget, or complete traversal on the hosted runner. The 1,800-second limit is a
bounded test execution allowance, not a frame-time requirement.

## Remaining validation

The public date-picker failure is still unreproduced locally: both its original
self-test and the version with named draft-read diagnostics pass. No production
picker fix is claimed. New failures will include the draft stage and original
worker backtrace; investigate observation readiness rather than assuming a fixed
number of rendered frames proves native snapshot delivery.

Current-head required CI, complete hosted table traversal, other catalog families
and broader OCH-17 requirements remain open. No milestone or release acceptance
is claimed. Full Linux desktop qualification remains OCH-47; a passing Linux job
does not establish graphical acceptance.


## Follow-up run — still incomplete

[Run 36791905054](https://github.com/dakotamurphyucf/gpuio/actions/runs/36791905054)
tests `e54d2795df761e05067b5359a9b074c8590cb770`. Its Linux job completed
successfully, including build/unit/private-bus/lint and independent consumer
checks; the informational X11/Wayland smoke steps also report success. This is
not full Linux desktop qualification.

At the 2026-09-30 local follow-up, macOS had passed native-test compilation and
advanced into native table full-history execution, but **macOS public canvas
checks and Public date picker failed**. The job was still running. Its job-log endpoint returned `BlobNotFound` (404),
so no failure cause is established yet; inspect the completed job logs/artifact
before diagnosing or changing the canvas test. Do not rerun/cancel the active
job just because its archived log is not yet available.

This hosted revision excludes the local grid-location commit and the uncommitted
rich Form/avatar-group work. Their local checks and remaining native acceptance
must not be inferred from this run.

## Follow-up run — terminal result, 2026-10-01 UTC

The same run is now complete: Linux **success**, macOS **cancelled**. Archived
[job logs](https://github.com/dakotamurphyucf/gpuio/actions/runs/36791905054/job/110146488948)
and the [macOS artifact](https://github.com/dakotamurphyucf/gpuio/actions/runs/36791905054/artifacts/11134164397)
are available. This supersedes the in-progress checkpoint above; no rerun was
started and no cancellation was requested by this agent.

The hosted full-history regression passed all **200,000 visits / 100,000 logical
rows**, retaining at most 128 rows and 512 cells, with zero retired cached row
payloads. Accounted bytes rose from 19,200,689 to 19,659,441 (458,752 bytes). Peak
process RSS rose from 73,891,840 to 153,731,072 bytes. The measured traversal took
1,034.9 seconds. Normal window release and the deliberately failing cleanup
subprocess both passed. These are hosted debug ownership/workload measurements,
not physical interactive latency or complete release-performance acceptance.

Two steps failed before cancellation:

- **Public canvas:** `scripts/test_canvas.py:102` timed out waiting for
  `Activated: Swift` after AXPress. The dump still showed `Moved: Swift` and
  native scene revision 1/generation 1. Keyboard movement had already passed.
  Source review shows that native AX routes fence captured scene snapshots and
  input is unavailable while a replacement publication is being prepared. A
  publication/AX timing race is a hypothesis, not an established cause. No canvas
  production fix, repeated activation, relaxed assertion or larger timeout has
  been substituted for a reproduction.
- **Public date picker:** the named diagnostic reports `initial open`,
  `is_open=true`, no error, and no draft. The preserved worker backtrace points to
  the initial selection assertion in `examples/calendar/picker.ml`. Two window
  frame acknowledgments do not promise delivery of the native calendar snapshot.
  The local self-test now waits for the current reactive picker to expose an open
  draft, with at most 120 frame requests and immediate failure on a picker error.
  Placement remount uses the same readiness condition. All selection, stale-action,
  cancellation and disposal assertions remain. Public interface docs now state
  this asynchronous readiness contract. The changed native self-test has **not**
  run successfully yet under the current desktop execution restrictions; if the
  observation never arrives, it must still fail rather than assume readiness.

The job was cancelled at 01:06:30 UTC during the combined agent-chat workload,
after runner setup began at 23:36:04 UTC. This is consistent with its configured
90-minute job ceiling; the API reports `cancelled`, not a separate causal timeout
annotation. Local workflow configuration gives macOS 120 minutes, with Linux
remaining at 90. This accommodates the measured full-history work without changing
individual test watchdogs, workloads or assertions. It still needs a complete
hosted run. Native image cleanup, editor, native smoke and production bridge smoke
were skipped after cancellation and are **not passes**.

The local corrections and the new avatar phase regression are uncommitted and
absent from the tested `e54d279` revision. Required current-head CI, the remaining
canvas investigation and wider catalog/release acceptance are still outstanding.


### Canvas investigation follow-up

The [canvas activation investigation](canvas-activation-och17.md) adds opt-in
native semantic-tree/callback/mailbox traces and independent example event
logging to the next public test run. A TestPlatform test now exercises scene
replacement, callback retirement and input admission; the full native library
suite with image/canvas test support passed locally (425 tests, two ignored).
These diagnostics preserve the existing production fences and do not establish
the hosted failure's cause or resolution. No new native AX run has passed.


### Rich avatar local implementation follow-up

The [rich avatar fallback](../design/avatar-fallback.md) adds checked Core/Bonsai
and group-item constructors, native slot selection and a separately owned hidden
set. Raster/SVG TestPlatform checks cover retained source leases, source failure,
first-frame SVG size failure/recovery, hidden tween deadlines, inert versus hidden
ancestors and teardown. An inert-ancestor regression failed before the corrected
paint-visibility decision and now passes. Passive-child mutation rollback and
node-count limits have native transaction coverage; the new contract requires
paired capability bit 56, without changing operation tags.

The Rust workspace suite, feature-enabled native library suite (426 passed,
two ignored), strict native/protocol all-target Clippy and fresh installed-gallery
consumer build pass locally on macOS arm64. Three Core expect tests and the OCaml
examples compile; the gallery uses an original registered person SVG for its
custom fallback. Its native driver has been extended but **not run**. No physical
GPU clipping, macOS AX ownership or Linux qualification is inferred from these
checks. Changes remain local/uncommitted and absent from the earlier hosted run.

The final `dune build -j2 @all @runtest @fmt` also passed after the inert-ancestor
correction, through the repository's isolated `scripts/gpuio exec` environment.
No native windows or new hosted workflow runs were started for this addition.

### Rich avatar acceptance-fixture follow-up

The retained-slot TestPlatform scenario now includes a nested avatar and delayed
animation program as well as a tween. It passes hidden deadline cancellation,
no wakes after a 20-second test-clock advance, owner identity on restoration,
inert/hidden ancestry and weak-owner retirement. The feature-enabled native
library suite again passes locally: 426 tests, two ignored private-bus tests.

`avatar_rich_test.rs` is wired into the existing `native_images` background
window. It includes GPU clipping/corner/hover, native macOS AX, retained image
leases, raster/SVG/GIF selection, event order and idle/disposal assertions.
**Execution is pending**: compiling this fixture does not make those assertions
passing native evidence. The harness now logs window-open, tree-application,
first-pixel and semantic-check phases to help distinguish startup failures on
the next actual run. These diagnostics do not repair or establish the cause of
the earlier black window/desktop-service timeout.

Final local checks passed on macOS arm64, using the isolated repository toolchain:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native \
  --features native-image-tests,native-canvas-tests --lib
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native \
  --features native-image-tests,native-canvas-tests --test native_images --no-run
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j2 -p gpuio-native \
  --features native-image-tests,native-canvas-tests --all-targets -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all --check
```

These test-only changes remain uncommitted on top of `83eb87e` and the prior local
implementation. No new Linux, native desktop or hosted CI result is claimed.

The subsequent TestPlatform window-close case passes with live nested avatar
tween/program deadlines. It closes the Session and GPUI window without first
unmounting the tree or stopping the global image service, then checks weak-owner
release, motion declarations, fallback source leases and post-close output.
The updated feature-enabled library suite passes **427 tests, two ignored**;
strict all-target native Clippy also passes. This is additional deterministic
lifecycle evidence, not real macOS window-close acceptance.

### Rating appearance follow-up

The [pinned Rating review](../catalog/rating-review.md) identified an independent
active/outline color gap. The new [appearance API](../design/rating-appearance.md)
now connects Core/Bonsai theme resolution, appearance-only retained updates,
bounded native RGBA admission and the star painter. Op64/capability57 extend the
paired protocol; existing config, request and event encodings remain unchanged.

The full OCaml build/expect/format suite, Rust workspace suite, feature-enabled
native library suite (427 passed, two ignored), strict native/protocol all-target
Clippy, Rust formatting and fresh installed-gallery consumer build pass locally.
Independent bytes, invalid-update rollback, theme failure/reset/no-op/identity
and current reducer policy have deterministic coverage. The first Rust workspace
run found eight stale aggregate capability-mask expectations; those were corrected
before the final passing run. No production behavior was weakened for those tests.

The native presentation executable links with the new GPU assertions and the
gallery has a dedicated `--section rating` driver. Neither was run on the desktop;
the consumer result is build-only (`run=False`). Actual colors, input, accessibility
and gallery retirement remain unaccepted. Changes remain local/uncommitted on
`83eb87e` plus earlier milestone work and are absent from the last hosted run.
