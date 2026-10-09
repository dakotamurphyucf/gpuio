# How the OCaml counter package constructs typed native instances

[Package README](../README.md) · [Implementation](gpuio_example_counter.ml)
· [Interface](gpuio_example_counter.mli) · [Rust author side](../rust/src/lib.md)

This consumer library uses `X = Gpuio.Extension`. It constructs checked schema,
codecs, and instance descriptions; it creates no native factory, application,
Bonsai graph, or mutable counter handle. Consumers select a statically composed
backend with the matching native factory before mounting its view.

`Properties.t` is abstract publicly. `Properties.create` validates value 0–100 and
step 1–10, defaulting step to 1. The property codec encodes exactly two bytes;
decoding checks length before indexing and goes through the constructor.
The one-byte value codec validates 0–100 on both encode/decode and is shared by
commands and events. These pure codecs do no I/O. The schema pairs name
`example.counter`, version 1, and SHA-256 of [schema.txt](../schema.txt) with Rust.
See [extension contract](../../../lib/core/extension.mli).

`definition` combines property/command/event codecs. `instance` optionally turns
`set_value:(sequence, value)` into `X.Command.create`, then builds a labelled
instance with generation and optional disabled flag. `let%bind.Or_error` sequences
validated results; it is not reactive Bonsai or an asynchronous effect bind.
Sequence/generation must be positive; command values are checked by their codec.

In [the consumer](../../extension_consumer/main.ml), `B.state` owns observed value
and command sequence. Its `let%arr` derives Properties/instance/current view from
those reactive values; `V.extension ~on_event` mounts the native package. Native
activation admits one-byte value event, Rust updates/refreshes its counter, host
delivery decodes Data, and the consumer setter effect changes Bonsai state/text.
The package itself contains no `let%arr` because it owns no application graph.

Properties update the existing native component and revoke old callbacks.
Increasing generation resets/disposes native state. An explicit Set command is
applied once per sequence; reusing a sequence with different bytes is invalid.
The consumer's “Set native value to 12” increments sequence while retaining
constant command payload 12. Disabled input does not redefine explicit command
admission policy. Handle Mounted, Command_completed, and Failed separately from
Data when deriving application behavior.

From the repository root using [development setup](../../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/extension_package/ocaml/gpuio_example_counter.cma
GPUIO_JOBS=2 ./scripts/gpuio build examples/extension_consumer/main.exe
_build/default/examples/extension_consumer/main.exe
```

The library build needs no GUI; the consumer uses the linked native backend and
a graphical session. No package executable exists. A composed catalog must match
`schema`; schema construction alone is no registration or acceptance evidence.
[Consumer README](../../extension_consumer/README.md) explains generation/build
steps and smoke limits. For another component, preserve validated domains, exact
byte lengths, stable schemas, and monotonic command/generation policy rather than
exposing raw native IDs to application users.
