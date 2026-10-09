# Native review profile

Read the [OCaml consumer API](ocaml/gpuio_example_document.md),
[Rust factory](rust/src/lib.md), [native scroll plugin](rust/src/scroll_card.md),
and [OCaml expect test](test/document_profile_example_test.md) for source-level
walkthroughs, ownership, exact commands, and evidence limits.

An independent static document profile built only against `gpuio-document-sdk`
and the public OCaml `Gpuio.Document.Profile` API. The gallery Documents page
links it alongside the separate counter component. Enable **Native document
profile** for Markdown or HTML; **Amber code highlights** changes its properties.

The example supplies whole-code coloring, native code/table action buttons and three
Markdown plugins: inline `` `review` ``, fenced `review-card`, and fenced
`review-scroll` blocks. HTML uses
highlighting/action slots without Markdown AST plugins. The review controls are
NonText objects; their labels are native accessibility labels, not selectable or
searchable document glyphs. Ordinary surrounding text retains reader ownership.
This is an authoring example, not a syntax grammar or Markdown execution engine.

## OCaml application use

```ocaml
let profile =
  Gpuio_example_document.instance ~accent:Indigo ~generation:1L
  |> Core.Or_error.ok_exn
in
Gpuio_bonsai.View.with_document_profile document_view profile
  ~on_event:(fun event ->
    match event.signal with
    | Data Inspect_code -> show_code_details
    | Data Summarize_table -> show_table_summary
    | Data Open_badge | Data Open_card -> show_review
    | Failed { stage; error } -> show_failure stage error)
|> Core.Or_error.ok_exn
```

The snippet assumes application-defined Bonsai effects and a Markdown/HTML
`document_view`. The instance has no mutable native handle. Properties are checked
bytes; changing them retires callbacks from the previous profile. Increasing its
generation explicitly requests a reset. Events include the installed source
revision/generation, and the Eio bridge applies normal source lifetime fences.

`schema.txt` is the canonical fixed-byte schema; its SHA-256 is the fingerprint in
both packages. Property bytes are exactly `00` (indigo) or `01` (amber); event bytes
are `01..04`. OCaml expect tests and Rust preparation tests cover these contracts.
The enclosing transport remains the versioned GPUIO binary protocol.

## Static backend composition

Add the package to a trusted application's backend manifest:

```json
"document_profiles": [
  { "path": "../document_profile_package/rust", "factory": "factory" }
]
```

Generate the backend with `scripts/compose_backend.py`, review its Cargo lockfile,
and select that backend's Dune library plus `gpuio_example_document`. Generated
initialization installs component and profile catalogs together before creating the
application. Do not install a second catalog after one has been frozen. The gallery
uses `examples/extension_consumer/native.json` as a complete manifest example.

Native hooks use the exact Base primitives re-exported as `sdk::base`; the pinned
SDK owns dependency compatibility. `guard_pointer` handles mouse/touch callbacks,
while `guard` handles keyboard/AX and unwinding panics. Delivery only queues typed
events. No closure enters OCaml during Rust layout or painting. Do not retain
Window/App references or perform I/O in parser/highlighter hooks. Plugins must
honor cooperative cancellation and honestly declare additional opaque retention.

## Validation and scope

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j 2 -p gpuio-example-document
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @examples/document_profile_package/test/runtest
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --workspace /private/tmp/gpuio-profile-gallery-check
```

The consumer workspace must be new. Without `--run`, the last command builds an
outside-checkout application against staged installed OCaml libraries, without
modifying any opam switch. It does not claim native input/VoiceOver/GPU acceptance.
Application-default profiles and release/platform qualification are separate
from these package tests; use the current public contracts and evidence for status.


The [real macOS gallery walkthrough](../../docs/evidence/document-profile-macos-och41.md)
now exercises code/table keyboard actions, inline-to-block Tab navigation, pointer
activation after property changes, profile removal/remount and page resource
release. Run `python3 scripts/test_macos_document_profile.py --output scratch/profile-native-001`
after building the gallery; it owns a foreground test window. The gallery's
`--trace-document-profile` option logs typed signals and source revisions for this
check. The public `review-scroll` block now demonstrates a bounded independent
viewport with ordinary native **Show review start/end** buttons outside it. Both
buttons work with Tab/Enter; the inner action emits the existing `Open_card`
event. Scrolling itself emits no application event. The outer document still owns
only its own reveal; a clipped inner action is skipped until the plugin reveals it.

The native scroll handle is keyed by parsed occurrence and source generation in
the document's element namespace. It survives consecutive mounted frames and
property updates; unmount releases that state. This is transient view state, not
persistent application state across eviction/removal. Parser data stays immutable,
and current revocable event guards protect both local scroll actions and emitted
actions. No new OCaml/Rust wire schema or dependency is required.

The physical walkthrough covers native wheel input, keyboard alternatives,
clipped traversal, property changes and unmount; arbitrary plugin composites,
trackpad momentum and VoiceOver remain separately scoped. See
[the evidence](../../docs/evidence/document-profile-scroll-och41.md#public-scroll-profile--2026-10-05).
