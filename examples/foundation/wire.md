# How `Wire` serializes private foundation batches and events

[README](README.md) · [Source](wire.ml) · [Caller](main.md)
· [Rust decoder](../../rust/foundation/src/protocol.rs)

This is the experiment's private bin_prot schema, unrelated to the current public
`Gpuio_protocol.Wire` version. It contains ordinary OCaml data and generated
serialization functions; no Bonsai graph, effect, view, Eio scope, or native handle
is created here. Keeping it adjacent to the Rust decoder makes the format's
actual role visible.

`op` is Upsert, Children, Remove, Root, or Edit. Upsert contains numeric node ID,
kind, text, and optional handler ID. Children supplies ordered numeric IDs.
Edit carries editor ID, expected native edit revision, and replacement text.
`batch` carries version, acknowledged base, next revision, and operations.
The main worker computes this diff; Rust validates/adopts it asynchronously.

`[@@deriving bin_io]` generates writers/readers; it is serialization PPX, not
reactive `let%arr`. `encode` uses `Bin_prot.Utils.bin_dump bin_writer_batch`,
converts the bigstring to a string, and then to bytes for the FFI. It performs no
native submission. The Rust decoder separately checks version 1, a 1 MiB batch
bound, container counts up to 4096, UTF-8 strings, option/operation tags, truncation,
and trailing bytes. Variant order is part of this private format; changing one
side alone breaks decoding.

`event` names Ready, Applied, Click, Text, Closed, Error, Probe, and Frame.
Applied acknowledges revision/node/op counts; Frame is a separate display
observation. Click includes displayed revision, although this worker primarily
checks its numeric committed handler. Text includes edit revision/content/
composition flag; the main handler does not use every field. Declaring a field
is not evidence the example validates its semantics.

`events` is an event list. `decode` converts bytes to a bigstring, invokes its
generated reader with mutable position, and asserts every byte was consumed.
Malformed or trailing input raises rather than returning a typed application
error. `[@@deriving sexp_of]` supplies readable event diagnostics. These private
reader/assertion choices should not be promoted into a public untrusted-input API.

Event flow is Rust FIFO → `Native.drain` serialized bytes → `Wire.decode` →
main handler → scheduled Bonsai effect/model change → new diff/encoded batch →
`Native.submit`. Native application and frame acknowledgements remain distinct;
serialization alone proves neither callback correlation nor physical presentation.

Build the owning executable and run its diagnostic from the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/foundation/main.exe
GPUIO_JOBS=2 ./scripts/gpuio smoke --self-test
```

Use [development setup](../../docs/development.md). There is no `wire.exe` or
separate expect suite here. The self-test submits malformed bytes to check Rust
rejection; Rust protocol unit tests separately cover oversized/truncated/trailing
inputs. Smoke also opens a real native window and uses synthetic probes, so it
is not physical IME or Linux desktop acceptance. New applications should use
current typed constructors/reconciler/App contracts instead of this historical
numeric schema.
