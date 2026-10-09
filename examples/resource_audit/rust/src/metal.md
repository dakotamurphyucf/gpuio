# How `metal.rs` samples the actual renderer device

[README](../../README.md) · [Source](metal.rs) · [Audit caller](lib.md)
· [Accounting design](../../../../docs/design/metal-resource-qualification.md)

This optional macOS qualification probe reads Metal device accounting. It retains
only the renderer's device object plus counters: no Window, NSView, CAMetalLayer,
Entity, event route, timer, or Bonsai graph. It is internal author-side native
instrumentation, not an OCaml consumer API or complete GPU-memory census.

`Probe::capture` requires a main-thread marker and AppKit raw window handle.
It borrows the live NSView, obtains its layer, checks `isKindOfClass:CAMetalLayer`,
then retains its MTLDevice and registry ID. Objective-C temporary ownership is
balanced; only the device survives. This checks the actual window renderer rather
than opening an unrelated default device. Missing/wrong handles return an error,
never a fabricated zero sample.

`require_same_device` checks both registry ID and object pointer equality between
cycles. The native audit replaces its probe for each new window only after this
check, so per-cycle render sample counts/maxima start anew while device identity
must remain stable.

`bytes` requires the main thread and sends `currentAllocatedSize` to that retained
verified device. `sample` updates the largest observed value and checked sample
count. The audit invokes it from its component render hook without requesting
extra frames or invalidating the view. It can miss peaks between renders; the
value is a sampled maximum, not exact lifetime peak allocation.

After closed-window settlement and entity check, `checkpoint` requires at least
one render sample, reads closed bytes, writes/flushed
`GPUIO_METAL_AUDIT checkpoint (cycle registry_id samples sampled_max closed_bytes)`.
The collector requires ordered matching Metal records before allowing the next
window. Failure of a read/write or unsupported backend fails the audit.
Non-macOS builds provide methods returning InvalidCommand; they cannot substitute
another GPU backend or zero-byte success.

From the repository root in the [configured environment](../../../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release examples/resource_audit/main.exe
python3 scripts/measure_resource_lifecycle.py --build-profile release --native-entities --metal-memory --physical-memory --check-closed-surfaces --executable _build/default/examples/resource_audit/main.exe --smoke --output scratch/metal-smoke-review
```

Use macOS, a native graphical session, and a fresh output directory. The external
collector owns deadlines/process cleanup and supplies continuation. Physical-memory
and closed-surface checks are separate OS accounting requests, not this module's
measurements. Smoke does not pass full budgets; full release runs with
`--check-budgets` apply the collector's 64 MiB final-ten-cycle growth guardrail.
See [actual evidence](../../../../docs/evidence/metal-resource-och17.md).

`currentAllocatedSize` reports Metal device allocation accounting, which differs
from process RSS/physical footprint, IOSurface category accounting, chart/canvas
source charges, and complete GPU residency. Reading that number is not proof of
presentation or absence of every leak. When adapting instrumentation, preserve
actual renderer identity, ownership/main-thread checks, honest sampled labels,
and failure on missing observations.
