# How active presentation collectors retire

[README](../../README.md) · [Source](presentation.rs) · [Audit owner](lib.md)

This module belongs to a qualification backend. Applications do not need it.
It contains no Bonsai component and makes no synchronous call into OCaml.
The OCaml workload passes a checked `presentation` flag when constructing its
audit extension; Rust then owns the measurement lifetime.

With Cargo feature `presentation-diagnostics`, `Probe::capture` starts a native
`Session` immediately during mount. The session owns bounded counters, histograms
and raw records, but no window, entity, Metal drawable or event sink. The audit's
application global retains it alongside the window's ID. This immediate start
matters: the separate performance probe deliberately delays its Begin operation,
and short lifecycle cycles can close before that operation completes. Enabling
that probe's presentation feature would miss those cycles and could compete for
the same window's single collector slot. This backend enables only the resource
audit's presentation feature.

After native close, the audit calls `stop`, preventing new admission. Its existing
one-second settlement timer then calls the consuming `retire(self, cycle)` method.
That method snapshots value data and drops the session, releasing the sole strong
measurement owner. Callback holders and the window's controller have weak
references; they cannot retain this measurement state after its owner is dropped.
The returned `Record` contains JSON values only. The audit can now check native
entities and Metal allocations, then call `Record::emit` before completing the
cycle. The process collector samples memory only after every requested record
and the OCaml retirement checkpoint have arrived.

`retirement_record` requires a closed window, stopped admission, no pending frames,
nonzero actual supported drawable outcomes, exact admitted/outcome accounting,
and no saturation, duplicate callbacks or histogram overflow. Zero-time and other
settled outcomes remain in the output. Their presence can be compatible with
released resources, but does not establish successful presentation or timing.
The separate presentation workload tests retain their stricter frame and latency
requirements. No timeout, RSS or Metal memory budget is increased here.

Without the Cargo feature, `Probe::capture` returns an error. With the feature on
an unsupported backend, the native `Session::start` also returns an error. Neither
case produces a fabricated successful audit. Without the OCaml/CLI flag, no
session is created, even when diagnostics were compiled into this backend.

A concrete full cycle is: OCaml opens window 4 and mounts its audit → Rust starts
its session → document/image/canvas work draws → OCaml requests close → native
close stops admission → the delayed check consumes the session → entity, Metal
and presentation records are emitted → Python verifies cycle and identity → Python
samples closed-window memory and writes `continue 4` → Eio starts the next cycle.
No callback is delivered to a retired extension to acknowledge this process.

From the repository root using the isolated development environment:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --manifest-path examples/resource_audit/rust/Cargo.toml --locked --features presentation-diagnostics --lib -j2
python3 scripts/test_measure_resource_lifecycle.py
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release -j2 examples/resource_audit/main.exe
python3 scripts/measure_resource_lifecycle.py --build-profile release --native-entities --presentation --metal-memory --physical-memory --check-closed-surfaces --executable _build/default/examples/resource_audit/main.exe --smoke --output scratch/presentation-retirement-smoke
```

Use a fresh output directory. Core tests use TestPlatform to verify ownership
and closed-state guards; synthetic callback values are explicitly marked and
are not native timing evidence. Python tests reject missing, unclosed, pending,
reordered and reused identities and withhold continuation until retirement.
Actual macOS runs are still needed for active callback and resource evidence.
Full qualification keeps three warmups and thirty measured cycles, repeated
three times, with the existing memory limits and all failures retained.
