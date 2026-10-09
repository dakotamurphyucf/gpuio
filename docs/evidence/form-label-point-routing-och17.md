# Form label sizing and screen-point routing — 2026-10-08

Local macOS 14.5 arm64, worktree based on `497236b7e7bbfa0f83c8e4e97d53a2892ff0a247`.
The [27-file archive](form-label-point-routing-och17/reports.tar.gz) and
[manifest](form-label-point-routing-och17/manifest.json) retain exact source,
before/after reports, logs and owned-window captures. Every archived file was
read back and compared with its input. The passing gallery executable SHA-256 is
`2694c047b64f750f17758ca521c3b5c7bb4fa70f13f9135f3a334adecdd382ee`.

## Reproduction and repair

A broader screen-point fixture passed header/navigation/button checks but could
not reveal the later Document title editor. Three failed attempts are retained:
ordinary wheel targeting, a gutter-target hypothesis that did not help, and a
capture run. The capture shows rich form labels wrapping one character per line.
The existing form geometry test checked placement and column widths but missed
this collapse. A new assertion fails before the repair: the rich label receives
**0px width and 508.5px height**.

`Form.render_item` now defaults vertical label rows to the item width; caller
label styles can still refine that default. The rich content grows into the
available row space beside the required marker and can shrink/wrap within it.
Horizontal labels retain their configured allocation. Native ownership, the
bridge and rendering engine are unchanged. The public interface and example
walkthrough explain the allocation. One existing API test assumed vertical
labels had no width; it was explicitly updated to expect the new percentage
default, without promoting its intermediate exception output.

The complete real form walkthrough passes **52 geometry cases** over both
themes, four form sizes, one/two/three columns, horizontal/vertical labels and
mixed/reordered items. It checks label dimensions, actual edits, three native
actions, retained editor identity/undo, hiding and page retirement. The after
capture confirms readable labels, including the explicitly vertical summary.
This does not qualify every display scale or explain the separately reported
list jitter.

## Screen-point evidence

`scripts/test_macos_hit_testing.py` calls the actual system-wide
`AXUIElementCopyElementAtPosition` at each target's center. It requires the child
PID and semantic identity: the result must equal the target, or its bounded
AXParent chain must reach that target. Returning only its containing AXWindow
fails. Every copied AX reference is released; the owned application is reaped on
failure and closed normally on success. Reports retain observations before
assertions, and failed identity checks are not retried until they pass.

After repair, **all 21 points return the exact requested element**: header and
navigation buttons, button before/after its update, editor, switch, slider,
dialog/drawer/confirmation/popover controls, and restored triggers after each
dismissal. The application exits zero. Together with the earlier
[two-window editor regression](window-accessibility-och17.md#screen-point-routing-repaired--2026-10-04),
this extends actual routing coverage beyond editors. It does not establish
VoiceOver speech/navigation, all catalog geometry, per-frame paint correctness,
character-level text hit testing or Linux desktop acceptance.

## Commands and checks

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/gallery/main.exe
python3 scripts/test_gallery.py --section forms --images <fresh-output-directory>
python3 scripts/test_macos_hit_testing.py --output <fresh-output-directory>
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @test/view_api/runtest @fmt
python3 -m py_compile scripts/test_macos_hit_testing.py scripts/gallery_forms.py
python3 scripts/audit_example_docs.py
```

These checks, `git diff --check` and workflow actionlint pass. The archive retains
an invalid guessed test target rejected before compilation and the subsequent
expected API-assertion failure before updating the old width assumption.
The Foundation workflow now includes the bounded native routing fixture;
hosted acceptance of this change remains pending. No milestone ticket is closed
by these scoped results.
