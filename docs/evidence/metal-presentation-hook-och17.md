# Native GPUI Metal hook qualification — OCH-17

2026-10-05, local macOS14.5 arm64, Apple M1 Max32GiB. The built-in Color LCD reports
1728×1117 at120Hz; this is the configured display mode, not a measured frame-rate
claim. The dev probe uses unoptimized first-party code and dependency opt-level1.
It validates the renderer hookup and ownership, **not workload performance budgets**.
Source baseline is `587671c` plus the [recorded source hashes](metal-presentation-hook-och17/source-hashes.json).
The [binary hash](metal-presentation-hook-och17/binary.json),
[complete native report](metal-presentation-hook-och17/report.json) and
[native log](metal-presentation-hook-och17/native.log) identify the tested artifact.

## Actual desktop result

`scripts/test_metal_presentation.py` built and ran the real GPUI Metal backend.
Two distinct sessions/windows each rendered 90 changing scenes. Both windows
were visible at the start/end observations. Each received exactly 90 admitted,
valid presented records and 90 submission-latency samples. The active second
window recorded89 animation intervals; the inactive first correctly recorded0.
Each trace retains its window/session identity, sequence, drawable identity,
active/animation/scene flags, bracketed host submission, native presentation,
callback arrival and conservative latency bounds.

Both windows had zero unsupported submissions, saturation, missing/zero/invalid
results, duplicate callbacks, trace truncation and histogram overflow. There
were no input events or input-latency samples; this run does **not** qualify
physical keyboard-to-presentation latency. After admission stopped and callbacks
settled, both windows were removed. Both retained snapshots report closed windows
and zero pending records. The app exited0 and the parent reaped it. No compiler
ran concurrently with the native measurement. The windows closed automatically;
no VoiceOver, accessibility automation or desktop preference changes were used.

The parent imposes a25-second runtime limit and requires a fresh structured report
whose assertions passed. An early NSApplication exit0 without a completed report
cannot pass. The sample checks at least90 valid presentations per window, distinct
window/session IDs, bounded counts, retirement and at least45 active animation
intervals. It does not impose a performance threshold inferred from these data.

## Build, ownership and packaging checks

- Native feature-enabled suite: **932 passed, 2 existing private-D-Bus skips**.
  The private-bus tests require their separate isolated bus runner; this local
  suite does not replace the required Linux private-bus gate.
- All5 performance collector tests pass, including explicit `Unsupported` for
  TestPlatform through the native presentation wrapper. The preceding core
  [ordering/clock/lifetime tests](presentation-core-och17.md) remain applicable.
- Strict native Clippy with presentation/image/canvas features, locked default
  native compilation, Cargo/Dune formatting and workflow lint pass.
- Six composer tests and two vendor manifest tests pass. All four standalone
  backend locks select the same path-patched Apple renderer. Root changes add
  that optional dependency and the existing serde_json package for the test;
  no dependency revision is upgraded.
- The independent installed gallery consumer builds outside the checkout with
  the new generated patch, passes component/document catalogs and removes its
  temporary workspace. No consumer GUI run was performed by that build check.
- Complete Apple reconstruction matches all committed files. Patch SHA-256:
  `dae77db60f07fa7e8675ce89e25c16e22df8d7daa209fc29194381ae4baa5fc1`.
  The core patch remains `2e66c189a9ebb323229fcd48a0fc19b1e26ed06e0a3ad03491423a4a65912eb7`.

Compressed logs are in [the evidence directory](metal-presentation-hook-och17/).
Representative exact commands:

```sh
GPUIO_JOBS=2 python3 scripts/test_metal_presentation.py \
  --output scratch/agents/root-20261004-resumed/apple-native-001
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --lib --locked \
  --features presentation-diagnostics,native-image-tests,native-canvas-tests -j 2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -p gpuio-native \
  --features presentation-diagnostics,native-image-tests,native-canvas-tests \
  --lib --tests -j 2 -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo check --locked -p gpuio-native --lib -j 2
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --cleanup
```

An initial simultaneous `@fmt` attempt encountered the consumer's Dune lock;
formatting passed after that build moved to its independent workspace. No build
or switch was interrupted. The CI workflow adds the native hook run on macOS and
keeps diagnostic compilation/tests on both platforms. New-checkpoint hosted/Linux
validation is still pending. The older run37297058441 is not evidence for this
source. VoiceOver remains on hold, and full Linux desktop qualification stays
separately deferred to OCH-47.

Next: integrate these observations into the actual list/table/document/streaming
workloads, preserve predeclared sizes/content and CPU/resource gates, qualify
paired native input, compare overhead/default builds, and collect the required
repeated optimized runs. These requirements remain open.
