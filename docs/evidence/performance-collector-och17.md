# Optional native performance collector — OCH-17

Implemented after `ce8bb76` on macOS 14.5 arm64. The `performance-diagnostics`
Cargo feature exposes `gpuio_native::performance`; default builds exclude it.
The module samples the pinned GPUI's cumulative window histograms on demand. It
adds no host loop, timer, redraw, file output, global trace activation or synchronous
OCaml callback. Native snapshots own bucket data and a window ID, not a window
handle. Capture overhead remains to be measured.

`Snapshot::since` subtracts each cumulative bucket count and the dropped-input
counter. It rejects different windows, reversed time and any regressed bucket,
even when total samples increased. Intervals expose raw ascending buckets,
nearest-rank percentiles, counts and elapsed time. Empty distributions yield no
percentile. Upstream three-significant-digit HDR upper bounds retain quantization;
durations are nanoseconds, while inputs-per-frame buckets count events. Native
input→frame and dirty→submission are not hardware latency or physical presentation.

Local validation, all exit zero:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked --offline -j2 \
  -p gpuio-native --features performance-diagnostics,native-image-tests,native-canvas-tests \
  --lib performance::tests
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked --offline -j2 \
  -p gpuio-native --features performance-diagnostics,native-image-tests,native-canvas-tests --lib
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked --offline -j2 \
  -p gpuio-native --features performance-diagnostics,native-image-tests,native-canvas-tests \
  --lib --tests -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo check --locked --offline -j2 -p gpuio-native
GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all --check
```

The focused suite passes five tests. Four cover interval mathematics/error cases;
the fifth captures a real GPUI Window using TestPlatform, observes no new samples
from repeated captures, records exactly one explicitly drawn frame, and closes
the window with snapshots still owned. The full feature-enabled library suite
reports **925 passed, two existing macOS private-bus skips**. No desktop window
was opened. Strict lint, default-feature compilation, formatting and workflow
YAML parsing pass. CI adds required no-display collector tests/lint on both OSes;
that new step has not run on the hosted source yet.

Logs: `performance-collector-00{1,2}.log`, `performance-native-suite-001.log`,
`performance-clippy-001.log`, `performance-default-check-001.log` and
`performance-format-check-001.log` under the local session directory
`scratch/agents/root-20261004-resumed/`.

## Dependency and notice evidence

No GPUI source/pin changes. Enabling its existing optional profiler adds locked
`hdrhistogram` 7.6.0. Its full MIT and Apache-2.0 license texts are collected.
The first supplemental collection correctly rejected the changed first-party
native Cargo manifest hash; after reviewing the added feature and unchanged
Apache workspace inheritance, only that package's declaration hash was updated.
The root license bytes and all third-party declarations remain unchanged.

Fresh profiled native collection, including `third_party/notice-sources.json`,
reports 514 packages, 868 collected-file hashes verified, and the same 27 existing
missing-text packages. Earlier no-supplemental and stale-declaration attempts are
retained separately. This is collection, not distribution/license approval.
Reproduction:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec python3 scripts/collect_rust_notices.py \
  --target aarch64-apple-darwin --root gpuio-native \
  --features gpuio-native/performance-diagnostics \
  --supplemental third_party/notice-sources.json \
  --output scratch/fresh-profiled-native-notices
```

See [the qualification plan](../design/performance-qualification.md). Optimized
workload drivers, actual input/presentation evidence, sampling-overhead comparison,
resource/idle budgets and release acceptance remain unfinished. Passing collector
math or TestPlatform checks cannot establish those outcomes.
