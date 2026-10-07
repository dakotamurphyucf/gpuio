# Declared custom block selection — OCH-17

Local macOS checkpoint, 2026-10-07, following
[streamed selection](rendered-stream-selection-och17.md). OCH-17/OCH-41 and the
full milestone 07 remain open. This is native selection integration, not rich AX
publication or release acceptance.

## Contract and change

The accepted [document-profile contract](../design/document-profiles.md#declared-block-text-selection)
already gives declared block `Text` to the reader's ordinary partial glyph
selection. Whole-document Copy can independently use `MarkdownNode.text`. The
old logical selection projection used those Copy bytes as character coordinates
and classified differing glyphs as unmapped. This prevented a logical range for
otherwise supported native partial selection.

The projection now addresses the actual declared block glyphs, including an empty
presentation. Ordinary text, separators and inline atomic alternatives retain
their existing behavior. Partial plain Copy uses selected glyphs; whole-block
Source Copy uses declared Markdown, and partial Source Copy falls back to glyphs.
No character mapping is guessed between unrelated glyph and Copy strings.

Whole-document Copy retains its existing representation. When compatible streaming
freezes Select All, its logical range follows old glyphs and an immutable snapshot
preserves a differing whole-Copy representation and the original source. New
selection or clear discards that snapshot. Its native Copy scope remains active
when its declared representation has content but its glyph range is empty.

Preparation independently counts logical and whole-Copy bytes, including different
separator requirements. It checks the existing aggregate bound without allocating
another full Copy string. Empty glyphs cannot hide oversized alternatives; source
and SDK generation limits remain unchanged. No new OCaml protocol/API or
synchronous OCaml callback is introduced.

## Validation

Before-001 reproduces mismatched and empty glyph projections exposing alternative
Copy characters. Empty-before-007 reproduces the missing active scope after freezing
whole Copy with empty glyphs. Tests cover:

- Partial backward glyph requests, plain/Source Copy, declared whole-block
  Markdown, exact whole-document Copy and repeated compatible appends.
- Actual current resource glyphs after refresh, stale request rejection and
  explicit clearing of an older whole-Copy snapshot.
- Empty declared glyphs without invented characters, plus exact and aggregate
  Copy-admission boundaries independent of glyph size.
- Native double/triple clicks on the second of two identical custom blocks,
  shared range/Copy and explicit Source-mode behavior.
- Actual macOS window/GPU selection pixels and keyboard Copy for `世界`, whole
  declared Copy, streaming retention, and empty-glyph whole-Copy scope.

Intermediate fixture failures are archived: after-002 expected a trailing newline
that existing Source reconstruction trims; native-004 set Source mode directly
while the scene still declared Plain on redraw, corrected by configuring the
scene; empty-before-006 passed a Rope where the test parser requires text. These
fixes do not weaken production selection assertions or change the existing source
formatting/parser contract.

Final qualification passed on local macOS arm64:

- Native library: **1,105 passed, 2 ignored**; full Rust workspace tests passed.
- Strict workspace Clippy with native features, Rust formatting, and
  `dune build -j2 @all @runtest @fmt` passed using the isolated toolchain.
- Native-014 passed the actual GPU/keyboard Copy suite, including
  `GPUIO_CUSTOM_GLYPH_SELECTION_OK` and `GPUIO_NATIVE_HIGHLIGHT_DOCUMENT_OK`.
- Fresh-gallery shutdown-013 passed rendered reading order, actual Copy and clean
  child exit; typed clipboard restoration is recorded. This does not qualify
  rich rendered-text AX selection attributes/actions.
- Vendored Base reconstruction matched **237 files** exactly, excluding
  `Cargo.lock`; patch SHA-256
  `d6a8c86e659f5e233525388f9e068fc65874360b3294b01ded79ce0245f7a2aa`.
- Example documentation structural audit remains **429 sources / 266 reviewed
  groups / 0 pending**; `git diff --check` passed.

The evidence archive contains **28 files / 547,839 compressed bytes**, verified
against its manifest. Its metadata records the parent commit and the exact
qualified source hashes. Hosted run `37612466848` covers the preceding
`4162d450` checkpoint and was still running when this local evidence was recorded;
it does not validate these changes.

The [archive](rendered-custom-selection-och17/reports.tar.gz) and
[manifest](rendered-custom-selection-och17/manifest.json) retain exact commands,
logs, source hashes, failures and qualified changes. Clipboard contents are
restored by the native drivers, which also reap their child processes. No
VoiceOver/system settings are changed by this checkpoint.

## Remaining scope

Selecting an empty-copy **atomic object** is different from a declared empty Text
presentation: it needs ordered object edges, since an empty byte interval alone
cannot distinguish object selection from a caret. Inline/opaque/NonText object
qualification, broader reflow/virtualization, hot-path performance, rich AX
publication/actions, physical-input/VoiceOver/IME and the broader release gates
remain required. No Linux GUI or physical presentation acceptance is inferred.
