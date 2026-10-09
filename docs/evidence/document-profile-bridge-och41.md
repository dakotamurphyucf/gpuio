# Document profile bridge — OCH-41

Local implementation checkpoint on macOS arm64, based on
`83eb87e865c86717a8bc51b9db6fe1f379d909a9` plus the current working tree.
This is not OCH-41 or milestone 07 acceptance. Profiles have checked public
attachment/catalog contracts, but the native reader does not yet apply them.
Worker/render integration, defaults, gallery, focus and physical qualification
remain open. See the [contract](../design/document-profiles.md).

## Implemented bridge

- Core `Document.Profile` definitions bind validated schemas to bounded property
  and event codecs. Instances have positive application generations. Core/Bonsai
  `View.with_document_profile` rejects non-reader and non-Markdown/HTML views.
- Op121 carries an optional profile plus a strictly increasing host epoch.
  Clearing preserves the epoch watermark. Generation cannot decrease while the
  same schema remains installed; clear/reinstall starts a new installation.
- Event78 carries host/application/source identity and bounded typed data or a
  stage/error. Core rejects obsolete callbacks and uses the latest callback when
  the wire value is unchanged. Invalid typed payloads become Input/Invalid_event.
  Eio routes these events through the existing source-registry admission check.
- Native Session admission checks the linked schema and pure property validator
  before atomic publication. Mode, handler and epoch constraints are validated
  on the candidate tree. Failed transactions preserve the old revision/retention.
- Components and document profiles share one immutable registration boundary.
  Both registries validate before either freezes. Catalog queries or legacy
  category-specific installation freeze both; mixed packages use the combined
  installer before querying or creating a transport.
- Backend manifests accept optional `document_profiles`. Profile-only manifests
  work with `components: []`; packages providing both kinds share a Cargo alias.
  Existing backend registration/Cargo files remain identical, with Dune output
  equivalent apart from formatter whitespace. Generation refuses changed outputs.
- SDK descriptors now declare opaque retained allocations, capped at 4 MiB.
  Host reservations are still to implement; this declaration is not an allocator
  measurement or proof of bounded runtime resources.

## Validation

Executed through the repository's isolated switch, with `GPUIO_JOBS=2`:

```sh
./scripts/gpuio exec cargo test --offline --locked -j 2 \
  -p gpuio-document-sdk -p gpuio-protocol
./scripts/gpuio exec cargo test --offline --locked -j 2 \
  -p gpuio-native --test document_profile --test extension_catalog
./scripts/gpuio exec dune build -j 2 @all @runtest @fmt
./scripts/gpuio exec cargo clippy --offline --locked -j 2 \
  -p gpuio-native -p gpuio-document-sdk -p gpuio-protocol \
  --features gpuio-native/native-image-tests,gpuio-native/native-canvas-tests \
  --all-targets -- -D warnings
python3 scripts/test_compose_backend.py
./scripts/gpuio check-fmt
python3 scripts/audit_component_catalog.py
git diff --check
```

SDK **15** and protocol **382** tests pass, as do both native catalog/admission
integration tests, full OCaml build/tests/format, strict feature-enabled lint and
six portable generator tests. The new Core tests cover paired binary encodings,
malformed typed data, mode admission, latest callbacks, property changes, clear/
reinstall, generation rollback, source mismatch, future tree revisions and unmount.
Protocol checks include every truncation, trailing bytes, strict option tags,
schema bounds and binary property/event limits.

An initial Clippy command omitted native test features and hit existing carousel
helpers used only with those features; the command above passes. The first full
native run passed 852 tests and failed the new mailbox fixture because its assumed
capacity ignored the existing 128-event cap. The corrected fixture respects both
count and byte limits. The full native suite then passed **853** tests with two
existing macOS private-bus skips:

```sh
./scripts/gpuio exec cargo test --offline --locked -j 2 -p gpuio-native \
  --features native-image-tests,native-canvas-tests --lib
```

The profile queue test verifies binary payload charging, ordered drain batches,
count/byte admission and delivery before Stopped. A fresh independent gallery
consumer passes against staged installed public libraries (no opam switch install):

```sh
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery \
  --workspace /private/tmp/gpuio-document-profile-bridge-gallery-20261004
```

This used `run=False`; it verifies public-library compatibility, not profile UI.
A generated profile-only backend containing a real SDK factory also
passes offline `cargo check --locked`; this compilation probe does not establish
rendering or event behavior.

Root and both example backend lockfiles were compared structurally with their
pre-bridge snapshots: only the local SDK package/native dependency edge changed;
all third-party package records are identical. The 234 GPUI Base files remain
identical to the verified document-actions reconstruction. No OS window was opened by these
checks. TestPlatform, compilation and source audit are not physical keyboard/IME,
VoiceOver, Linux desktop or release acceptance.

## Next implementation boundary

Add profile identity to document worker requests and reserve declared opaque
retention plus generated strings, highlights and adapter allocations before
configuration. Wait for a positive published source revision. Install prepared
text, profile, plugin interpretation and complete highlighter output together;
preserve exact weight/strike styling. Rebuild image/render closures without
changing parser identity unnecessarily. Revoke event leases on obsolete source,
properties, visibility, modal and unmount transitions; report failures once.
Then add native rendering/focus/search/selection/copy tests, a real profile package,
its installed consumer and the public gallery. Application defaults remain a
separate required contract.
