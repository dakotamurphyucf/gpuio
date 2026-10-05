# Real Japanese IME acceptance — OCH-17

On 2026-10-04, the public Component Studio Settings editor passed a real macOS
Japanese Romaji/Hiragana input-method sequence on Apple M1 Max, macOS 14.5
(23F79). This uses the installed OS input method and its candidate window;
it does not invoke `setMarkedText` or another text-client callback directly.

The application baseline is `b8d261c`; this follow-up adds the harness and
restoration tests. A fresh isolated gallery build and final harness run 008 pass. Its executable
SHA-256 is `6741d16dc1501729ffb5608d021cafb0731081687d99b132e50a114f9fd314c4`.
Earlier exploration runs 006/007 used
`c2da583f4b4e0dff16e445fee1de920d6b1ebc8a583cac9c4efc682ed9a47545`.

## Observed behavior

- Real physical-keycode input of `nihongo` starts marked Japanese composition.
  The editor exposes preedit while the form's committed value remains `a`.
- Space opens the actual OS conversion candidate window, owned by the child
  application at window layer 20. Its 159×140-point bounds were `(1107,598)`,
  beside/below the editor at `(1108,568,242,23)`. The harness checks coarse
  placement; it does not establish exact caret pixels or every display scale.
- Captured candidate screenshots were inspected: kana/kanji choices and the
  selected row are visible. Candidate ranking reflects existing OS conversion
  state; the test does not reset or assert a user's dictionary/ranking.
- Return commits the selected Japanese text. A single Command-Z restores `a`;
  Command-Shift-Z restores the complete commit. A subsequent `ni` composition
  cancels with Escape without changing the committed form value.
- The original U.S. source and all observed enable states are restored and
  independently re-enumerated. The owned window closes and the child is reaped.

Exploration run 006 committed `a日本語`; the repeatable harness run 007 committed
`a日本ご` after cycling candidates. Final rebuilt run 008 committed `a日本語`. All passed undo/redo and cancellation.
Neither candidate spelling is a framework assertion: the OS owns conversion.

## Reproduction and recovery

Build the public gallery with the isolated repository environment, then run on
an otherwise idle macOS desktop with Accessibility and screen-recording access:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/gallery/main.exe
python3 scripts/test_macos_ime.py --output scratch/ime-acceptance-001
python3 scripts/test_mac_input_source.py
```

Use a new output directory for each run. Before enabling/selecting the installed
Japanese source, the harness saves `original.json`. It modifies only the Japanese
Romaji source family and restores it in `finally`, including a test exception,
KeyboardInterrupt or handled SIGTERM. Four portable tests cover the successful
scope, partial selection failures, interruption and an unavailable method.
These are required CI checks; the physical test is explicitly invoked on a desktop.

SIGKILL or power loss cannot execute cleanup. Recover from the saved record with:

```sh
python3 scripts/test_macos_ime.py --restore scratch/ime-acceptance-001/original.json \
  --output scratch/ime-recovery-001
```

The restoration report must match the original state. Restoration does not rewrite
unrelated input-source preferences if they change concurrently. The API contracts
are in the installed Apple SDK's `HIToolbox/TextInputSources.h`:
`TISEnableInputSource`, `TISSelectInputSource`, `TISDisableInputSource`.

Direct-to-PID keyboard events produced Latin input in the initial exploration.
The successful sequence re-established foreground after source selection and
used `CGEventPost`. It checks child liveness and foreground before each event;
focus checking and posting are not atomic, so do not interact with other apps
during this short sequence. This is harness evidence, not a production editor fix.

Local artifacts are in `scratch/agents/root-20261004-resumed/ime-native-006/`
`ime-native-007/` and `ime-native-008/`, with adjacent logs. They contain candidate-only screenshots,
window/editor bounds, application output, original and restored source states.
They are evidence, not build dependencies.

## Coverage limits

This validates one real Japanese input method in a single-line public editor on
one physical macOS desktop. The existing US dead-key Settings sequence separately
covers layout changes and native identity during composition. This does not
qualify every input method, multiline/scrolled candidate geometry, VoiceOver,
external file drag/drop, Linux IME or the complete OCH-17 release gate.
