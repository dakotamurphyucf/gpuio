# Embedded asset notice review — OCH-17

2026-10-04, local macOS arm64, dirty checkout based on `83eb87e`. This review
identifies concrete asset paths and improves notice collection. It is not final
licensing approval, a complete transitive generated-data audit or release acceptance.

## Readable syntax acknowledgements

`rust/native/src/document_highlight.rs` loads
`two_face::syntax::extra_newlines`. The pinned package is
`two-face 0.5.2+bat-0.26.1`, using its fancy-regex syntax dump. The notice collector
already copied the package's MIT/Apache files and compressed
`generated/acknowledgements_full.bin`, but did not include the repository's readable
`third_party/licenses/two-face-assets.md`. Binary acknowledgement data alone is
not a useful readable notice bundle.

The checked-in supplemental manifest now explicitly attributes that unchanged
Markdown file to this exact package/manifest. It contains 83 path/text sections.
All **78** sections returned by the pinned crate's
`acknowledgement::listing().to_md()` match it byte-for-byte, including paths and
license text. The full listing's five additional sections are preserved: the
Sublime package license, GLSL, GraphQL, PureScript and github-sublime-theme. Upstream
documents that its embedded listing is a subset of the full published listing;
this check does not independently establish the five additional sections' origins.
No notice was removed or replaced to force an exact whole-document comparison.

The local registry archive matches Cargo.lock's SHA-256
`915be7adc2ff6f4338acbf71f3eda0a146f46003b992a1669c674308610efdac`.
The installed manifest, acknowledgement dump, selected syntax dump and exporter
source match that archive exactly. Relevant byte identities:

| Input | SHA-256 |
| --- | --- |
| `generated/acknowledgements_full.bin` | `e4bf45ad159b67de6a7152874904728adb6421996eb15da01ca75406e6cd51fb` |
| `generated/syntaxes-fancy-newlines.bin` | `758bf07782d9d81479db7a0c4651aebf844ef20510ceb5bee08c61158368b0f3` |
| Exported embedded Markdown subset | `9bccc030238fc6c60c3a256eb5c15b2d5eef1f888f62d71088432fb980917fcd` |

The full listing's hash and exact manifest selector are in
[`notice-sources.json`](../../third_party/notice-sources.json). Reproduce the
upstream embedded export without a GUI or dependency changes:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo run -p gpuio-native \
  --example document_notices --offline --locked -j2 > /path/to/embedded-notices.md
```

This example prints only upstream acknowledgement data. It does not select license
alternatives, generate notices for unrelated dependencies or certify completeness.

The example builds and its output has the subset hash above. Ten portable notice
collector tests pass. Strict Clippy for the example and repository formatting
pass (`assets-notices-clippy-001.log`, `assets-notices-format-001.log`). Repeating offline locked collection with the supplemental
manifest for the three existing macOS dependency graphs gives:

| Root | Packages | Copied files | Supplemental packages | Packages without text |
| --- | ---: | ---: | ---: | ---: |
| `gpuio-native` | 513 | 862 | 49 | 28 |
| `gpuio-signal-backend` | 485 | 820 | 49 | 28 |
| Gallery `gpuio-counter-backend` | 486 | 821 | 50 | 28 |

All **2,503 copied file hashes** pass independent comparison, and all three
collections include the exact readable listing. These discovery counts include
the compressed acknowledgement file; they are not counts of readable licenses.
Each inventory still records `license_review_complete=false`. The supplemental
manifest has 53 exact package attributions.

Collection commands follow the [dependency checkpoint](dependency-notices-och17.md),
using fresh outputs `native-assets-macos-001`, `signal-assets-macos-001` and
`gallery-assets-macos-001` under the ignored
`scratch/agents/root-20261003-release-notices/`. That directory also holds
`two-face-notices-cargo-001.log`, `assets-notice-compare-001.log`,
`assets-notice-tests-001.log` and `assets-notices-hashes-001.log`. Scratch files are
evidence only; the checked-in example and attribution manifest are reproducible
inputs. No application runtime behavior or dependency selection changed.

## Asset paths reviewed

| Input | Actual use and release-review consequence |
| --- | --- |
| Two-face syntax dump | Embedded in the native highlighter; readable upstream acknowledgement subset now covered by the explicitly attributed full listing. Review remains necessary for applicability and completeness. |
| Syntect `assets/default.themedump` | `ThemeSet::load_defaults` embeds a set of seven themes. GPUIO chooses `base16-ocean.dark` and `InspiredGitHub`, but choosing two at runtime does not remove the other five from the dump. The crate's MIT code license alone is not evidence establishing each theme's attribution. The subsequent [theme-source checkpoint](syntect-themes-och17.md) traces all seven inputs and collects three exact root licenses; derivative attribution/final review remains. Do not reuse two-face theme notices merely because names resemble each other. |
| Agent Workspace icons and portrait | SVG paths are literals in `runtime/icons.ml` and `runtime/contributor_portrait.ml`; the portrait is a geometric glyph, not a bundled photograph. Review source authorship/attribution with the example code. Source literals do not by themselves prove independent authorship. |
| Gallery images and icons | SVG literals in asset/avatar/badge/link/spinner/tab examples and a PNM gradient generated by `image_samples.ml`. The invalid-image demonstration and `example.invalid` URL are failure fixtures, not downloaded image assets. |
| Fonts | `font_defaults.rs` enumerates installed native families and chooses an available default. The reviewed reference apps do not register a bundled font. `asset_svg_test.rs` embeds the Bonsai example font only in the test module; it is not a reference-app font input. Source/test distributions still need their own applicable notices. |
| Native shaders | `gpui_platform` is built with `runtime_shaders`; the Apple renderer embeds stitched Metal source produced from the maintained GPUI shader/header sources. They are generated code, not a separately downloaded art pack. Retain the corresponding GPUI source notices and review generated/toolchain/system inputs; a dynamic-library load audit does not close this review. |
| Upstream example assets | GPUI's example SVG/PNG/JPEG files and test-only embedded images are present in the source tree. Presence in a vendored repository is not proof that a reference executable embeds them. Keep source-distribution obligations separate from application-binary inputs. |

This is a reviewed set of direct asset paths, not an automatic guarantee that all
transitive data tables, fonts, generated bindings or system SDK inputs have been
accounted for. The [package notice inventory](dependency-notices-och17.md) and
[OCaml/runtime inventory](ocaml-notices-och17.md) remain required. The 28 Rust
missing-text packages, ten OCaml inventory gaps and final derivative-asset/system
review remain open. Syntect theme source/root-notice provenance now has the
[separate checkpoint](syntect-themes-och17.md). None of the local qualification bundles is a
licensing-approved distribution artifact.
