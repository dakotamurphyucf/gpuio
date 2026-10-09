# Isolated production codec diagnostic — OCH-17

Measured 2026-10-06 UTC on Apple M1 Max, 32 GiB RAM, macOS 14.5 arm64 at
`04bdfd719689f33365ab954e88afa15f4e63dd7e`. Stock OCaml 5.3, Core v0.17,
Dune release profile; Rust 1.97.1 Cargo release profile. No native window or
simultaneous local GPUIO build/test was running during measurement. This is a
short diagnostic on the owner's ordinary desktop, not controlled release
qualification or a React/DOM comparison.

The OCaml probe calls the current `Wire.Message.encode` on prebuilt `Apply`
messages containing `Set_text` operations. The Rust probe decodes the exact OCaml
bytes with production `gpuio_protocol::decode`, consuming and dropping the owned
result every iteration. There is no alternative test codec. Each case has 100
warmups, then 11 timed batches; reported values are the median **batch mean per
message**, not single-message p50 or tail latency. Two sequential process runs
are retained. OCaml timing includes validation, allocation, binary writing and
Bigstring-to-string copying. Rust timing includes decoding, owned allocations
and disposal. Fixture file I/O, message construction and warmups are untimed.

| Workload | Encoded bytes | Encode μs, run 1 / 2 | Decode μs, run 1 / 2 |
| --- | ---: | ---: | ---: |
| One 8-byte text update | 18 | 0.236 / 0.139 | 0.264 / 0.104 |
| 100 updates, 64-byte text each | 6,806 | 7.385 / 7.378 | 8.285 / 8.173 |
| 1,000 updates, 64-byte text each | 69,752 | 76.071 / 75.603 | 82.890 / 84.862 |
| One 256-KiB text replacement | 262,158 | 226.206 / 220.995 | 13.629 / 13.273 |

Loops per batch respectively: 10,000, 1,000, 100, 100. The large string and repeated
small strings are ASCII. They do not represent complex widget/property mixtures,
Unicode-rich data, changing allocations across application state, or cold caches.
The two sub-microsecond results vary substantially, so do not treat their decimal
precision as a stable latency guarantee. Summing encoding and decoding medians
suggests about 16 μs of codec work for 100 updates and 160 μs for 1,000, but those
sums are not measured end-to-end bridge latencies.

## What this does and does not measure

The production FFI submission borrows the OCaml string as a byte slice, decodes
it synchronously into owned Rust messages, and submits those through a bounded
in-process mailbox. It does not transmit serialized payloads over a socket or to
a separate process. Encoding currently writes a Bigstring and then copies it to
an OCaml string. Reply encoding/copying, decoding allocations, mailbox locks and
wakeup coordination remain real costs.

This experiment **does not** time the FFI call, mailbox/wakeup latency, waiting
for the main thread, native session/tree admission, GPUI layout/paint, response
round trips, GPU execution or display presentation. Fixture messages reference
synthetic node IDs; they are valid codec inputs, not complete live-tree updates.
No claim of native tree admission or a working 256-KiB text widget follows.
It also excludes Bonsai stabilization and reconciliation. The dedicated production
performance workloads remain necessary; this evidence does not close OCH-17.

## Reproduction and source preservation

The ignored scratch probe copied the protocol OCaml source unchanged into a
standalone Dune project using the existing repository switch. The Rust package
uses a path dependency on the production protocol crate. Its private Cargo.lock
is retained; no repository dependency pin, opam default or switch was changed.
Builds used two jobs and completed before the sequential measurements.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build \
  --root scratch/agents/root-20261004-resumed/bridge-codec-probe \
  --profile release -j 2 main.exe
GPUIO_JOBS=2 ./scripts/gpuio exec cargo build --offline --release -j 2 \
  --manifest-path scratch/agents/root-20261004-resumed/bridge-codec-probe/rust/Cargo.toml
# From that scratch probe directory, repeat twice:
./_build/default/main.exe
/Users/dakotamurphy/gpuio/target/release/gpuio-codec-probe
```

[Probe source, private lock, exact byte fixtures, build logs and both results](bridge-codec-diagnostic-och17/validation.tar.gz)
are retained with a [checksum manifest](bridge-codec-diagnostic-och17/manifest.json).
Absolute paths in the local probe Cargo.toml must be adjusted when reproducing on
another checkout. Protocol-source hashes identify the unchanged copied inputs;
the Git revision supplies those versioned source files.
