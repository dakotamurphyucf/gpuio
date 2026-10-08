# Preview source payload review — OCH-17

2026-10-08, base `1f360479` with documentation/notice edits in progress. D1 is
being reviewed against the actual **source-library preview**, not the internally
built reference-app bundles. Final assembled source-archive review remains open.

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
