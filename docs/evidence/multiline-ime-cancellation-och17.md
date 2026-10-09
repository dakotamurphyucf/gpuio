# Multiline native IME cancellation — OCH-17 / OCH-41

Real macOS Japanese IME testing exposed and repaired a multiline cancellation
bug: after committing Japanese text and undoing/redoing it, composing over that
text and cancelling removed the earlier commit. The native regression reproduces
the same loss (`prefix 🙂\n日本語` became `prefix 🙂\n`) before the repair.

The Base adaptation previously saved a pre-composition draft only for
single-line inputs and used rollback only for formatted input. It now saves the
bounded draft/active selection for every bridge-owned editor mode and restores
both when marked text is cancelled. Cancellation adds no undo entry; the prior
commit remains undoable/redoable. The regression covers both a reversed selection
and an explicit UTF-16 replacement range, with multibyte text. Ordinary unbridged
Base inputs retain their existing behavior. No source pin or public wire/API
version changes.

The gallery's Working notes field now explicitly sets `~submit_on_enter:false`
and temporarily shows its public composition state in the existing status line.
The shared configuration defaults to submit-on-Enter, appropriate for a composer;
notes require an explicit newline policy. The final newline test explicitly
collapses the restored selection to the end, because cancellation correctly
preserves the selection that existed before composition.

## Final physical acceptance

Both final runs pass on clean source
`9bd6e7ee6464119b367cd1d8573b05e9ed506f91`, using the same preserved gallery
executable SHA-256
`7834d90d5b019bc7e3e2e840e1d08e7f99ea553db80bb71faeec4000b8212390`.
This is a development build, not a performance measurement. Hardware is Apple
M1 Max, macOS 14.5 arm64, built-in Retina desktop. One owned application runs at
a time, with actual foreground OS key events and the installed Japanese
Romaji/Hiragana input method. No direct `setMarkedText` call substitutes for it.

- Multiline run 007 uses 41 logical lines, a multibyte/emoji prefix and a long
  final line. The public viewport query reports line 41 at 1054 px vertical
  scroll. It checks preedit, actual OS candidates, commit, exact undo/redo and
  cancellation first with wrapping, then after disabling wrapping and native
  horizontal reveal. Both phases pass; each commit needed two Returns, observed
  through the public composition state instead of assuming a fixed count.
- The earlier committed text survives cancellation in both layouts. Return after
  collapsing to the end inserts a newline; one undo restores the exact draft.
- The single-line Settings follow-up also passes real preedit/candidate/commit,
  committed form-state observation, undo/redo and cancellation. This run committed
  `a日本ご`; candidate spelling/ranking is owned by the OS and is not a framework
  assertion.
- Both runs independently re-enumerate and restore the original U.S. source and
  every observed enable state. Both applications close and their children are
  reaped. Full source-state records remain local; the versioned report includes
  hashes and equality verification without publishing unrelated preferences.

The candidate windows are owned by the tested child. The harness checks positive
size and coarse placement near the editor, and captures only owned windows.
Wrapped and nonwrapping editor/candidate captures were inspected at this binary.
This does not prove exact caret pixels, every display scale, every IME or VoiceOver.

[Machine-readable report and artifact hashes](ime-cancellation-och17/report.json),
[multiline observations](ime-cancellation-och17/multiline-observations.json),
[multiline log](ime-cancellation-och17/multiline.log),
[single-line log](ime-cancellation-och17/settings.log).

Captured examples: [wrapped editor](ime-cancellation-och17/first-editor.png),
[wrapped candidates](ime-cancellation-och17/first-candidate-0.png),
[nonwrapping editor](ime-cancellation-och17/after-wrap-change-editor.png),
[nonwrapping candidates](ime-cancellation-och17/after-wrap-change-candidate-0.png).

## Reproduction and verification

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-image-tests,performance-diagnostics --lib
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -p gpuio-native --features native-image-tests,performance-diagnostics --lib --tests -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/gallery/main.exe @fmt
python3 scripts/test_mac_input_source.py
python3 scripts/test_macos_ime.py --editor multiline --output scratch/ime-multiline
python3 scripts/test_macos_ime.py --editor settings --output scratch/ime-settings
```

927 native tests pass with two existing skips; strict Clippy, gallery build and
format checks pass. 269 standalone Base input-related tests pass with the pinned
local overrides documented in [range geometry](editor-range-geometry-och41.md).
Four input-source restoration tests pass. The canonical Base patch SHA-256 is
`c956cb8541d6b530182eaccad5de38f3640bd666f652ca6f02c7b6dc27841c25`;
reconstruction from the verified archive matches all 235 tracked files exactly.
Unified patch context may end with a space-only line; the narrowly scoped patch
whitespace attribute permits that syntax without relaxing source-file checks.

Local development attempts remain retained: 001 stopped at an AX label-read
error before composition; 002 found the real cancellation loss; 003/004 passed
both repaired IME phases but rejected the gallery's submit-on-Enter policy;
005 passed both phases and exposed a newline oracle that ignored the restored
selection. 006 passes the corrected complete harness, and 007 repeats it at the
clean committed source. All attempts restored input-source state. None of the
failed attempts is claimed as full acceptance.

The milestone remains open for the other documented macOS native/accessibility,
catalog, GPU/presentation and distribution requirements. Linux desktop IME remains
OCH-47; these results do not establish it.
