# How `Native` exposes the private Rust experiment

[README](README.md) · [Source](native.ml) · [Caller](main.md)
· [Rust exports](../../rust/foundation/src/bridge.rs)

This module is nine OCaml `external` declarations binding `gpuio_*` C symbols.
It has no Bonsai graph, PPX model, effect scheduler, or runtime abstraction of its
own. The foundation Dune rule builds `libgpuio_foundation.a` through
`build_native.py` and links its generated platform flags. It is separate from the
public `Gpuio_native` handle API and not a recommended production interface.

`init` takes the wake pipe descriptor. Rust duplicates it, owns the duplicate,
and sets nonblocking/close-on-exec flags. The OCaml caller closes its original
write end but retains the read end while the worker/native runtime run.
`run` executes GPUI on the OS main thread, releasing the OCaml runtime during
that call. Rust emits queued events and writes wake bytes; a full nonblocking
pipe means a wake is already pending, not that event payloads live in the pipe.

`submit : bytes -> bytes` decodes a private batch and attempts admission to a
bounded 64-command channel. An empty result means success; nonempty bytes contain
an error string. This is not a typed Result or a guarantee the batch was applied.
`drain` removes queued Rust events and serializes them as a private event list.
The worker uses [`Wire.decode`](wire.md) to interpret them. Rust's event FIFO is
separate from the bounded command channel; do not infer public mailbox/resource
limits from this prototype.

`probe` posts diagnostic commands. Positive IDs activate fixture controls;
negative codes select special probes handled by the native experiment. `quit`
requests termination; its Rust implementation ignores channel-admission failure,
so this prototype helper is not the public emergency-abort contract.
`cleanup` releases the wake descriptor and queued events after the caller joins
the worker. The bridge uses a process-static `OnceLock`, not reusable per-run
application handles with generations.

`test_panic` deliberately raises a Rust panic to check the export boundary maps
it to an OCaml failure. `run_two_windows` invokes a separate native-only smoke
path, releases the OCaml runtime while running, and cleans up afterward. It does
not exercise the foundation Bonsai component, keyed callbacks, or file task.

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/foundation/main.exe
GPUIO_JOBS=2 ./scripts/gpuio smoke --self-test
GPUIO_JOBS=2 ./scripts/gpuio smoke --two-windows
```

Use the [repository toolchain/native prerequisites](../../docs/development.md).
There is no separate `native.exe`. Dune selects the pinned Cargo package/lock and
archive; do not compile unrelated native switches or manually substitute ABI
symbols. These launches are graphical diagnostics, not unit-only tests or
physical input acceptance. For application code, use
[the public runtime](../../lib/eio/app.mli), whose scopes and exact lifetimes
replace these process-global experimental responsibilities.
