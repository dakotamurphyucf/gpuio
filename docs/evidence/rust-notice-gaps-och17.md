# Remaining Rust notice gaps — OCH-17

2026-10-04 follow-up to the [notice inventory](dependency-notices-och17.md).
The original 28 zero-text package rows have a hash-bound
[review record](../../third_party/rust-notice-gaps.json). This distinguishes source
identity from notice completeness. It does not remove a discovery gap, establish
an exemption or approve redistribution. No package or application behavior changed.
After network access was restored, the exact Boxroot notice was collected;
**27 packages lacked collected text** at that checkpoint. The later source-equivalent
Pathfinder collection below reduces the missing-text count to **26**, confirmed
by fresh native, Signal and gallery inventories.

## Eighteen Objective-C family packages

The exact four revisions referenced by these published crates contain the same
root `LICENSE.md`, Git blob `540ae4c81adb5f64a9f61e222b688880da24300e`:

- `7b1abfd750a2cacaea71d6a56ecfb83cb7de560b`
- `b4167b582b2f75f9a1be75495c41b765344fd03c`
- `8852b424193ca41602281b3d7540d7c8ed51e49a`
- `8d214f5477365ffcbcbb7de058c86ed9a518efb7`

The [policy](https://github.com/madsmtm/objc2/blob/7b1abfd750a2cacaea71d6a56ecfb83cb7de560b/LICENSE.md)
identifies MIT-only and alternative-license groups, links to standard license
texts, and explicitly discusses an unresolved question about derivation from
Apple SDKs. It is a policy document, not a complete set of attributable notice
texts. All four returned file contents match the recorded blob hash. No old
repository license or generic copyright attribution was substituted, and no
license alternative has been selected by this audit.

## Six source trees inspected

Each published crate archive matches Cargo.lock's checksum. Original manifests
and source bytes were compared to upstream Git blob identities; normalized Cargo
metadata, generated lockfiles and VCS-info records are outside that source
comparison. The original release tags resolve missing VCS metadata for some older
packages without assuming that the current default branch represents the release.

| Package | Verified revision | Result |
| --- | --- | --- |
| `block 0.1.6` | [release tag](https://github.com/SSheldon/rust-block/tree/47178790cfc9d4a8b092051d8b413b78bd31254a) | All four packaged source/original-manifest files match. No notice file in the full tree. |
| `mac 0.1.1` | [release tag](https://github.com/reem/rust-mac/tree/66afc663b68a65633ea165c742b5a9c6734581c2) | All twelve packaged files match. README has a license declaration, but no complete license text/file was found. |
| `malloc_buf 0.0.6` | [release tag](https://github.com/SSheldon/malloc_buf/tree/a7811e5f4c6f9685dd57ad9bfe34e9bf6b0ba4f9) | Implementation matches. Published manifest additionally excludes `.gitignore`; do not claim exact whole-package/tag equality. No notice file in the tree. |
| `objc_exception 0.1.2` | [published VCS revision](https://github.com/SSheldon/rust-objc-exception/tree/c86ad3a52984461fc5c63980d12e8ceed847854c) | All four source/original-manifest files match, including the Objective-C source. No notice file in the tree. |
| `simd_helpers 0.1.0` | [published VCS revision](https://github.com/lu-zero/simd_helpers/tree/ca1a2f84aa386d758e98f8a609d990263932fb85) | All five source/original-manifest files match. README/source do not supply complete license text. |
| `seahash 4.1.0` | [published VCS revision](https://gitlab.redox-os.org/redox-os/seahash/-/tree/94b632aeac099031c373599313d5b5f0acbbaec0) | Restored access permits complete recursive-tree inspection: 17 entries, 14 blobs, no additional page. All fourteen packaged original source/asset/manifest files match. No notice file or complete terms in that tree. |

This is evidence that these source questions are now narrower than “unknown
release revision.” It is not permission to manufacture copyright statements from
Cargo author names or silently count a manifest's license identifier as full text.

## Exact Boxroot notice recovered

The project [LICENSE](https://gitlab.com/ocaml-rust/ocaml-boxroot/-/blob/45d6313d4065e8ff65e75c6fad038c8da0c7b3eb/LICENSE)
is now available at the revision recorded by `ocaml-boxroot-sys 0.4.0`'s
`.cargo_vcs_info.json` (`rust/ocaml-boxroot-sys`). The GitLab file API confirms
the exact commit, Git blob `d474b9e07c8aadc68a008c0d815cea0dcec0e469` and SHA-256
`ddfc2cb4681112f197d193c7d95352f1122da17710c7d2bcd429c0d29842840a`.
The separately fetched raw bytes match both hashes. The original MIT notice,
including its named authors, is copied unchanged and explicitly attributed in
`third_party/notice-sources.json`; no generic text or invented attribution was used.

Boxroot and SeaHash published archives match their Cargo.lock checksums,
respectively `0f3c2664a427c8046d334bf3a50fc466170a3dc53c65bc926a9be31a8e8debd1`
and `1c107b6f4780854c8b126e228ea8869f4d7b71260f962fefb57b996b8959ba6b`.
Their archive VCS records match the installed records. The Boxroot notice gap is
resolved; SeaHash's source retrieval is resolved but its notice gap remains.

## Earlier three retrieval/source questions (superseded below)

- `leak 0.1.2`: declared repository redirects from `jmesmon/leak` to
  `codyps/leak`; published archive has no VCS revision. Exact release source and
  attributable notice text remain unresolved.
- `leaky-cow 0.1.1`: published archive has no VCS revision; the declared
  repository's tag-ref request returned 404. No exact release source was inferred.
- `pathfinder_geometry 0.5.1`: retain the earlier exact-revision 404 evidence;
  its recorded 40-character VCS identity was rechecked locally. A different
  Pathfinder crate's license is not automatically this package's attribution.

## Evidence and next action

All 28 live installed Cargo manifest hashes match the classification record.
Raw tree responses, source/archive comparisons and policy responses are retained
in ignored `scratch/agents/root-20261003-release-notices/` as
`rust-missing-notices-{trees,source-comparison}-001.json` and
`objc2-policy-fetches-001.json`. These are evidence, not build inputs.

The supplemental notice manifest now has 55 entries. Fresh native, Signal and
gallery-backend collections contain 866, 824 and 825 notice files respectively;
all **2,515** hashes match, with 27 missing-text packages per graph. Collector
tests pass (ten cases). Exact-source/API/archive comparisons and fresh inventories
are retained under `scratch/agents/root-20261004-resumed/`. No incomplete policy
or identifier was counted as full notice text. Resolve the listed source questions and
applicable attributable texts, then complete review of the entire distribution,
including already collected files and system/SDK inputs. This record must not be
used as an automatic exclusion list or a completed release gate.


## Historical source equivalence follow-up — 2026-10-04

The two oldest archives do not record a VCS identity. Comparing every packaged
file against all 35 available historical commit trees identifies byte-equivalent
snapshots, without assuming a default branch or inventing a publishing commit:

| Package | Matching source snapshot | Packaged files verified | Whole tree |
| --- | --- | ---: | ---: |
| `leak 0.1.2` | [024bd1ccfa33](https://github.com/codyps/leak/tree/024bd1ccfa33aea1eff0026b0d8b676874b5e3d2) | 2 | 16 entries, not truncated |
| `leaky-cow 0.1.1` | [88f4a8cd01ff](https://github.com/notriddle/rust-leaky-cow/tree/88f4a8cd01ffc634d9f7642b97eb764a8dcd3760) | 5 | 6 entries, not truncated |

Fresh published archive downloads match Cargo.lock checksums and every regular
archive file matches the upstream Git blob at the linked snapshot. Both trees
lack complete notice files; `leaky-cow`'s source says it uses Rust's terms, which
is preserved as a declaration rather than fabricated copyright text. Their source
questions are narrowed to equivalent verified snapshots; missing notices remain.

For `pathfinder_geometry 0.5.1`, the recorded VCS identity still returns no matching
GitHub commit. However, all ten original manifest/source files in its
checksum-verified published archive match the `geometry` subtree at
[1cd5966d3d7c](https://github.com/servo/pathfinder/tree/1cd5966d3d7c3d395b6886d1ed63eb7b306f9be3/geometry).
Generated normalized Cargo metadata and `.cargo_vcs_info.json` are excluded from
this source equivalence comparison. The linked snapshot's complete
[MIT](https://github.com/servo/pathfinder/blob/1cd5966d3d7c3d395b6886d1ed63eb7b306f9be3/LICENSE-MIT)
and [Apache](https://github.com/servo/pathfinder/blob/1cd5966d3d7c3d395b6886d1ed63eb7b306f9be3/LICENSE-APACHE)
notices are retained unchanged, with both file SHA-256 and Git blob verification.
The supplemental record explicitly distinguishes this equivalent snapshot from
the unresolved recorded publishing revision; it does not silently substitute
another crate's notices or select a license alternative.

The review JSON records archive checksums, verified paths and this distinction.
Raw commit/tree/source/archive comparisons are retained under
`scratch/agents/root-20261004-resumed/` as `remaining-source-*`,
`pathfinder-source-*`, the downloaded archives and corresponding tree responses.
The ten existing collector tests, including retained notice/blob identity checks,
pass after adding these bytes. This resolves source retrieval/equivalence work,
not full distribution approval: eighteen policy-only and eight no-notice source
packages still need notice/completeness review.

Fresh locked/offline native, Signal and gallery collections confirm **26** missing-text
packages each. They contain respectively 513/485/486 packages and 868/826/827 copied
texts; all **2,521** copied hashes match. The supplemental manifest now has 56
entries. Outputs are `native-pathfinder-macos-001`, `signal-pathfinder-macos-001`
and `gallery-pathfinder-macos-001` in the session scratch directory; the verifier
summary is `pathfinder-inventories-verified-001.json`. No dependency version,
license declaration, compiler switch or native code changed.
