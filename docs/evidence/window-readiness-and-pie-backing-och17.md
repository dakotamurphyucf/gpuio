# Window readiness and synthetic pie backing repairs

OCH-17 / OCH-41, 2026-10-06. Local changes after `29dda32` address two failures
from [hosted run 37538145025](hosted-presentation-calibration-och17.md#hosted-run-37538145025).
This is scoped validation, not release acceptance or confirmation that a hosted
rerun passes. Linux has not run these changes yet.

## Window opening is distinct from receiving metadata

Signal Studio's workload previously waited for `Window.snapshot` to become
`Some` before requesting a frame on each reopened window. The native lifecycle
can deliver `Window_changed` while the OCaml handle is still `Opening`; only the
later `Opened` acknowledgement admits frame requests. This explains the hosted
exception after the cycle-2 close.

[`App.Window.is_open`](../../lib/eio/app.mli) now exposes that lifecycle predicate:
opening is acknowledged, closing has not begun, and the application is not
stopping. It enforces the same OCaml UI-domain ownership as other window methods.
It is not the negation of `is_closed`, nor a focus, visibility, publication or
presentation guarantee. Frame admission uses this predicate; its existing pending
request and queue bounds remain unchanged.

The example now awaits `is_open`. The adjacent
[beginner walkthrough](../../examples/signal_studio/checks.md) explains the actual
UI effect, Eio polling/promise wait and Bonsai state flow. A GPT-6.1 Sol documentation
agent drafted that scoped update; the implementing agent reviewed it against the
code. No synchronous Rust-to-OCaml callback or runtime scheduling change is added.

A deterministic Eio expect test injects an early snapshot before `Opened` and
proves frame rejection despite snapshot presence, then admission after the
acknowledgement. It also covers shutdown, closing, closed state, close before
opening and a late opening acknowledgement. That test uses native transport
allocation with injected lifecycle events; it is not a GUI or physical input test.

## Synthetic scale requires sufficient native backing

GPUI's test-only `Window.set_scale_factor` changes the scene scale. It does not
resize the native drawable. The macOS image readback takes its dimensions from
`CAMetalLayer.drawable_size`. The failed hosted caption starts at logical x=149;
at synthetic 2× it would be outside a 240-pixel drawable on a 1× display. The
failed run did not record image dimensions, so this remains a source-backed
explanation requiring hosted confirmation.

The hidden pie test now reserves a 480×480 logical native viewport, awaits its
resize acknowledgement, then draws its unchanged 240×240 root through synthetic
scales 1, 1.25, 1.5 and 2. Every readback asserts a backing of at least 480×480
physical pixels and the requested scene scale. Afterward the test restores the
original viewport and scale. Caption failures also save the diagnostic image
when configured and report its dimensions.

Caption pixel counts, leader hue/coverage thresholds, wrong-hue rejection, neutral
negative controls, font/width variations and source invariance checks are unchanged.
The complete hidden native chart suite passes locally, including subsequent tests
using the restored window. Final metrics are `(178,19,2,0,0)`. This repairs test
geometry; it does not change production chart rendering or simulate physical
high-density monitors.

## Local validation

Apple M1 Max, macOS 14.5 arm64, repository-isolated toolchain, two build jobs:

- `GPUIO_JOBS=2 ./scripts/gpuio exec dune runtest lib/eio -j 2` passes, including
  the new lifecycle regression.
- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests --test native_chart_view --locked -j2`
  passes the actual hidden-window GPU suite, including the four pie test densities.
- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -p gpuio-native --features native-canvas-tests,native-image-tests --lib --tests --locked -j2 -- -D warnings`
  passes; `cargo fmt --all --check` also passes through the repository wrapper.
- `GPUIO_JOBS=2 ./scripts/gpuio exec dune build @all @runtest @fmt -j 2` passes.
- `python3 scripts/measure_signal_studio.py --output scratch/agents/root-20261004-resumed/ci-repairs/signal-workload`
  passes in 6.24 seconds: 384 desired updates, 96 render samples, 12 native commands,
  12 close/reopen cycles and 24 native component/callback lifetimes. Callback
  values peak at two; final source charge is zero. These are resource/render
  observations, not physical presentation or keyboard/IME acceptance.
- The example inventory remains 421 source files / 262 reviewed groups / zero
  pending. The changed guide's links and scoped whitespace checks pass.

All local commands finished and the diagnostic windows closed. The
[logs and source hashes](window-readiness-and-pie-backing-och17-logs.tar.gz) and
[verified manifest](window-readiness-and-pie-backing-och17-manifest.json) preserve
the dirty-tree test identity after `29dda32`, rather than attributing these tests
to that unchanged commit. The next hosted run must confirm the two repairs and
current-source Linux coverage. The separate Metal probes, physical presentation,
distribution and other milestone requirements remain open.
