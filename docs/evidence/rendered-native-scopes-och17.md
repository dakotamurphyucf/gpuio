# Visible document text scopes — OCH-17

Local macOS arm64 work after `7dbff84d`, 2026-10-07. This extends
[logical document publication](rendered-logical-document-och17.md) into native
nested structures. It is not complete OS accessibility or release acceptance.

## Reproduced problems and change

The previous native list exposed `firstnested λ🙂 textlast` where prepared text
contains line breaks between items. A two-column table exposed
`FirstSecondαβ` instead of spaces between cells and newlines between rows.
Two paragraphs in a blockquote also reused the same native accessibility ID.

Preparation now retains each original nested block slot's semantic owner,
including empty slots for rules and definitions. Rendering carries these IDs as
metadata in `NodeRenderOptions`; prepared revision IDs never become native keys.
Nested blocks complete their own text ranges during their existing synchronous
prepaint lifetime. Tables additionally complete native cell and row scopes;
description lists complete terms, definitions, entry groups and the list itself.

Each callback visits only the actual completed subtree and associates represented
text with its existing direct children. It validates ordered, contained ranges
before inserting missing separators or alternatives between those children.
Original native child identities, actions and relative order remain intact.
Completed descendants are never mutated or moved in the builder's postorder
storage. Missing text receives no fabricated geometry or input handler.

Blockquote and list continuation children now use their original sibling index
for native identity, fixing collisions. Root publication no longer duplicates
separators supplied by native scopes. A document with empty semantic nodes still
gets a caret endpoint. Clamped previews keep completion disabled.

The child-slot map is included in actual retained-allocation accounting and the
conservative preparation bound. Document input limits remain unchanged. Rendering
does not clone the full link-reference context for each child.

## Validation

All **1,148 native tests pass, with 2 existing ignored tests**. Seven new tests
cover nested lists, tables, blockquotes, description lists, custom block alternatives,
original child slots with continuation paragraphs/rules, and empty heading/fence/rule
caret endpoints. They compare actual tree reading text against prepared text and
round-trip every legal prepared endpoint. Description coverage also requires a
realized `DescriptionList` node; the initial unsupported empty-value YAML fixture
fell back and was replaced before accepting that coverage.

Existing tests still cover control actions/identities, replacement, virtualization,
measurement-only rows, clamped previews, atomic edges, Unicode and CRLF. Strict
all-target workspace Clippy passes. GPUI Base reconstructs exactly from the pinned
cached archive: **243 files**, excluding Cargo.lock; maintained patch SHA-256
`0f35af5474974eb2f76db05370d3b9faa167d3143672359819dd8a7d46f9cf74`.

The full Rust workspace suite and `dune build -j2 @all @runtest @fmt` pass.
Actual macOS editor/document and NSView text-client checks pass. The rebuilt
gallery passes native reading order, rendered Select All/Copy and normal close;
its child exits cleanly and the typed clipboard is restored. Rust formatting,
diff checks and the example documentation inventory pass.

The [report archive](rendered-native-scopes-och17/reports.tar.gz) and
[manifest](rendered-native-scopes-och17/manifest.json) retain commands, logs,
initial failures, source hashes and exact reconstruction evidence.

Dedicated inline-object/link interiors and clipped nested content still need
coverage and any necessary repair. These local checks establish no current-source
Linux or complete OS text-selection acceptance.
Final-paint selection, guarded OS actions and actual VoiceOver validation remain
open, as do full catalog, performance/resource and distribution acceptance.
OCH-17/OCH-41 and milestone 07 remain in progress.
