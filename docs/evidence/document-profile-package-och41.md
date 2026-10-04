# Public document profile package — OCH-41

Local macOS arm64 evidence on `83eb87e865c86717a8bc51b9db6fe1f379d909a9`
plus the working tree. This follows the [native renderer integration](document-profile-renderers-och41.md).

## Public application path

`examples/document_profile_package` is a separate OCaml/Rust package. It imports
only public `Gpuio.Document.Profile` and `gpuio-document-sdk` APIs. The SDK exposes
its exact pinned Base primitives through `sdk::base`; the package does not import
host implementation modules or private parser state. It supplies two checked accent
properties, four typed native events, whole-code highlighting, code/table actions,
an inline review badge and a block review card.

A canonical fixed-byte schema has SHA-256
`985f53077246b78111f091454a0d6f07c770cab3c3cb9218c114e4936f52a26d`.
Both languages use that fingerprint. The property/event codecs validate exact
lengths and tags. The package documents trusted hook ownership, queued callbacks,
pointer versus keyboard guards and truthful NonText semantics.

The gallery's existing generated backend now composes the counter and document
profile catalogs atomically. Markdown/HTML expose an optional profile and accent
switch, native actions and typed event/failure notices with source revisions. A
separate optional Markdown document demonstrates inline/block controls; the original
Markdown sample remains intact. Its source belongs to the same cancellable preview
scope. These are source packages on the experimental release train, not published
registry packages or dynamic plugins.

## Checks

Two Rust package tests pass: exact property validation/cancellation, and actual
prepared Markdown with both plugin families, preserved source and distinct full
highlight styles. The OCaml expect test passes for both property bytes, all four
event tags, malformed payloads and invalid generation. The initial expected inline
record S-expression had extra parentheses; it was manually corrected against the
observed derived representation.

Full `@all @runtest`, strict document SDK/package lint and six backend-generator
tests pass. The root Cargo lockfile adds only the local example package; the gallery
backend lockfile also adds its local dependency edge. Every preexisting third-party
package record is unchanged. No dependency pins, switches or vendored sources changed.

The independent-consumer script copies both packages to a new workspace, generates
an external backend, builds against staged installed OCaml libraries, runs the
copied expect test, then launches the gallery with `--check-catalogs`. That mode
validates both schemas in a fresh linked process and exits without a window.
The first independent run passed at
`/private/tmp/gpuio-document-profile-render-gallery-20261004`; the final optional
preview layout also passed in `/private/tmp/gpuio-document-profile-gallery-final-20261004`.
Both reported `GALLERY_CATALOGS_PASS counter=1 document_profile=1` and
`INDEPENDENT_EXTENSION_CONSUMER_PASS example=gallery run=False`. Final targeted
gallery/expect checks, official formatting, catalog audit and whitespace checks pass.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j 2 -p gpuio-example-document
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @all @runtest
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --offline --locked -j 2 \
  -p gpuio-example-document -p gpuio-document-sdk --all-targets -- -D warnings
python3 scripts/test_compose_backend.py
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 \
  examples/gallery/main.exe @examples/document_profile_package/test/runtest
GPUIO_JOBS=2 ./scripts/gpuio check-fmt
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery \
  --workspace /private/tmp/gpuio-document-profile-gallery-final-20261004
python3 scripts/audit_component_catalog.py
git diff --check
```

The copied tests execute against installed public OCaml libraries. Backend Rust
sources use the selected pinned checkout as designed; this is not a binary-only
Rust distribution. No `--run` GUI driver, physical input/clipboard/VoiceOver or GPU
qualification is claimed. The gallery README records the remaining native
walkthrough. Application defaults, virtual offscreen focus/clipping qualification,
remaining catalog coverage and all OCH-17 release gates stay open.
