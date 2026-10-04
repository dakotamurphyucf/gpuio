# OCaml notice inventory — OCH-17

Local macOS arm64 audit input, 2026-10-04, on `83eb87e` plus the working tree.
This closes the absence of an OCaml-side collection mechanism. It does not approve
licenses, establish exact linked content or qualify a distributable application.

## Inventory contract

`scripts/collect_ocaml_notices.py` requires an explicit opam root and switch. It
queries that switch's prefix and installed name/version list, then reads the exact
installed opam manifests, installed documentation and available cached source trees.
License fields are normalized by opam and retained without choosing alternatives.
The collector performs only `var`, `list`, `show --just-file` and version queries;
it never installs/updates packages, selects a switch or changes defaults.

All installed packages are included, including developer tools and build dependencies.
Explicit vendor roots supplement them because the native Bonsai fork is built from
repository sources rather than installed as ordinary opam packages. Project/opam
manifests and `third_party/sources.json` are retained with hashes. Package metadata,
original notice bytes and every source-to-output mapping retain separate identities.
Local source/doc trees are audit inputs, not verified unmodified upstream archives.

Discovery includes named license/notice/copyright/author/credit/acknowledgement files
and contents of such directories. Symlinks are not followed. Missing or empty texts
remain visible, and an existing output is never overwritten. A second installed-list
query rejects package/version drift before output creation. The report always records
`license_review_complete: false`; filename discovery cannot prove all embedded-source
or asset obligations have been collected.

## Actual collection

```sh
python3 scripts/test_collect_ocaml_notices.py
python3 scripts/collect_ocaml_notices.py --opam-root .opam-root --switch gpuio \
  --vendor vendor/bonsai --vendor vendor/virtual_dom --vendor vendor/incr_dom \
  --vendor vendor/incremental --vendor vendor/incr_map --vendor vendor/incr_select \
  --vendor vendor/abstract_algebra --vendor vendor/ppx_pattern_bind \
  --output scratch/agents/root-20261003-release-notices/ocaml-macos-002
```

The collection contains **136 installed packages and eight vendored roots**, with
**499 copied notice texts**. All notice, package metadata, project input and installed
list hashes were independently recomputed and match the report. Five portable tests
pass, covering exact bytes/metadata drift, nested notice directories, unsafe/symlink/
missing inputs, output collision prevention, explicit-switch-only commands and a
simulated installed-version change. The tests are added to both foundation jobs;
this checkpoint does not claim hosted execution.

The OCaml 5.3.0 installed `doc/ocaml/LICENSE` and cached `ocaml-compiler.5.3.0/LICENSE`
match byte-for-byte: SHA-256
`65f47d3bf5a921011ac7fa30f23d54f9dffbe7683921eeb116d5ce8dc26ff0c1`.
Both include the original LGPL text and OCaml linking exception. They are retained
unchanged; this evidence makes no interpretation of redistribution obligations.
The compiler manual's separate license is also collected, conservatively.

## Explicit remaining gaps

Ten package rows have no independently collected text:

- `base-bigarray`, `base-domains`, `base-effects`, `base-nnp`, `base-threads`,
  `base-unix` and `seq`.
- `ocaml-base-compiler`, `ocaml-config` and `ocaml-options-vanilla`.

These ten rows now have an explicit, hash-bound
[classification record](../../third_party/ocaml-package-review.json), described
below. The discovery count remains ten: the collector does not silently call
missing text an exemption or merge rows into another package.

## Classification and compiler-source verification — 2026-10-04

All ten current installed manifests still match their recorded inventory hashes.
The classification uses the actual manifests, install receipts and payload bytes,
not assumptions based on package names:

| Package rows | Classification and evidence |
| --- | --- |
| Six `base-*` rows above | Compiler capability/library markers. No source, build or install instructions and no individual install receipts. Bigarray/Threads/Unix describe compiler-distributed libraries; Domains/Effects/NNP describe compiler capabilities. This does not remove the compiler/runtime's licensing obligations. |
| `ocaml-base-compiler 5.3.0` | Compiler selector with an exact `ocaml-compiler = 5.3.0` dependency. No independent payload/install receipt. The implementation's license is already collected. |
| `ocaml-options-vanilla 1` | Compiler option/dependency constraints, with no independent payload/install receipt. Its declared `CC0-1.0+` metadata remains recorded. Redistribution of package metadata is a separate review from application code. |
| `seq base` | Compatibility metadata: the install receipt contains only `lib/seq/META`. That file declares no requirements or archive and matches the pinned `META.seq` extra-source SHA-256. The actual `Seq` implementation is part of the compiler distribution. |
| `ocaml-config 3` | Build configuration helper: the install receipt contains only `share/ocaml-config/gen_ocaml_config.ml`. The `ocaml` virtual package runs it during configuration to generate switch variables. It is not an application link library. Its declared ISC license is retained; a separate full notice for redistributing the helper itself remains unestablished. |

All four extra-source cache files for `seq` and `ocaml-config` match the hashes in
their installed manifests. The installed `seq` metadata is byte-identical to its
source. The installed configuration helper matches its pinned template after the
recorded vanilla-switch substitutions and opam's `%%` unescaping. The initial
comparison omitted that unescaping; inspection isolated literal percent signs,
and the corrected comparison changes no installed file or expected source bytes.

The original OCaml 5.3.0 compiler archive is still available in the local download
cache and matches the installed manifest's SHA-256:
`22c1dd9de21bf43b62d1909041fb5fad648905227bf69550a6a6bef31e654f38`.
Its `LICENSE` is byte-identical to the installed notice already collected, with
SHA-256 `65f47d3bf5a921011ac7fa30f23d54f9dffbe7683921eeb116d5ce8dc26ff0c1`.
The archive's `stdlib/{seq,bigarray,domain,effect}.ml` files also match installed
source bytes. This strengthens the license's exact-source provenance; it does not
attest every compiler build output or interpret the linking exception.

The classification JSON retains each manifest hash, the two install-receipt
hashes, compiler/archive/license identities, and the template/installed-helper
hashes. It is a manual review supplement, **not** an automatic collector exemption
list. Revalidate it for another switch or changed inputs. `license_review_complete`
remains false in both that record and the original inventories.

Local evidence is `ocaml-metadata-review-002.log` and `verify_ocaml_metadata.py`
under ignored `scratch/agents/root-20261003-release-notices/`. Verification reads
local metadata/cache/archive bytes only: no switch, package, generated helper or
source tree was changed. Scratch paths are not build dependencies.

Review the entire inventory, including all rows with text. Complete exact-source,
generated configuration and asset attribution, native/system dependencies and each
shipped consumer's actual distribution inputs. Repeat the inventory on Linux for
its distinct installed graph. Prepare reviewed notice files for packaging rather
than shipping this unreviewed audit directory wholesale. Clean-machine execution,
signing/distribution, macOS native/performance and the remaining OCH-41/OCH-17 gates
are still open. No native GUI or platform acceptance was run for this tooling work.
