# Milestone 07 hosted validation follow-up

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
