# GPUIO document SDK

Static Rust profile definitions for rich document parsing, highlighting and native
rendering. The experimental host path now includes checked Core/Bonsai attachment,
worker preparation, native renderers and queued events with revocable ownership.
Local TestPlatform cases exercise these paths. A [public example package](../../examples/document_profile_package/README.md)
and gallery pass an independently installed consumer build, copied expect tests and
fresh-process catalog checks. [Application defaults](../../docs/design/document-defaults.md)
are carried by the OCaml runtime; virtual focus and physical qualification remain open. See the
[implementation contract](../../docs/design/document-profiles.md).

The registry validates pinned dependency identity and schema fingerprints before
retaining bounded property bytes. Hosts must configure immutable profiles on workers;
the SDK itself does not schedule them.
`prepare_markdown` and `prepare_html` apply checked plugin adapters, the reader's
bounded parser and aggregate checked code highlights. Preparation failures include
a Parse/Highlight stage; completed results include separate phase timings. HTML requires the host's
image replacement callback; Markdown plugins do not intercept HTML ASTs.

Plugins must declare Text, NonText or Opaque presentation. The reader owns glyphs,
selection and search for Text; arbitrary renderers cannot claim those properties
merely by supplying a label. Preparation rejects invalid names, failed hooks and
excess generated strings rather than quietly treating an error as a declined match.
Render adapters expose their first failure for host reporting and render an alert
fallback; they are not proof of host lifecycle or accessibility qualification.

Limits: 64 profiles, 32 plugins per profile, 64 KiB properties/source/code, 16 KiB
events, 32,768 highlight runs across a preparation and 1 MiB generated public node
strings. Base additionally bounds line length, structure and displayed glyph bytes.
Each descriptor declares additional opaque retention (at most 4 MiB); this is a
trusted author declaration, not a measurement of arbitrary Rust allocations. Host
reservations and cancellation/install/event fencing are implemented in the host;
they remain requirements for any alternative host. Cooperative cancellation and panic containment cannot
preempt malicious Rust or recover aborts/unsafe memory corruption.

Use the repository toolchain:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j 2 -p gpuio-document-sdk
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --offline --locked -j 2 \
  -p gpuio-document-sdk --all-targets -- -D warnings
```
