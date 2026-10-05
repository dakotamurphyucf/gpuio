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
The integrated results follow below; those are a distinct measurement.

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
does not attribute every byte in the observed plateau. The full runs below check
whether it stabilizes beyond warmup; this is not a zero-allocation result.

## Three optimized full runs

All three full attempts passed, with no discarded full run. Each fresh process
performed three warmups and thirty measured closes, preceded by one separate
four-cycle optimized preflight for the series. Source
`afd4c0bbe4e3de22571179305fbf81e3bbf263d5` was clean at every launch. The optimized
executable SHA-256 was
`715a80cddd4aafa705d21d2e2f921a3f708e752ddd9f5a6172f9e48c5fc5a358`.
The same M1 Max / 32 GiB / macOS 14.5 desktop ran one normal visible window at
a time, on AC power with no recorded thermal/performance warning and without
another owned GUI workload or local compiler. No OS preference,
VoiceOver, forced collection or cache-purge operations were performed.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release examples/resource_audit/main.exe @examples/resource_audit/ocaml/runtest @fmt
python3 scripts/measure_resource_lifecycle.py --build-profile release --native-entities --metal-memory --physical-memory --check-closed-surfaces --executable _build/default/examples/resource_audit/main.exe --smoke --output scratch/agents/root-20261004-resumed/metal-audit-release-smoke-001 --timeout 120
```

For each `NNN` in `001`, `002`, `003`:

```sh
python3 scripts/measure_resource_lifecycle.py --build-profile release --native-entities --metal-memory --physical-memory --check-closed-surfaces --executable _build/default/examples/resource_audit/main.exe --check-budgets --output scratch/agents/root-20261004-resumed/metal-audit-full-NNN --timeout 300
```

| Run | Final closed Metal bytes | Final-ten Metal growth | Final-ten RSS growth | Peak settled physical bytes | Wall seconds |
| --- | ---: | ---: | ---: | ---: | ---: |
| [001](metal-resource-och17/run-001.json) | 4,587,520 | 0 | 4,227,072 | 106,842,944 | 121.530 |
| [002](metal-resource-och17/run-002.json) | 4,587,520 | 0 | 3,293,184 | 103,418,816 | 121.712 |
| [003](metal-resource-och17/run-003.json) | 4,587,520 | 0 | 6,209,536 | 103,189,248 | 121.438 |

All runs used renderer device `4294969919`. Each first closed sample was
5,128,192 bytes; by the final warmup and through the measured portion the counter
was 4,587,520 bytes, with zero final-ten range or growth and zero change from the
warmup baseline. This satisfies the predeclared 64 MiB growth guardrail on this
workload/hardware. It does not attribute every surviving allocation or guarantee
the same resource behavior for arbitrary applications.

All 99 application-retirement checkpoints, all 90 post-warmup native entity
checks and all 99 closed IOSurface checks passed. Each process returned zero and
was reaped. The [001](metal-resource-och17/run-001-artifacts.tar.gz),
[002](metal-resource-och17/run-002-artifacts.tar.gz) and
[003](metal-resource-och17/run-003-artifacts.tar.gz) archives preserve application
output and all raw OS tool captures; every recorded artifact hash was checked
before archiving. The reports retain per-cycle sampled live maxima, sample
counts, closed values, hardware and process measurements.

This closes the scoped renderer-device allocation workload check. Actual drawable
presentation, full GPU/driver/compositor attribution and other release acceptance
remain separate. Hosted checks for this new source have not yet run; Linux GUI
qualification remains deferred to OCH-47.
