# Native entity retirement — OCH-17

Three full optimized local runs pass at clean source
`8a4e5d916fcda40dcc87184ef6793603f1e8ce4b`. Each process completes three
warmup windows followed by 30 measured open/exercise/close cycles. All 99 closed
application checkpoints satisfy the resource/scope/queue retirement contract;
all 90 post-warmup native checks find **no new live GPUI entity handles relative
to the closed final-warmup baseline**. Each run had a separate four-close smoke
warmup. No full attempt failed or was discarded.

Hardware: Apple M1 Max, 32 GiB, macOS 14.5 arm64, built-in display reporting
1728×1117 logical / 3456×2234 physical pixels at 120 Hz. Each ordinary visible
window requests 1200×800 logical pixels. The raw reports record AC power,
charging, and no recorded thermal/performance warning. One owned GUI workload
ran at a time, without simultaneous compilation; read-only review and remote CI
continued. All children and the owned `caffeinate -di` helper were reaped.

Executable SHA-256:
`2e2a80dc9f69a8a356af31f28aeb4df3d4a808e76052e12babe49fcf91726d8d`.

| Run | Post-warmup native checks passed | Whole-process peak RSS bytes | Final-ten settled RSS growth bytes | Whole-child CPU seconds | Wall seconds |
| -- | --: | --: | --: | --: | --: |
| 001 | 30 / 30 | 137,363,456 | 5,537,792 | 1.731642 | 103.283852 |
| 002 | 30 / 30 | 142,721,024 | 4,964,352 | 1.659008 | 103.157516 |
| 003 | 30 / 30 | 136,134,656 | 4,980,736 | 1.696453 | 103.256077 |

The predeclared 64 MiB final-ten RSS growth limit passes in every run. Entity
tracking changes the build and has its own overhead: these CPU/RSS values are
specific to this audit, not replacements for ordinary responsiveness results.

## Contract and sensitivity

The [dedicated backend](../../examples/resource_audit/README.md) enables the
pinned GPUI `leak-detection` feature. Cargo feature inspection verifies it is
absent from the ordinary performance backend and present here. Both executables
use the same OCaml lifecycle workload: unique decoded images, streamed Markdown,
canvas resources and alternating completed/interrupted asynchronous extension
operations. The ordinary backend also passes its four-close smoke after the
shared-workload refactor.

A qualification component registers each sequential window. An application-owned
subscription waits one second after close, requires no open native windows, then
captures/checks the entity ID snapshot. The app-owned task contains no strong
window/entity handles. The collector waits for both the native audit record and
the ordinary two-second settled application checkpoint before sampling RSS and
permitting the next window. Final acknowledgement also requires the native
completion marker. Missing, reordered, duplicate or failed records reject the
run. The native subscription is cancelled at completion; task storage is released
with the application. No GC or cache purge is forced.

The detector intentionally ignores entities present in the warmup baseline.
This is not a claim that all global entities reach zero or that every Arc,
Objective-C or Metal object is tracked. The separate
[physical-memory audit](physical-memory-och17.md) remains necessary; it caught
a native window/accessibility-adapter cycle outside this entity contract.

The negative native fixture deliberately retains a newly created GPUI entity
and verifies a contained failure. Releasing it makes the same check pass. Panic
containment runs inside the global lease and the test verifies the failed
assertion preserves that baseline/state. No Rust panic crosses FFI, no callback
enters OCaml, and no event is sent through a retired component route.

Validation: two native audit tests, strict audit Clippy, paired OCaml expect
checks, formatting, eight lifecycle collector tests (including schema hashes,
both record arrival orders and final-marker gating), five physical-memory parser
tests and six backend composition tests pass. Optimized ordinary/audit binaries
build locally. This is real macOS execution; hosted Linux unit/build coverage is
separate and does not qualify a Linux desktop.

## Reproduction and raw evidence

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --manifest-path examples/resource_audit/rust/Cargo.toml --locked --lib
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --manifest-path examples/resource_audit/rust/Cargo.toml --locked --all-targets -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release examples/resource_audit/main.exe examples/performance_lifecycle/main.exe @examples/resource_audit/ocaml/runtest @fmt
python3 scripts/test_measure_resource_lifecycle.py
python3 scripts/measure_resource_lifecycle.py --build-profile release --native-entities --executable _build/default/examples/resource_audit/main.exe --smoke --output scratch/entity-warmup-001 --timeout 300
python3 scripts/measure_resource_lifecycle.py --build-profile release --native-entities --executable _build/default/examples/resource_audit/main.exe --check-budgets --output scratch/entity-full-001 --timeout 300
```

Repeat the separate warmup/full pair three times. These runs used a preserved
copy of the release executable with the hash above. The report and application
records were revalidated with the committed collector before copying them here.

- [Run 001](native-entity-run-001-och17.json), [application log](native-entity-run-001-och17.log).
- [Run 002](native-entity-run-002-och17.json), [application log](native-entity-run-002-och17.log).
- [Run 003](native-entity-run-003-och17.json), [application log](native-entity-run-003-och17.log).

OCH-17 remains in progress. Other native interaction/accessibility, catalog/API,
GPU/presentation and distribution gates remain tracked in [status](../status.md).


## Standalone probe lockfile repair — 2026-10-05

Hosted Linux job 111674143362 in run 37282672995 stopped before the audit's tests:
Cargo refused `--locked` resolution. This also reproduced locally, intermittently:
two of eight independent offline/locked metadata invocations failed. Their warnings
listed unused patches in a different order; successful invocations retained the
original order. An unlocked offline resolution rewrote no package records and did
not consistently cure the rejection.

The standalone core/entity probe does not use the document SDK, `gpui_macos` or
`accesskit_macos`. Remove these three unused patch declarations from its manifest
and regenerate its lockfile offline. The only lockfile changes remove those three
`patch.unused` records. Its actual resolved package identities, dependency edges
and features compare exactly equal before/after. The composed rendering backend
retains all of its platform/document patches.

After this change, eight independent offline/locked resolutions pass, and both
probe unit tests pass (including deliberately retained entity failure followed by
release). Commands:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo metadata --offline --locked --manifest-path examples/resource_audit/rust/Cargo.toml --format-version 1
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --manifest-path examples/resource_audit/rust/Cargo.toml --locked --lib
```

This is a reproduced resolution failure and local repair, not a new Linux pass.
The required hosted Linux rerun remains outstanding. No toolchain default, version
or unrelated switch changed. Local raw resolution logs are retained under
`scratch/agents/root-20261004-resumed/resource-audit-*`; these are not build inputs.
