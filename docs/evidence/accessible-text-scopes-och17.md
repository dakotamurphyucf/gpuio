# Independent accessibility text scopes — OCH-17

Local macOS arm64 checkpoint, 2026-10-07. OCH-17/OCH-41 and milestone 07 remain
open. This is a prerequisite for rich document accessibility, not its acceptance.

## Observed behavior and adaptation

The exact AccessKit consumer 0.38.0 concatenated a nested editor's text into its
parent Document's text range. An isolated probe returns `beforeeditorafter`.
That mixes two independent selection owners and makes the parent's offsets
incompatible with GPUIO's rendered-document projection.

A narrow [consumer fork](../../vendor/accesskit-consumer/GPUIO.md) stops text
traversal at descendant text inputs, Documents and Terminals. The semantic tree,
child actions and each child's own text APIs remain intact. Ordinary labels,
headings, links, tables and cells remain traversable, including off-layout text
without geometry. Protocol and platform-adapter versions are unchanged.

The regression fails before the patch (`A😀editor終` instead of `A😀終`) and passes
afterward. It checks single-line, multiline and password inputs, Document and
Terminal children; backward selection; scalar/UTF-16 offsets; child-local text
selection; retained heading/link hierarchy; and ordinary table/cell/label runs.

The fork is wired into the root and all five composed backend Cargo workspaces,
the generator, default/native Dune dependencies and the table-adapter probe.
Each separate consumer workspace needs its own Cargo patch. The six lockfiles
change only the consumer's registry source/checksum; versions and dependency
edges are preserved. CI verifies source reconstruction and explicitly runs the
excluded consumer dependency's own tests.

## Validation

- All **207 consumer tests** pass, with all upstream tests retained.
- All **1,118 native library tests** pass; 2 existing tests remain ignored.
- Actual macOS `native_editor` and `native_document` pass. Typed clipboard data
  is restored after the desktop driver. These are existing native behavior
  checks, not a rich-document VoiceOver test.
- Freshly rebuilt gallery reading order, actual rendered Copy and normal window
  shutdown pass; child exit is zero and typed clipboard contents are restored.
- Complete Rust workspace tests, strict all-target Clippy and Dune
  `@all @runtest @fmt` pass through the isolated toolchain with two jobs.
- Exact registry reconstruction passes: 10 original files, one patch, three
  pinned upstream notice hashes. Eight backend composition tests pass.
- Five affected backend walkthroughs explain the dependency propagation and its
  limits. Documentation inventory: 429 sources / 266 reviewed groups / 0 pending;
  this is structural coverage, not an independent content review.

The [archive](accessible-text-scopes-och17/reports.tar.gz) and
[manifest](accessible-text-scopes-och17/manifest.json) preserve probe source,
regression failure and success, final commands/results, source hashes, native
logs and notice inventories. Archive members are verified against the manifest.

## Notices and remaining work

Full native notice collection exposed a stale first-party manifest pin. Its
exact delta from `3b5649ab209f5cead00804ef7117050d72f637b7` adds CoreFoundation,
AppKit menu/image features and native test targets. The Apache-2.0 declaration
and attributed root license bytes are unchanged. Only that manifest pin and its
review rationale were refreshed. This does not approve transitive licensing.

The consumer-only inventory has 17 packages, zero missing collected notice texts
and one supplemental entry. The full native inventory has 513 packages, **26
missing collected texts** and 49 supplemental entries. Full notice/license and
release review remain open.

The isolated probe also proves that two empty TextRun nodes produce a degenerate
range with identical UTF-16 offsets. GPUIO's ordered empty-object selection edges
therefore still need an explicit accessible representation. No replacement
characters were added to Copy or to the public protocol in this checkpoint.

Rich TextRun publication, checked native OS selection actions, virtualization and
same-frame geometry, and physical VoiceOver remain required. Broader catalog,
performance/resources, physical presentation, API and distribution gates also
remain open. Hosted run 37629820030 covers the preceding `e4630996`, not this
consumer change; no current-source Linux or clean-machine acceptance is inferred.
