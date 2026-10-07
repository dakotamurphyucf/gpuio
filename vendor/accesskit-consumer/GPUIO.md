# GPUIO accessible text scopes

This is the published `accesskit_consumer` 0.38.0 at AccessKit revision
`c88605b96d04431f9c3c792464a0f2f253480e94`, with one scoped patch. AccessKit protocol
and platform-adapter versions are unchanged. `UPSTREAM.json` pins the registry
archive, every original source file, and all three upstream root notice files.
License texts were copied from the already verified supplemental source catalog
for this exact revision; no licensing choice or release approval is implied.

`text-scopes.patch` stops a text root's range traversal at descendant text inputs,
Documents and Terminals. Those are independent selection owners. Ordinary labels,
links, headings, lists and table/cell structure remain traversable. The patch does
not remove or hide semantic nodes, change actions, inspect editor values, or alter
the nested owner's own text APIs. Off-layout runs remain readable without geometry.

The unmodified consumer concatenated a nested editor's `TextRun` value into its
parent Document range. That would mix the editor's independent content/indices
with GPUIO's rendered-document selection projection. The regression checks parent
and child text, directed selections, Unicode scalar/UTF-16 offsets and preserved
semantic hierarchy for single-line, multiline and password inputs, Document and
Terminal children. All upstream consumer tests remain enabled.

Reconstruct with `python3 scripts/verify_accesskit_consumer.py --archive <crate>`.
The archive URL/hash are in `UPSTREAM.json`. The verifier reuses the existing
registry-archive verifier, applies the exact patch without fuzz/offsets, compares
every original file and checks license hashes. Run the consumer tests through
`./scripts/gpuio exec cargo test -p accesskit_consumer --locked`; `scripts/gpuio test`
includes them because this dependency is intentionally outside workspace membership.

Root and composed extension backends must propagate the same Cargo path patch;
dependency manifests cannot impose patches on consumers. `compose_backend.py`
and the generated application manifests/Dune dependencies carry it. No unrelated
dependency versions are changed. Remove the adaptation only after a pinned upstream
consumer provides equivalent scope isolation and the regression passes without it.

This is a prerequisite for rich rendered-text accessibility, not proof that GPUIO
already publishes rich TextRuns, implements OS selection actions or passes
VoiceOver. Those implementation and native acceptance steps remain required.
