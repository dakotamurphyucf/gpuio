# Native palette commands — OCH-41

Local qualification on macOS 14.5 arm64, 2026-10-06 UTC, at `c0694a2` plus the
archived source patch and new files. This is a controller addition, not complete
palette/catalog or release acceptance.

`Command_palette.Command` and `Gpuio_eio.Palette_controller` expose correlated
native read/focus/query/highlight operations. Requests capture the exact observed
window/node/subscription. An optional query fence refreshes current native editor
state before admission, including edits whose notifications are still pending.
Explicitly clearing the highlight survives redraws and configuration changes;
subsequent Down/Up select the first/last enabled row. Query updates and highlight
changes never activate a command. [Contract](../design/palette-commands.md).

## Validation

- Full native suite: **963 pass, two existing skips** with `native-image-tests`.
  Three new TestPlatform scenarios cover query replacement, filtered/disabled
  selection rejection, explicit clear across render/configuration, native Up/Down,
  no activation/dismissal, unnotified query ABA rejection, subscription replacement,
  observer detachment, invalid text, composition preservation and hidden-query focus.
- Independent Rust/OCaml fixture bytes cover appended request 22 and response 80,
  truncation/trailing data, invalid query/ID/revision and malformed snapshots.
- Full OCaml `@runtest`, formatting and gallery build pass. Controller expect
  coverage rejects replies older than observations, replacement subscriptions and
  reset. App tests check wrong observer identity, duplicate/regressing replies,
  query validation, 64-request admission and exactly-once completion on closure.
- Strict native/protocol all-target Clippy and Rust formatting pass. Python syntax
  and whitespace checks pass. No expectations were automatically promoted.
- A fresh independently installed gallery consumer builds and passes catalog
  negotiation, then the complete real macOS Feedback walkthrough. Added actions
  clear/choose highlights, retain explicit no-selection through ordinary view
  updates, set/clear query text and return focus. Real Down/Up and typing/Backspace
  continue to update observations. Existing rich content, header editing,
  keyword/Escape/searchability, command/menu and notification cleanup checks pass.
  Screenshots were inspected: controls wrap inside the palette and preserve the
  rich command rows. This is functional presentation evidence, not final visual
  design or screen-reader qualification.

Installed executable SHA-256:
`1e6a5991e1b1240d70821efa7157837fd45a4346f47ae018ce6cf3f4574cffc8`.
The driver closes/reaps its application and restores all saved clipboard
representations. No OS preferences, input-source configuration or VoiceOver were
changed. Composition guards here have TestPlatform evidence; this slice does not
add physical IME or VoiceOver acceptance.

## Retained failures and limits

Initial native test compilation used a missing helper and the wrong operation
name; both were corrected. A subsequent incorrectly named test filter ran zero
tests and is **not** acceptance evidence; the targeted three-case run and full
963-test run establish actual execution. Clippy caught test fixture field
reassignment and passes after an initializer-only correction.

The first installed walkthrough omitted the window title in two test helper
calls. It failed looking for a field named `store` in a window named `Preview
commands`. Correcting those calls required no application change; the second
complete walkthrough passes against the same binary. Both runs' logs are retained.
The first traceback's source line reflects the already-corrected file on disk;
its exception records the original erroneous arguments.

[Commands, exact source patch/new files, logs and screenshots](palette-commands-och41/validation.tar.gz)
have a [verified checksum manifest](palette-commands-och41/manifest.json).
Loading, persistent embedding, atomic query-fenced external-result publication,
OS popup menus and remaining chart/catalog qualification stay open. Hosted run
37398392336 covers earlier `c0694a2`, not this addition; current-source Linux/hosted
checks and the broader macOS release gates remain required.
