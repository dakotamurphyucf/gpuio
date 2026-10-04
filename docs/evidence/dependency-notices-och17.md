# OCH-17 dependency notice collection

Local macOS arm64 checkpoint, 2026-10-03, working tree based on `83eb87e` with
uncommitted milestone changes. This is a release audit input, not licensing
approval or completion of the distribution gate.

The collector runs offline locked Cargo metadata for a named target and walks
normal/build dependency kinds from explicitly named roots. It preserves distinct
Cargo identities even when names/versions coincide, raw license expressions,
manifest hashes, original notice bytes and their hashes. It retains metadata and
the lockfile. It neither rewrites dependency declarations nor chooses license
alternatives. Symlinks/outside-package declarations and missing/empty texts remain
review issues. Collection into an existing directory is rejected.

## Local evidence

```sh
python3 scripts/test_collect_rust_notices.py
GPUIO_JOBS=2 ./scripts/gpuio exec python3 scripts/collect_rust_notices.py \
  --target aarch64-apple-darwin --root gpuio-native \
  --output scratch/agents/root-20261003-release-notices/native-macos-001
```

Five tests pass: cyclic/transitive build graphs and mixed dependency kinds;
ambiguous roots/incomplete graphs/unknown kinds; distinct same-name packages,
byte/hash preservation and nested asset acknowledgements; missing/outside/symlink
texts; explicit nonstandard filenames and empty notices. These are non-GUI,
portable Python tests, also wired into both foundation CI jobs. Hosted execution
is pending.

The local collection contains **512 packages and 800 text files**, with **75
packages flagged for collection review**. Examples include upstream Zed crates
whose Apache license paths are symlinks to their repository root; nested derive
crates with workspace-level notices; first-party crates covered by GPUIO's root
license; and registry archives without separate license files. A package declaring
MIT or Apache in metadata does not manufacture its missing copyright/license text.
These flags require source-specific resolution, not a blanket license inference.

The requested target is recorded accurately, but the result is conservative:
Cargo's macOS-filtered metadata retains Linux normal dependency kinds for
`gpuio-portal` and `gpuio-wayland` on edges also admitted as dev dependencies.
Those mixed edges are retained. Build tools, proc macros, workspace-unified
features and embedded source notices can also exceed actual linked content.

The separate Signal Studio backend was collected with:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec python3 scripts/collect_rust_notices.py \
  --manifest-path examples/signal_studio/backend/Cargo.toml \
  --target aarch64-apple-darwin --root gpuio-signal-backend \
  --output scratch/agents/root-20261003-release-notices/signal-macos-001
```

It yields **484 packages, 758 text files and 75 flagged packages**. Its closure
includes the example extension and composed backend, and resolves a different
`crc32fast` package identity. Unlike the root workspace graph, this independent
graph omits the Linux portal/Wayland dependencies and their transitive packages.
This confirms why a root-workspace inventory cannot stand in for every shipped
consumer. All 1,558 copied text hashes across both outputs were independently
recomputed and match their inventory entries. The six existing portable packaging
tests and `git diff --check` also pass; no runtime changes required native or GUI
tests for this tooling slice.

## Supplemental source checkpoint

A subsequent exact-source audit records 27 package attributions in
`third_party/notice-sources.json`: eighteen Zed crates with package-local Apache
symlinks, two nested FFI derive crates and seven first-party crates across the
root/Signal graphs. All Zed symlink targets were read from the pinned checkout and
compared byte-for-byte with the existing retained Apache text. The two derive
workspace licenses are now copied unchanged into `third_party/licenses`; no
license expression or dual-license choice is rewritten.

Repeating both commands above with `--supplemental third_party/notice-sources.json`
and fresh `native-macos-002`/`signal-macos-002` output directories passes. Each
graph uses 25 attributions and records the other two as unused. The native graph
now has 825 text copies and Signal has 783, with **50 packages without collected
text in each graph**. All 1,608 text hashes and both supplemental manifest hashes
were independently recomputed and match. The original 75 discovery-issue rows
remain visible; supplementing bytes is not license approval.

Nine portable tests pass, including four additional cases for exact supplemental
bytes, manifest/license drift rejection before output creation, source identity
changes remaining unresolved, and text hash/path/symlink/duplicate admission.
Inventory schema 2 distinguishes original discovery issues, supplemental texts,
and remaining packages with no collected text. A source/manifest pin change
requires revisiting the corresponding attribution, not removing validation.

## Registry source checkpoint

The published crate `.cargo_vcs_info.json` files identify exact upstream revisions
for further collection. Read-only GitHub contents/file requests retrieved those
revisions; decoded bytes were verified against Git blob identities before copying.
Thirteen distinct complete license/notice files are retained under
`third_party/licenses/registry/`, adding 21 package attributions. This includes
AccessKit's additional Chromium notice and both texts for dual-license sources.
No current/default-branch license was substituted for a historical revision.

The manifest now contains 48 package attributions. Repeating both collection
commands with the supplemental manifest and fresh `native-macos-003` and
`signal-macos-003` directories passes: the native inventory has **858 text copies**
and Signal has **816**, with **46 supplemental package attributions** and **29
packages without collected text** in each graph. All 1,674 copied text hashes and
both manifest snapshots were independently checked. Ten portable tests pass;
the added repository-data test checks retained text SHA-256 and upstream Git blob
identities, so accidental edits to notices fail the existing CI test step.
`git diff --check` passes.

The remaining gaps are substantive source questions:

- Eighteen newer objc2-family packages point at a
  [licensing policy](https://github.com/madsmtm/objc2/blob/7b1abfd750a2cacaea71d6a56ecfb83cb7de560b/LICENSE.md)
  that links to terms and discusses generated SDK bindings. It is not a complete
  replacement for the full notices. The policy was inspected but not counted as
  satisfying missing-text collection; older MIT text is not silently reused.
- The `pathfinder_geometry` archive records commit
  `a5e98fac00f433fe2ff4b2135383d82491b8bfc5`, but GitHub's exact-revision request to
  its declared `servo/pathfinder` repository returned 404. Another revision's
  text is not attributed to that package.
- `objc_exception` and `simd_helpers` have no license file at the inspected
  repository roots; nested/source documentation still needs review.
- Five older crates lack recorded VCS revisions; two point at GitLab hosts;
  `zune-inflate` lacks a repository declaration. Resolve their exact source and
  attribution explicitly rather than treating missing metadata as permission to
  copy a convenient contemporary license.

Fetch responses and byte-verification inputs are retained in the local ticket
notepad directory as `upstream-notice-fetches.json`. Durable retained texts,
source URLs, hashes and package selectors live in the versioned attribution
manifest. This remains notice collection, not final legal/release qualification.

## Current consumer graphs and exact-source follow-up — 2026-10-04

The document SDK and public profile package changed the dependency graphs after
the preceding checkpoint. The native manifest attribution was explicitly reviewed
and refreshed, and root-license mappings now include the document SDK, public
profile example and composed gallery backend. The supplemental manifest contains
**52 package attributions**; unchanged third-party pins were not rewritten.

The `zune-inflate` package's recorded revision
`69502ce83fdfecdd0beefd677e2abb3781b29d98` resolves in
[etemesi254/zune-image](https://github.com/etemesi254/zune-image/tree/69502ce83fdfecdd0beefd677e2abb3781b29d98).
Its packaged `Cargo.toml.orig` is byte-identical to that revision's
[zune-inflate manifest](https://github.com/etemesi254/zune-image/blob/69502ce83fdfecdd0beefd677e2abb3781b29d98/zune-inflate/Cargo.toml),
Git blob `882d8114f2611f0a631ec6fbb4c791f6457645c1`. This establishes provenance
despite the absent repository field. The exact root copyright/licensing policy and
full Zlib license were copied unchanged and their Git blob hashes verified. Full
MIT/Apache texts were absent from that root; the collection does not choose an
alternative or resolve final completeness review.

Fresh offline locked collections with `--supplemental` now produce:

| Actual root/consumer | Packages | Copied texts | Supplemental packages | No collected text |
| --- | ---: | ---: | ---: | ---: |
| `gpuio-native` | 513 | 861 | 48 | 28 |
| Signal `gpuio-signal-backend` | 485 | 819 | 48 | 28 |
| Gallery `gpuio-counter-backend` | 486 | 820 | 49 | 28 |

The gallery command uses
`--manifest-path examples/extension_consumer/backend/Cargo.toml --root gpuio-counter-backend`;
the native/Signal command shapes are unchanged. Exact outputs are retained locally
as `native-macos-004`, `signal-macos-004` and `gallery-macos-001` in the ticket
notepad directory. All **2,500 copied text hashes** were independently verified.
Ten collector tests pass. No dependency version or switch changed.

The remaining 28 missing-text packages are the previous gaps except zune-inflate;
this is not a count of unlicensed dependencies. Every package still needs final
applicability and completeness review. The OCaml/compiler/runtime side now has a
separate [144-package inventory](ocaml-notices-och17.md), rather than remaining
entirely uncollected.

## Remaining acceptance

The [remaining-gap review](rust-notice-gaps-och17.md) classifies all 28 zero-text
rows: eighteen share a verified upstream policy without complete notice texts,
five have inspected source trees without notice files, and five retain specific
source/retrieval questions. Release tags and published source bytes now establish
several older crates' identities. The count is unchanged; this is review evidence,
not an exemption list or permission to substitute invented attribution.

A subsequent [embedded-asset checkpoint](embedded-assets-och17.md) adds readable
two-face notices to all three inventories (53 supplemental package attributions
in the manifest). All 2,503 copied file hashes pass; the 28 missing-text packages
remain. It separately identifies the full Syntect default-theme dump as requiring
attribution review; runtime selection of two themes does not reduce its embedded
contents. That set now has [exact theme-source/root-license evidence](syntect-themes-och17.md): 54 supplemental manifest entries and 2,512 verified copied file hashes across the three refreshed inventories. Final derivative attribution review remains.

Every collected package still needs review, including those without discovery
gaps. Confirm copyright/NOTICE obligations, asset and generated-source coverage;
resolve upstream/workspace files against exact source revisions. The collector
does not parse license text for legal equivalence or claim full SPDX validation.
Complete review of the OCaml/runtime inventory and collect native/system inputs; repeat for independent
consumer lockfiles and the shipped targets/features, then prepare reviewed bundle
notices. Existing internal bundles remain unqualified for distribution. No new
GUI, clean-machine, Linux, signing or publication acceptance is claimed.
