# Sankey label worker and paired public option

OCH-41, 2026-10-06, macOS 14.5 arm64. Source base `ba02d72` plus the feature
changes in this commit. This checkpoint implements the public option and executes
native font measurement; it does **not** complete rendered-label acceptance.

`Chart_options.Sankey.create ~label_placement` accepts Inside (default) or Outside.
Options schema 6 appends the placement tag to the previous options record; default
encoding is 102 bytes. Style/data remain -2/1. Independent OCaml/Rust goldens and
the paired `chart-v6-label-placement-view.hex` cover this boundary. Unsupported
old versions and unknown Rust decoder tags are rejected. Matching packages are
required on both sides of the bridge.

The native request captures its inherited font, rem-derived backing padding and
shared text system. Request equality includes this context. Preparation creates a
private text cache on the worker, resolves rich overrides by node ID, checks
cancellation between lines, shapes actual strings and supplies source-ordered
metrics to the [geometry engine](sankey-label-layout-engine.md). Empty overrides
hide labels. Invalid measurement fails preparation rather than silently estimating
width. Ready plans retain the measured font/padding; the painter uses that style
while a replacement plan is pending. Explicit empty font fallbacks prevent a later
inherited fallback list from changing an old plan's text style.

The existing two-worker admission, cancellation and stale-result machinery remain
in place. Temporary text caches are discarded after a request; the existing
workspace reservation is admission accounting, not an allocator/RSS guarantee.
No window/App borrow or OCaml callback is moved onto the worker.

## Actual worker execution

The new `native_chart_labels` harness starts the macOS platform application without
opening a window. It captures the platform text system on the UI thread, awaits
measurement on GPUI's background executor, and asserts the worker thread differs.
The UI loop stays available; no synchronous thread join blocks it. A 20-second
internal deadline bounds waiting for the result.

Passing checks cover proportional W/i widths, Japanese/Greek/joined emoji text,
exact additional backing padding, a changed bold font context, rich multiline
font sizes/heights, an empty hidden override, cancellation and measured preparation
at scales 1, 1.25, 1.5 and 2. These are real font metrics and prepared geometry,
not GPU readback, visible glyph assertions or a claim of pixel-perfect placement.
The harness is added to macOS CI and its prebuild list; hosted execution is pending.

## Commands and results

All commands use the isolated repository environment with `GPUIO_JOBS=2`.

```sh
./scripts/gpuio exec cargo test --locked -j2 -p gpuio-protocol
./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native --lib --features native-tests
./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native --features native-canvas-tests --test native_chart_labels
./scripts/gpuio exec cargo clippy --locked -j2 -p gpuio-native --features native-canvas-tests --lib --tests --no-deps -- -D warnings
./scripts/gpuio exec dune build -j2 @all @runtest @fmt
./scripts/gpuio exec dune build -j2 @test/chart/runtest @fmt
cargo fmt --all --check
```

- Protocol suite: **430 pass**. The initial run failed against three obsolete v5
  goldens; updating the independently specified default bytes/version/fixture and
  adding explicit placement/old-version checks resolves those failures.
- Native library: **641 pass, two existing skips**. This is the library suite,
  not every desktop integration executable.
- Real platform font harness: **PASS**, no window opened.
- Strict native-canvas Clippy and Rust formatting: **PASS**.
- Full OCaml build/tests/format invocation: exit 1 from four formatting diffs;
  no other build/test failures reported. After formatting exactly those four
  files, focused chart expect tests and the repository formatting alias **PASS**.
  The latter is a focused correction check, not a second full all-example run.

[Archived raw logs](sankey-label-worker/logs.tar.gz) and the
[manifest](sankey-label-worker/manifest.json) retain hashes of every log and the
relevant source inputs. Initial failures are preserved. Vendor deprecation/future
compatibility warnings remain visible in those logs.

## Still required

The [design acceptance list](../design/sankey-label-placement.md) remains open:
three-column public gallery with short/long/rich labels, middle placement,
inside/outside switching, narrow resize, font/theme transitions, unchanged raw
selection, source updates and cleanup; actual rendered geometry/pixels; and a
fresh independently installed consumer. Mounted request replacement/stale-result
behavior needs coverage with the new context, beyond existing generic worker
tests and context equality checks. Current-source hosted checks remain required.
No new keyboard, IME, VoiceOver, performance or Linux desktop acceptance is claimed.
