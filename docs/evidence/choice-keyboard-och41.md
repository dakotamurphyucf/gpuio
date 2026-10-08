# Choice picker foreground keyboard check — 2026-10-08

The public gallery choice walkthrough now covers real foreground keyboard search
in addition to its existing accessibility-value setter. On macOS 14.5 arm64 it
passes typing `4095` over the retained `4096` query, Backspace to `409`, retyping
the last digit, Down/Return selection of Workspace 4095, popup dismissal and
reopening with the updated query retained. The selected-result text verifies the
native event reached Bonsai; matching the query alone would not prove commitment.

`scripts/gallery_choice_picker.py` first verifies the active US/ABC layout. It
changes neither input sources nor clipboard. Global OS key posting checks the
owned application's foreground state before every event and restores the
driver's original posting function afterward. The complete walkthrough also
passes grouped multiple selection/clear, controlled single selection and disabled
policy, the 4,096-item AX setter path, native Escape, and empty/create/reset.
The owned application closes normally and the driver exits zero.

The initial attempt failed before typing because the fixture required the query
editor's `AXFocused` to be true. The native picker intentionally assigns accessible
focus to its active result through `aria_active_descendant_for`, while the query
retains native keyboard ownership. The corrected fixture requests query focus
once and proves key delivery through actual edits and commitment. No production
focus, editor, picker or protocol code changed. This does not claim real IME or
VoiceOver acceptance.

The [six-file archive](choice-keyboard-och41/reports.tar.gz) and
[manifest](choice-keyboard-och41/manifest.json) retain both attempts, owned-window
captures, source and summary. Files were read back and verified. Source base is
`1ac94f45`; gallery SHA-256 is
`2694c047b64f750f17758ca521c3b5c7bb4fa70f13f9135f3a334adecdd382ee`.

```sh
python3 scripts/test_gallery.py --section choice-pickers --images <fresh-directory>
python3 -m py_compile scripts/gallery_choice_picker.py
```

These pass locally. A new hosted choice-picker step will exercise the
expanded fixture on the next revision; that result and consolidated release
acceptance remain pending.
