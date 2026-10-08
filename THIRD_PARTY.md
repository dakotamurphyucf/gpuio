# Third-party provenance

GPUIO is Apache-2.0. Imported project research/design documents retain links to
their upstream sources. Dependency licenses remain with their respective authors.

`rust/foundation/src/text_input.rs` adapts Zed GPUI's `examples/input.rs`, copyright
Zed contributors, Apache-2.0, at revision
`a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b`. Changes add bridge events, edit revisions,
composition-selection correction, construction helpers, an input probe and an
internal text accessor for the two-window smoke scenario. The original Zed copyright
notice and license text are preserved in [third_party/licenses/zed-gpui.txt](third_party/licenses/zed-gpui.txt).

`vendor/` retains the upstream source and MIT licenses for eight Jane Street
v0.17 repositories. `third_party/sources.json` pins upstream commits, downloaded
archive hashes and compatibility patch hashes; `third_party/patches` contains the
reviewed native-library selection and reserved-identifier changes. A small Bonsai
driver extension captures a typed, single-use lifecycle snapshot for asynchronous
native acceptance, including when another window stabilizes the shared Incremental
universe. It preserves normal Bonsai lifecycle diff/reset behavior and does not
change Incremental or the clock interface. An optional action-history policy
releases the stabilization tracker cache after a completed batch; native windows
select it to bound retained keyed action paths. The existing default stays intact.
See `docs/design/runtime.md` and `docs/design/managed-lists.md`. Unbuilt browser source remains
in the snapshot for provenance; the Dune subset excludes it from compilation.

GPUI/GPUI platform dependencies are Apache-2.0; ocaml-interop and binprot-rs retain
their upstream licenses in Cargo's pinned source checkouts. Cargo.lock records the
full transitive dependency set. These dependency licenses are independent of
GPUIO's project license. Run `cargo metadata --locked` when auditing the closure.

Vendored example assets can have different terms from their surrounding code.
The GPUI SVG example includes **Dragon clip art.svg** by **Ebaychatter0**, under
CC BY-SA 3.0. The [asset notices](third_party/ASSET_NOTICES.md) retain its source,
license link, attribution and byte-verified provenance. GPUIO has not changed the
artwork or its existing upstream source/license comments.

`third_party/notice-sources.json` records exact supplemental notice attributions
for pinned packages lacking separate package-local files. Zed Apache symlinks
were compared byte-for-byte with `third_party/licenses/zed-gpui.txt`; the nested
binprot and ocaml-interop derive crates have unchanged workspace texts retained
as [binprot-rs](third_party/licenses/binprot-rs.txt) and
[ocaml-interop](third_party/licenses/ocaml-interop.txt). First-party crates refer
to GPUIO's root license. Each entry includes source/manifest/text hashes and an
attribution rationale. These collection inputs do not select dual-license
alternatives or establish a complete distribution notice set. See the
[notice audit](docs/evidence/dependency-notices-och17.md).

`third_party/licenses/registry/` retains additional unchanged upstream notice
files from exact revisions recorded by the published crates' `.cargo_vcs_info.json`.
The attribution manifest links each package to its source URL, Git blob SHA-1,
local manifest hash and notice SHA-256. These include AccessKit's additional
Chromium notice as well as dual-license files; collecting them does not choose
an alternative or assert that source/asset obligations are fully reviewed.

The zune-inflate attribution additionally verifies the packaged original manifest
against its exact recorded upstream commit; retained files include its full Zlib
text and root copyright/licensing policy. This does not select a license alternative.

The OCaml collector separately captures installed package metadata, source/doc
notices and all eight named native Bonsai vendor roots. The OCaml 5.3 source and
installed runtime license texts match byte-for-byte, including the original linking
exception. Tool/configuration packages and native/system/assets still require
review; see [OCaml notice evidence](docs/evidence/ocaml-notices-och17.md).

`vendor/gpui-base` retains GPUI Kit's Apache-2.0 sources/license at
`84f57fdfcb4910623fb0bb7f795b077e249f9271` (0.6.1). Its original manifest and
upstream README are retained. The adapted manifest unifies GPUI/macros/sum-tree
with GPUIO's pinned Zed source; patches are recorded in
`third_party/patches/gpui-base.patch`. Source/patch hashes and the reconstruction
script preserve provenance. This imports native behavior without the JavaScript
shell or the styled component facade. See `docs/design/native-editor.md`.

Display-document highlighting uses Syntect5.3.0 and Two Face0.5.2+bat-0.26.1,
locked in Cargo.lock with registry checksums. Their licenses are retained in
[Syntect](third_party/licenses/syntect.txt) and
[Two Face](third_party/licenses/two-face.txt). The bundled syntax definitions and
themes include additional upstream notices; preserve
[two-face-assets.md](third_party/licenses/two-face-assets.md) with distributions
that contain those assets. It is the pinned tag's generated acknowledgements.

GPUIO's Base adaptations also expose externally prepared bounded Markdown,
selection transfer across compatible snapshots, safe reference-image resolution,
and a source line origin for read-only document pages. They are recorded in the
same source patch; GPUIO's worker/transport code remains outside the vendor tree.

`rust/plot` retains the Sankey layout and ribbon geometry from the same Longbridge
commit, independently of its styled component crate. Its [source provenance and
adaptations](rust/plot/UPSTREAM.md) and [Apache-2.0 license](rust/plot/LICENSE-APACHE)
remain with the extracted sources. The validated GPUIO chart adapter lives outside
that extraction.

`docs/catalog/sources` contains unmodified documentation snapshots from Longbridge
GPUI Kit `84f57fdfcb4910623fb0bb7f795b077e249f9271` and GPUIX
`18e695ed0ee8121a7793413ca795e08eda2a13df`, both Apache-2.0. Their complete license
texts and an exact-source/SHA-256 manifest are retained in that directory. These
entry-point/contract snapshots support the OCH-41 audit and are not new compiled
or runtime dependencies.

`rust/native/src/text_shimmer_paint.rs` and `text_shimmer_color.rs` adapt the
glyph-mask approach and Oklab mixing from that same Longbridge revision's
`crates/component/src/shimmer.rs` and `theme/color.rs` (copyright 2024–2026
Longbridge, Apache-2.0). Changes delegate layout to GPUIO's existing StyledText,
take a native phase sample, validate bounded work, preserve color emoji and
expose diagnostic paint reports. The original notice/license is retained in
[`docs/catalog/sources/gpui-kit-LICENSE`](docs/catalog/sources/gpui-kit-LICENSE);
the shimmer source snapshot is recorded in the adjacent manifest.
