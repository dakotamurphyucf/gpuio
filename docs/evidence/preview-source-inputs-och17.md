# Preview source payload review — OCH-17

2026-10-08, base `1f360479` with documentation/notice edits in progress. D1 is
being reviewed against the actual **source-library preview**, not the internally
built reference-app bundles. The assembled source-archive review below closes D1 for this source preview.

## Payload inspection

Inspected tracked paths and streamed all 256 tracked tar/gzip archives, including
11 nested source archives, without extracting them over the working tree. The
inventory covers 19,642 file/container/member observations. This count includes
both archive containers and their members; it is not a count of unique sources.
Twenty standalone compressed logs also received decompressed-prefix inspection.

No inspected file had an ELF, Mach-O, PE or ar archive signature, or a compiled
object/library/executable extension. No archive links or special entries were
found. The report records all member paths and sizes, archive hashes and flagged
asset/archive paths. These checks identify common native binary payloads; they
are not a generic proof against executable code disguised as text or an audit of
every possible file format. The retained files are sources, data, screenshots,
logs and notice/provenance evidence, rather than the locally assembled `.app`
bundles or downloaded Cargo/opam caches.

The 26 unresolved Rust missing-text classifications are retained in the
[earlier review](rust-notice-gaps-och17.md). Matching paths in this source payload
are retained Objective-C notice files, not those packages' implementations.
Downloaded Cargo dependencies become part of applications built by consumers;
that binary distribution requires its own complete notice review. The existing
499-text OCaml provenance result also remains useful for that work. Neither the
source-payload distinction nor these inventories approves an incomplete binary
notice bundle or removes the original findings.

The [four-file evidence archive](preview-source-inputs-och17/reports.tar.gz)
includes the inspection script, member inventory, summary and font provenance.
All entries were read back and checked against the
[checksum manifest](preview-source-inputs-och17/manifest.json). The scan predates
the new untracked installation/source-review evidence; final artifact assembly
must include and review those additions as well.

## Concrete notice repair

The only tracked font is `vendor/bonsai/examples/font_hosting/font.ttf`. Its
metadata identifies Fira Code Regular 6.002 under SIL OFL 1.1, but no complete
font license text accompanied that retained browser example. Its bytes exactly
match the original 6.2 release's `ttf/FiraCode-Regular.ttf`. Added the unmodified
upstream license and explicit attribution in the
[asset notices](../../third_party/ASSET_NOTICES.md). Those notices record source
URLs, font/archive/license hashes, and the distinction between the unbuilt
browser example and native application fonts. No font bytes changed.

All 15 tracked vendor roots retain their upstream license files. Existing
Feather, Dygraph, Lodash and virtual-DOM nested notices remain present; the asset
index now points out the browser-binding notices as well. The earlier Dragon
SVG attribution and source equivalence remain recorded. This closes the specific
font omission, without certifying a future binary release or claiming that
GPUIO's Apache license replaces third-party asset terms.

## Remaining D1 step

Review the exact assembled source artifact and its notice/index contents at the
candidate revision. Carry the binary/system/SDK/embedded dependency review into
the separately delivered application artifacts in OCH-164. Do not repeat the
completed dependency-source archaeology unless the chosen artifact adds those
implementations or a new source discrepancy appears.

## Assembled source review — b90fb769

Created `gpuio-b90fb769.tar.gz` with:

```sh
git archive --format=tar.gz --prefix=gpuio-preview/ \
  -o scratch/agents/root-20261007-access-check/preview-source/gpuio-b90fb769.tar.gz \
  b90fb769
```

The artifact is 427,400,224 bytes, SHA-256
`c64ebbfd5681d77b75d0bd1d645bbf02566a3895ec2a6ed17020d63bdbee844f`.
A streaming verifier independently matched **all 8,168 regular files** to their
Git blob identities and executable modes at
`b90fb769926a8467b8d673dbc1aa56417c343b7b`; no unexpected, duplicate or missing
paths, links or special entries were accepted. Local build outputs, scratch,
Git metadata and opam/Cargo caches are absent. Required project/vendor/asset
notice indexes, the new full font notice, source pins and preview guides are
present. The artifact is retained locally and has **not been published**.

A second payload scan includes the installation/source-review additions: 258
tracked archives plus 11 nested archives, 19,659 file/container/member
observations, and no native binary signature findings. The
[five-file assembled-review archive](preview-source-inputs-och17/assembled-review.tar.gz)
and [manifest](preview-source-inputs-och17/assembled-manifest.json) preserve the
verifier, exact result, scan and member inventory; all archived entries were read
back and checksum-verified. The 427 MB source artifact itself is not recursively
committed into the repository.

D1 is closed for the **source-library preview**: its shipped vendor sources and
assets retain their upstream notices, the concrete font omission is repaired,
and dependencies obtained by the user's compiler are distinguished from shipped
implementations. The unresolved Rust binary-notice classifications, compiler/
system/SDK and embedded runtime-asset completion remain required when preparing
OCH-164 application binaries. This is not approval of those internal app bundles.

R2 must bind the published tag and source payload to its final revision. The only
subsequent additions here are this review record and its text/JSON/script evidence;
any later change to shipped code, assets, dependencies or distribution format
requires a corresponding input/notice impact review. The source preview provides
a Git clone/tag workflow; it does not require redistributing locally compiled
native archives or downloaded build caches.
