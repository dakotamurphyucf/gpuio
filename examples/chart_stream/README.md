# Public chart streaming workload

This executable measures the public Core/Bonsai/Eio chart API. It opens one
foreground 800×400 chart and closes after the workload. It does not contact a
network service or mutate a global toolchain.

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/chart_stream/main.exe
python3 scripts/measure_chart_stream.py --output scratch/chart-stream
```

The runner records exact child CPU and peak RSS with `wait4`, hardware/toolchain
metadata, the application log and a JSON report. It terminates/reaps its child on
failure. The executable also has a 180-second Eio timeout. Builds must finish
before measurement; avoid concurrent compiler/GUI workloads when comparing runs.

The workload uses immutable stable-ID sinusoidal line datasets:

- Thirty 10,000-point updates with the default explicit envelope policy.
- Thirty 100,000-point updates with the same policy.
- Ten 100,000-point updates with exact rendering.
- Ten bursts of eight 100,000-point desired datasets. All eight are constructed
  before submission; the source scheduler must publish only the latest one.

Each of the 80 observed publications must become the current data revision,
prepare successfully, and produce a native render callback. Source counts and
exact retention are asserted. Burst revisions advance once, and submitted bridge
bytes show one dataset upload. Source registration charge returns to zero after
release; native worker/lifetime tests separately verify native reclamation.

The report separates dataset construction from update-to-render-callback latency.
Neither is a physical presentation measurement or an idle frame-rate benchmark.
Mesh vertices/quads describe native frame work; whole-process CPU includes OCaml
construction/validation, transport, native decoding/preparation and rendering.
Process peak RSS includes retained burst inputs and allocator/GC high-water state.
It must not be equated with chart plan bytes or source admission charges.

`native_queue_peak_bytes` is the exact maximum serialized-size charge of accepted
commands awaiting native dispatch, including non-chart messages. It excludes
currently executing work, output events and OCaml queued source values. The
separate `peak_source_charge` includes scheduler-owned desired, publishing and
accepted data/encoding reservations. Diagnostics sampling adds no bridge message.

Development builds are useful for regression comparison, not release throughput
claims. See [the measured baseline](../../docs/evidence/chart-streaming-och40.md).
