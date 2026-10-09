# Streamed selection and stale native input — OCH-17

Local macOS checkpoint, 2026-10-07, following
[mapped multi-click selection](rendered-multiclick-selection-och17.md).
OCH-17/OCH-41 and milestone 07 remain open. This change does not publish AX
selection or qualify VoiceOver, IME, Linux desktop or physical presentation.

## Implementation

A compatible append converts genuine Select All into a current directed range
with its own provenance. Plain Copy uses the mapped range; source-format Copy
keeps the exact old source snapshot. New content remains unselected. An old
terminal structural separator retains its offset only while it remains that
separator; otherwise its endpoint stays at the old content edge. This does not
trim owned code linebreaks or infer selection by searching repeated text.

The window controller stages and rebinds captured anchor, cursor and pending
extension positions along with the document. A held gesture therefore continues
against the current preparation after an append or compatible resource refresh,
even if streaming occurs before the first drag movement. Invalid mapping or
incompatible replacement cancels the old gesture. Other participant cleanup runs
outside the window-state lease; the initiating view resets its own owners to
avoid re-entering its lease. A new local selection survives the queued empty
notification from the cancelled gesture.

Painted native text/object callbacks carry the bounded preparation revision and
reject obsolete presses before mutating owners or entering legacy Copy fallback.
The guard covers the tested same-view replacement-before-repaint case and checks
selection policy. It is separate from the interaction epoch, which ordinary
mouse-down clearing intentionally changes.

## Evidence

Before-001 reproduces a lost Select All logical range. Before-002 restores old
text from a replaced frame's callback. Held-before-007 loses the captured drag
range after streaming. The final tests cover:

- Genuine Select All and backwards explicit full selection through same-paragraph,
  new-paragraph, quote and fenced-code append; exact plain/source Copy, owned
  code newline preservation and rejection of old queued requests.
- Separator-only and empty selection collapse without selecting inserted text.
- Old ordinary/atomic callbacks with single/double/triple clicks before repaint;
  current-frame interaction still works afterward.
- A held drag through append, an append before first movement, and a resource-only
  refresh with stable parser configuration.
- Cancellation on incompatible replacement, including a new Select All installed
  in the same application update.
- Actual macOS window/GPU selection pixels, continued backwards held drag after
  streaming, exact logical range and native keyboard Copy, plus the existing
  Unicode, policy, style, cross-document and teardown suite.

Fixture failures are retained: compile-006/009 used incorrect test type names;
010/016 used different owning parser extensions or added a new renderer name
instead of refreshing resources. 017 registered the fixture before source-profile
installation overwrote it. Corrected resource-021 installs the fixture after
profile installation and refreshes an existing renderer name. No production
parser rule, selection assertion or release budget was relaxed.

All checks use the repository's isolated toolchain with `GPUIO_JOBS=2`:

- Full Rust workspace: passed (`cargo test --offline --workspace --locked -j2`).
- Native canvas/image unit suite: 1,102 passed / 2 ignored.
- Strict workspace/all-target Clippy with canvas/image/presentation features: passed.
- `dune build -j2 @all @runtest @fmt`: passed, including rebuilt examples.
- Actual native `native_highlight_document` suite: passed, including
  `GPUIO_RENDERED_HELD_STREAM_OK`.
- Fresh rebuilt gallery Select All/Copy and normal shutdown: passed with Rust
  backtraces; the report identifies the exact executable hash.
- Cargo format and diff whitespace checks: passed. Base patch reconstruction:
  exactly 237 files, excluding generated `Cargo.lock`. Example documentation audit:
  429 sources / 266 reviewed groups / 0 pending.

The [archive](rendered-stream-selection-och17/reports.tar.gz) and
[manifest](rendered-stream-selection-och17/manifest.json) retain commands, logs,
source hashes, failures and the qualified patch. Typed clipboard contents were
restored and native test children reaped. No VoiceOver/system settings changed.

## Limits and remaining work

These checks cover native dispatch and actual GPU/clipboard behavior, not physical
mouse/keyboard input or accessible selection. Arbitrary custom glyph/Copy mappings,
zero-byte selected objects, wider held-drag reflow/virtualization behavior and
selection hot-path performance still need qualification. Rich TextRun publication,
OS selection actions and broader release gates remain required. Existing table
startup and physical-presentation budgets have not been waived. Hosted Linux
and macOS evidence must be identified by its exact revision; no compilation result
establishes Linux desktop acceptance.
