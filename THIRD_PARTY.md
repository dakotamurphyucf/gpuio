# Third-party provenance

GPUIO is Apache-2.0. Imported project research/design documents retain links to
their upstream sources. Dependency licenses remain with their respective authors.

`rust/foundation/src/text_input.rs` adapts Zed GPUI's `examples/input.rs`, copyright
Zed contributors, Apache-2.0, at revision
`a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b`. Changes add bridge events, edit revisions,
composition-selection correction, construction helpers, an input probe and an
internal text accessor for the two-window smoke scenario.

`vendor/` retains the upstream source and MIT licenses for eight Jane Street
v0.17 repositories. `third_party/sources.json` pins upstream commits, downloaded
archive hashes and compatibility patch hashes; `third_party/patches` contains the
reviewed native-library selection and reserved-identifier changes. No lifecycle
or Incremental runtime workaround is included. Unbuilt browser source remains
in the snapshot for provenance; the Dune subset excludes it from compilation.

GPUI/GPUI platform dependencies are Apache-2.0; ocaml-interop and binprot-rs retain
their upstream licenses in Cargo's pinned source checkouts. Cargo.lock records the
full transitive dependency set. These dependency licenses are independent of
GPUIO's project license. Run `cargo metadata --locked` when auditing the closure.

`vendor/gpui-base` retains GPUI Kit's Apache-2.0 sources/license at
`84f57fdfcb4910623fb0bb7f795b077e249f9271` (0.6.1). Its original manifest and
upstream README are retained. The adapted manifest unifies GPUI/macros/sum-tree
with GPUIO's pinned Zed source; patches are recorded in
`third_party/patches/gpui-base.patch`. Source/patch hashes and the reconstruction
script preserve provenance. This imports native behavior without the JavaScript
shell or the styled component facade. See `docs/design/native-editor.md`.
