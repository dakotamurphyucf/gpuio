# How the Rust profile factory supplies native document hooks

[Package README](../../README.md) · [Source](lib.rs)
· [OCaml consumer codec](../../ocaml/gpuio_example_document.md)
· [Scroll plugin](scroll_card.md)

This is extension-author Rust code. It imports only the public pinned
`gpuio-document-sdk`, using its GPUI/Base re-exports. It is a static source package,
not a dynamically loaded binary plugin or an OCaml Bonsai component.

`factory()` returns `Arc<dyn sdk::Factory>`. `descriptor` pairs name/version/
fingerprint with SDK, GPUI, and Base revisions; properties/events are each bounded
to one byte and declared retained bytes to 65,536. `validate_properties` accepts
exactly one byte 0/1. `configure` checks cooperative cancellation, validates bytes,
and constructs a Profile combining action renderer, highlighter, inline badge,
block card, and scroll-card plugins.

The composer registers this factory before catalogs freeze. Defining a factory
or OCaml schema does not itself install it. The manifest in
[extension_consumer/native.json](../../../extension_consumer/native.json) includes
both components and document_profiles, and generated backend initialization owns
catalog installation. Review generated files and Cargo.lock; do not install a
second catalog after freezing.

`button` builds a native Base button with accessible label and one-byte event.
Mouse/touch callbacks use `guard_pointer`; keyboard/AX use `guard`. Revocable
sinks prevent hidden/obsolete callbacks from acting and contain ordinary unwinding
panics. Emission queues bytes asynchronously; it never calls an OCaml closure from
native layout/paint. `ActionRenderer::code/table` return buttons emitting 1/2.

`Highlighter::highlight` colors the entire nonempty code byte range, using weight
600 and accent-specific RGB. This is whole-code coloring, not token syntax parsing.
The inline plugin recognizes exactly inline code `review`, preserving original
Markdown source and rendering a NonText badge emitting 3. Card recognizes fenced
`review-card`, rendering a NonText block button emitting 4. `render_inline` returns
an inline element only for the badge; block Card returns None. HTML uses action/
highlighter slots, not these Markdown AST recognizers.

Parsed data is immutable and preparation hooks use worker-side contexts. Native
render hooks receive temporary Window/App references; retain no such references
or perform I/O in them. NonText labels are native accessibility text, not selectable
or searchable document glyphs. The host retains surrounding reader ownership.
See [SDK profile contract](../../../../rust/document-sdk/src/profile.rs).

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-example-document
```

Use [development setup](../../../../docs/development.md). The crate has no binary.
Tests exhaust property bytes/lengths, check cancellation, and run actual Markdown
preparation to verify three plugins, source/parser epoch, code block/highlights,
and palette change. They do not mount controls or generate keyboard/pointer/AX
input. The README links separate macOS gallery evidence and harnesses.

To compose a trusted consumer, use `python3 scripts/compose_backend.py manifest.json
generated-directory` with paths relative to that manifest, review its output/lock,
and select its generated Dune backend. For new hooks, preserve exact schema pairing,
cooperative cancellation, retention accounting, and revocable event guards; do
not infer platform acceptance from successful crate/consumer compilation.
