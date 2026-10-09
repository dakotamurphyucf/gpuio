# Readable inline objects — OCH-17

Local macOS arm64 work after `f1b48182`, 2026-10-07. This extends
[native document scopes](rendered-native-scopes-och17.md). It does not complete
OS text selection, guarded actions or milestone 07 acceptance.

## Change

The new regressions reproduced missing inline alternatives both inside and outside
links: `Before atom λ\r\n🙂 after` was exposed as `Before  after`. Two linked
empty native objects had no readable replacement characters or text endpoints.

Text snapshots now distinguish glyph-fragment coordinates from prepared reading
part coordinates. An atomic alternative can expose readable characters across
multiple TextRuns while retaining whole-object native selection. Reading runs
carry the exact prepared part identity and character interval; current projection
checks and atomic-edge conversion still reject stale IDs and interior selection
positions. CRLF remains one character. Empty objects use the prepared U+FFFC
alternative without changing Copy text. Existing shared text is reused when it
matches the prepared reading text; no glyph geometry is invented for widgets.

The existing logical link owns the snapshot when an object is linked. Otherwise
the actual native object wrapper owns it. The snapshot is published once, while
custom child controls keep their existing nodes, labels and actions. Ordinary
glyph-run identities retain their existing key format.

## Validation

All **1,153 native tests pass, with 2 existing ignored tests**. Five new tests cover
unlinked alternatives, an existing link spanning text and an object, adjacent
linked empty objects, native actions/identity/coordinate retirement, and a wide
horizontal table. The inline tests have their own module and use the shared
document fixtures.

The action fixture dispatches real TestPlatform accessibility requests to three
native buttons across redraw and equal-text replacement. Button IDs remain stable,
old reading-run IDs retire, multiline atomic interiors remain unselectable, and
replacement characters preserve distinct outer edges. These checks use the actual
GPUI tree and action registry; they are not macOS OS action acceptance.

The horizontal-table fixture requires a real overflow track, a second header
beyond the 440-point viewport, an accessible ancestor chain, full prepared reading
text and every legal endpoint. It does not authorize actions on offscreen controls
or replace preview-clipping and final-paint policy checks.

Strict all-target workspace Clippy passes. GPUI Base reconstructs exactly from
the pinned cached archive: **243 files**, excluding Cargo.lock. Maintained patch
SHA-256: `ad611a51c9b4f1cbda95a2d7a3249c96a4b734f27acb79833c57034705482b1d`.

The full Rust workspace suite and `dune build -j2 @all @runtest @fmt` pass
(169.075 seconds and 416.331 seconds respectively). Actual macOS editor/document
and NSView text-client checks pass (65.388 seconds). The rebuilt gallery passes
native reading order, rendered Select All/Copy and normal close; its owned child
exits with status 0 and the typed clipboard is restored. Rust formatting, diff
checks and the example documentation inventory pass (429 sources / 266 reviewed
groups / 0 pending).

The [report archive](rendered-inline-objects-och17/reports.tar.gz) and
[manifest](rendered-inline-objects-och17/manifest.json) retain exact commands,
initial failures, logs, source hashes and reconstruction evidence. Initial action
fixture failure came from asserting before queued events were delivered; the
horizontal fixture initially assumed a word occupied one TextRun. Corrected tests
preserve event delivery and permit valid native run fragmentation. Production
action handling and native glyph segmentation were not changed for those fixes.

These local checks do not establish current-source Linux acceptance. Final-paint selection,
visual-line adjacency, guarded OS actions, physical accessibility/VoiceOver and
full release validation remain open. OCH-17/OCH-41 and milestone 07 remain active.
