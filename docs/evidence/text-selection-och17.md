# Native editor accessibility selection — OCH-17 / OCH-41

The missing source-view selection range is repaired in
`e4d35ae86c3985f520323e725b1c907b17d4a51b`. The existing focus-owning editor node
now has native TextRun children, current anchor/head metadata and a guarded
selection action. Read-only code/source pages and plain single-line/multiline
editors share the adapter. Passwords remain excluded, including revealed fields.
There is no vendor, protocol or public OCaml API change.

The former gallery binary, built from `9bd6e7e` with SHA-256
`7834d90d5b019bc7e3e2e840e1d08e7f99ea553db80bb71faeec4000b8212390`, reproduces
`Missing AXSelectedTextRange` after native Select All in Code preview.
[Before log](text-selection-och17/before.log).

The final real macOS check passes on clean source `e4d35ae`, using the preserved
development gallery executable SHA-256
`db8a1f539b5c1000d74ac32ece6970e6e2278f2bb42c7db67002fe6a31593c5f`.
Hardware: Apple M1 Max, macOS 14.5 arm64, built-in Retina desktop. One owned
foreground application receives OS keyboard events; external AX queries inspect
the results. This is functional validation, not a performance measurement.
[Raw report](text-selection-och17/macos-report.json),
[run log](text-selection-och17/macos.log).

The check covers:

- Source Select All reports the full installed page and its exact UTF-16 range.
- An external AX request selects `世界`; native Copy produces exactly those
  characters, and Backspace preserves the read-only document.
- Multiline requests select an astral emoji, the joined family emoji, a combining
  sequence, Japanese text and text spanning a newline, with exact UTF-16 ranges.
- Native Select All and backward Shift-Left update the reported selection.
- A caret after the final newline reports an empty selection at the correct end
  offset in both the source and multiline views.
- The owned application closes successfully; the existing pasteboard helper
  restores all eagerly readable original representations and verifies equality.

An intermediate run passed AX selection checks but failed an immediate `pbpaste`
assertion. The final harness instead waits for the native pasteboard change count
and reads its UTF-8 representation directly, using the existing preservation
helper. No production clipboard repair is claimed.

Native regressions cover CRLF and UTF-8 boundaries, empty text, foreign/out-of-range
positions, reversed selection, read-only selection, disabled controls, stale text
revision/run requests, rejection during active composition and hidden/revealed
password privacy. The complete native library suite passes **930 tests**, with
two existing platform skips. Strict native Clippy and gallery/format builds pass.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-image-tests,performance-diagnostics --lib
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -p gpuio-native --features native-image-tests,performance-diagnostics --lib --tests -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/gallery/main.exe @fmt
python3 scripts/test_macos_text_selection.py --output scratch/text-selection-check
```

The final run used the same command with `--executable` pointing to the preserved
binary. Local build/test logs are `ax-selection-native-suite-003.log`,
`ax-selection-clippy-003.log` and `ax-selection-gallery-build-002.log` in the
implementing agent's scratch directory. They are not build dependencies.

Scope remains the installed editor page, not the complete paginated document.
Runs are generated only during accessibility tree construction and share the
parent's text allocation. The [contract](../design/document-accessibility.md)
states remaining limits: logical rather than wrapped visual lines, no character
rectangles or word-navigation metadata, separate rendered-Markdown selection and
unqualified VoiceOver reading/tracking. This does not close either ticket or
qualify Linux desktop behavior.
