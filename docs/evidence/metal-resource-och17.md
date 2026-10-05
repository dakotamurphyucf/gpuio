# Metal allocation qualification — OCH-17

2026-10-05, Apple M1 Max, macOS 14.5 (23F79). Follow the
[predeclared design](../design/metal-resource-qualification.md). This is separate
from the completed entity, RSS, physical-footprint and IOSurface audits.

## Counter calibration

The [standalone Swift tool](../../scripts/qualify_metal_counter.swift) creates no
window and changes no OS preferences. The [raw result](metal-resource-och17/calibration.json)
records a 65,536-byte baseline, 8,454,144 bytes with an 8 MiB buffer held, and a
return to 65,536 bytes after release. Both the disposable view and layer were
released while their device remained held. The initial attempt checked too early,
before an implicit Core Animation transaction settled; the tool now gives normal
run-loop settlement a bounded two-second wait and still requires both weak
references to clear. No cache purge or forced collection is used.

```sh
xcrun swiftc -module-cache-path scratch/agents/root-20261004-resumed/swift-modules scripts/qualify_metal_counter.swift -o scratch/agents/root-20261004-resumed/qualify-metal-counter
scratch/agents/root-20261004-resumed/qualify-metal-counter > scratch/agents/root-20261004-resumed/metal-counter-calibration-002.json
```

This result validates the counter's sensitivity and the proposed ownership
boundary. It does not establish actual GPUI device capture, bounded workload
allocations, GPU completion, compositor memory or physical frame presentation.
Integrated workload qualification remains pending.

## Integration smoke

The schema-v2 qualification backend and collector passed the local development
build's four-cycle smoke: the actual renderer device remained `4294969919`, every
cycle had two or three live render samples, and all four matching Metal/entity/
application retirement checkpoints completed. Closed Metal counts were 3,031,040,
4,587,520, 4,587,520 and 4,587,520 bytes. The separate physical-memory check found
no nonzero closed IOSurface category. The owned child returned zero and was reaped.
This is smoke evidence, not optimized budget acceptance.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --manifest-path examples/resource_audit/rust/Cargo.toml --offline --lib
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --manifest-path examples/resource_audit/rust/Cargo.toml --locked --all-targets -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/resource_audit/main.exe @examples/resource_audit/ocaml/runtest @fmt
python3 scripts/test_measure_resource_lifecycle.py
python3 scripts/measure_resource_lifecycle.py --build-profile dev --native-entities --metal-memory --physical-memory --check-closed-surfaces --executable _build/default/examples/resource_audit/main.exe --smoke --output scratch/agents/root-20261004-resumed/metal-audit-smoke-001 --timeout 120
```

Two Rust tests, ten collector tests, scoped OCaml expect tests, strict probe
Clippy, OCaml formatting and workflow actionlint pass locally. The updated macOS
workflow includes calibration and integration smoke; hosted execution is pending.
The standalone probe adds only the already pinned objc2/objc2-encode packages;
the rendering backend already had them. No production renderer or fork changed.

Source inspection finds an application-shared `InstanceBufferPool` in the pinned
`gpui_apple` Metal renderer. It retains reusable 2 MiB buffers after GPU completion.
That is one expected source of nonzero settled allocations, but the counter alone
does not attribute every byte in the observed plateau. Full runs must establish
whether it stabilizes beyond warmup; do not call this a zero-allocation result.
