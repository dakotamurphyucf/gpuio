# How the expect test checks profile bytes and failures

[Package README](../README.md) · [Source](document_profile_example_test.ml)
· [OCaml package](../ocaml/gpuio_example_document.md)

This pure OCaml inline test uses `Gpuio.Document.Profile.Expert` to inspect the
paired codec contract. It opens no runtime/window and registers no Rust factory.
`let%expect_test` and `[%expect]` are Jane Street test PPX, not reactive Bonsai
syntax. There is no graph, state setter, asynchronous effect, or Eio capability.

For Indigo and Amber, it constructs generation-1 instances, converts to wire,
and prints property bytes as integer lists. Expected output is 0 and 1.
It then constructs synthetic wire events with config epoch 1, instance generation
1, source revision 2, and source generation 1. Payloads 1–4 decode to the four
typed action names. Empty, unknown byte 5, and two-byte input produce
`Failed (stage Input) (error Invalid_event)` signals rather than valid Data.
Generation 0 must fail instance construction.

This distinguishes structural instance errors (`Or_error`) from malformed native
payload signals after conversion. The fabricated provenance fields do not prove
actual event lifetime fencing: that belongs to host integration. Output equality
checks codec pairing, not rendered buttons, Markdown plugins, cancellation,
source retirement, native focus, or physical accessibility input.

From the repository root in the [repository environment](../../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune runtest -j2 examples/document_profile_package/test
```

[Dune](dune) enables inline tests, ppx_jane, and ppx_expect and links the Core
package/protocol. There is no test executable to launch manually and no graphical
session needed by this path. Rust factory/preparation tests and the gallery
harness are separate, described in [the Rust guide](../rust/src/lib.md).

When changing the schema, update both codecs/fingerprint and author-side contract
checks. Add tests for exact accepted lengths/values and typed rejection, while
keeping native lifetime/interaction claims grounded in actual host evidence.
