# How the OCaml profile package defines checked instances

[Package README](../README.md) · [Implementation](gpuio_example_document.ml)
· [Interface](gpuio_example_document.mli) · [Rust author side](../rust/src/lib.md)

This is a consumer-facing OCaml library, not an application or native renderer.
`P` abbreviates `Gpuio.Document.Profile`. It creates pure schema/codec/instance
values; none registers a Rust factory or opens a window. The statically composed
backend must independently supply the matching profile catalog.

`Accent.t` is Indigo/Amber. `Event.t` names Inspect_code, Summarize_table,
Open_badge, and Open_card. `schema` validates name `example.document`, version 1,
and the SHA-256 fingerprint of [schema.txt](../schema.txt). Rust uses the same
identity. A matching name alone does not prove compatible payload bytes.

The property codec has exact one-byte input: `00` is Indigo, `01` is Amber.
The event codec maps bytes `01`–`04` to typed actions and rejects all other
lengths/values. `P.Codec.create ~max_bytes:1` bounds payloads; callbacks are pure
validated package code, not filesystem/network operations. `P.Definition.create`
pairs these codecs with the schema. `instance ~accent ~generation` validates a
positive generation and encodes properties. See [profile contract](../../../lib/core/document_profile.mli).

A consumer binds current accent in its Bonsai graph, constructs an instance,
and attaches it to an eligible rich document with
`Gpuio_bonsai.View.with_document_profile ~on_event`. In the gallery,
[documents_page.ml](../../gallery/documents_page.ml) uses current warm-profile
state to choose Amber/Indigo. Reactive `let%arr` there derives a view from current
values; callbacks return `Bonsai.Effect` work rather than mutate during rendering.
This package itself has no Bonsai graph or `let%arr`.

Click a Rust code action: its revocable event sink queues one byte; native delivery
carries installed source revision/generation; the host fences obsolete callback
lifetimes; this codec decodes a typed Data signal; the consumer's effect updates
its application notice/view. Failed stage/error signals are separate from Data.
No Rust parser, AST, Window, or native handle crosses to OCaml.

Property changes revoke old callbacks. Increasing instance generation requests
reset; clearing/reinstalling also creates a fresh native epoch. The borrowed
profile instance is description, not an owner extending document source lifetime.
Keep source provenance when handling events instead of treating every Inspect_code
as referring to the newest document automatically.

From the repository root in the [repository environment](../../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/document_profile_package/ocaml/gpuio_example_document.cma
GPUIO_JOBS=2 ./scripts/gpuio exec dune runtest -j2 examples/document_profile_package/test
```

There is no package executable. [Expect tests](../test/document_profile_example_test.md)
check fixed bytes and typed failures, not native rendering/input. The package
README explains static backend composition and graphical harness prerequisites.
For adaptation, version/fingerprint the complete paired schema, keep codecs
bounded/pure, and use current checked instances rather than unchecked raw payloads.
