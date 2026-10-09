# OCaml notice source provenance — OCH-17

Read-only macOS arm64 check, 2026-10-08, based on
`12f39fb9` and the isolated `.opam-root` / `gpuio` switch. All **499 collected
notice files** now have verified original-archive byte provenance. This closes
that specific gap in the [OCaml inventory](ocaml-notices-och17.md); it does not
approve a release notice bundle or prove all embedded attributions were discovered.

## Verification

The existing collector produced a fresh inventory of 136 installed packages and
eight vendored roots. The audit checked every copied manifest and notice against
its recorded hash and current local input. For primary source archives it then:

1. Read checksums from the exact installed manifest, or the pinned Bonsai-family
   source manifest for vendored roots.
2. Located the original archive in the existing checksum-addressed opam cache or
   previously verified Bonsai archive directory. No downloads were needed.
3. Recomputed every declared checksum, requiring a SHA-256 or SHA-512 identity.
4. Read archive members without extracting or following symlinks. Cached-source
   and vendored notices had to match the same relative archive path. Installed
   documentation could match renamed/flattened source files by exact bytes; every
   matching original path remains in the report.

| Coverage | Result |
| --- | ---: |
| Installed packages / vendored roots | 136 / 8 |
| Package records with verified primary archives | 133 |
| Distinct verified archives | 115 |
| Notice files matched to their package's primary archive | 498 |
| Virtual `ocaml` notice explicitly matched to compiler archive | 1 |
| Notice files with unresolved byte provenance | 0 |
| Package records with no collected text | 10, unchanged |

The `ocaml 5.3.0` package is virtual and has no primary source archive. Its installed
`LICENSE` matches `ocaml-compiler 5.3.0`'s archive `LICENSE` exactly. The audit
records this explicit provider, the archive hash
`22c1dd9de21bf43b62d1909041fb5fad648905227bf69550a6a6bef31e654f38`
and license hash
`65f47d3bf5a921011ac7fa30f23d54f9dffbe7683921eeb116d5ce8dc26ff0c1`.
It does not apply an inferred cross-package exemption to other packages.

The ten zero-text rows still match the version/manifest identities in
[their classification record](../../third_party/ocaml-package-review.json).
This follow-up revalidates those identities, not the earlier install-receipt or
configuration-template payload comparisons. The collector and provenance report
retain `license_review_complete: false`.

## Commands and evidence

```sh
python3 scripts/collect_ocaml_notices.py --opam-root .opam-root --switch gpuio \
  --vendor vendor/bonsai --vendor vendor/virtual_dom --vendor vendor/incr_dom \
  --vendor vendor/incremental --vendor vendor/incr_map --vendor vendor/incr_select \
  --vendor vendor/abstract_algebra --vendor vendor/ppx_pattern_bind \
  --output scratch/agents/root-20261007-access-check/ocaml-notice-provenance/inventory
python3 scratch/agents/root-20261007-access-check/ocaml-notice-provenance/verify.py
python3 scripts/test_collect_ocaml_notices.py
```

Collection, provenance verification and all five portable collector regressions
pass. The archived verifier intentionally accepts the simple top-level source
URL/checksum blocks in these exact manifests; it is an audit artifact, not a
replacement general-purpose opam parser. Revalidate assumptions for new inputs.

The [archive](ocaml-notice-provenance-och17/reports.tar.gz) and
[manifest](ocaml-notice-provenance-och17/manifest.json) preserve the full fresh
inventory, copied texts and package metadata, verifier, per-file source mappings,
first/final results and test log. Archives themselves remain in local caches;
the report records source URLs and checksums for independent retrieval.

No source, package, switch selection, default, license alternative or application
behavior changed. This does not attest compiled binary contents or whole source
builds. Remaining review includes source headers and embedded assets beyond
filename-based discovery, configuration/helper attribution, native/system inputs,
the Rust notice gaps, each shipped consumer's distribution inputs and preparation
of the final notice bundle. This is macOS switch evidence, not a Linux inventory
or GUI/signing/distribution qualification. OCH-17 remains open.
