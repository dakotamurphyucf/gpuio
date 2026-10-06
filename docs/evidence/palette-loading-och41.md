# Native palette loading — OCH-41

Local qualification on macOS 14.5 arm64, 2026-10-06 UTC, at `50104a6` plus the
archived patch and new design file. This qualifies loading presentation and its
controller contract; palette/catalog and release acceptance remain open.

`Command_palette.Command.Set_loading` and `Snapshot.loading` now expose native
loading state through the existing asynchronous controller. Loading preserves
query identity, editing, selection and undo; optional native query fencing rejects
outdated completion. The palette shows an animated status indicator, supplies busy
semantics and hides empty content while retaining existing command rows.
[Contract and representation compatibility](../design/palette-loading.md).

## Validation

- Final full native suite: **968 pass, two existing skips**, with both
  `native-image-tests` and `native-canvas-tests`. Added scenarios cover snapshot
  sequencing/no-op stability, query fences including composition, activation of
  existing rows while loading, hidden-query and covered-overlay changes, stale
  commands after close, immediate empty-content input/AX suppression, busy/progress
  semantics, reduced-motion static paint, stopping animation and unmount cleanup.
- Independent Rust/OCaml bytes cover the appended Boolean snapshot field and
  command tag 4. All three protocol fixture tests pass, including malformed Boolean
  and truncated inputs. Full OCaml `@runtest`, gallery build and formatting pass.
- Strict native/protocol all-target Clippy, the all-example build, `check-fmt`,
  Python syntax and whitespace checks pass. No expect output was auto-promoted.
- A fresh independently installed gallery consumer builds and negotiates the
  catalog. Its complete real macOS Feedback walkthrough passes. Screenshot samples
  show 76 changed pixels within the spinner, excluding the query caret. Loading
  preserves the highlighted command; an unmatched query hides the empty action;
  native typing works while loading; finishing restores the empty action. Refocusing
  and real Cmd-Z undo the character typed while loading. Existing query/highlight,
  menu, notification and cleanup checks continue to pass.

Installed executable SHA-256:
`cd39e4f8731c517255f77968aa31095c1111e71f9209c275651b60792e8435c1`.
The wrapper reaps its driver/application and restores the saved clipboard.
Screenshots were inspected for indicator placement and content retention; they do
not establish final visual polish. No OS preferences, input sources or VoiceOver
settings changed. Composition has native TestPlatform coverage here; this slice
adds no real IME or screen-reader acceptance.

## Retained failures and limits

Two early native animation assertions incorrectly counted callbacks already queued
before switching to static or closed state. Corrected tests deliver those callbacks
and verify they do not renew the animation. Another fixture used noncontiguous
node ID 20 and was correctly rejected; using the next ID 2 fixes that fixture.
The initial OCaml command requested `.cma` for a native-only library; the corrected
`.cmxa` command and subsequent complete builds pass.

The first installed walkthrough failed initial query focus before reaching loading.
Its cause remains unestablished. After adding failure-only focus diagnostics, two
complete walkthroughs pass against the same application binary; the last includes
the expanded typing/undo check. Passing repeats do not explain the initial failure.
All three logs and the initial failure screenshot are retained. The archive's
`gallery-failure.png` is from that first failure, not the final successful run.

[Commands, source patch/new design, logs and screenshots](palette-loading-och41/validation.tar.gz)
have a [verified 38-member checksum manifest](palette-loading-och41/manifest.json).
Atomic query-fenced external results, persistent embedding, native OS popup menus
and remaining chart/catalog qualification stay open. Hosted run 37400903839 is
still in progress at the earlier `2e9cd54`; it does not cover this addition.
Current-source Linux/hosted checks and broader macOS release gates remain required.
