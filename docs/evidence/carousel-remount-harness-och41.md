# Carousel page-remount test repair — OCH-41

2026-10-08, based on `1d14da3c`, physical macOS 14.5 arm64 (Apple M1 Max).
This is a test-driver repair. No application, native widget, dependency, focus
policy or timeout changes are involved.

## Failure and cause

Foundation [37789987337](https://github.com/dakotamurphyucf/gpuio/actions/runs/37789987337)
at `2d429f2a` fails two carousel steps. Automatic movement passes all five fresh
resumes and the pause policies, then cannot focus the track after the last page
return. The interrupted-drag test passes the first two cases, then cannot find
Card draft after page unmount/remount. These are actual hosted failures, not
successful qualification of those steps.

The unmodified lifecycle test reproduces locally: after returning to Carousels
& journeys it requests the editor before revealing the track. The track exists
but its card children are clipped and therefore absent from AX. The gallery page
has a keyed outer scroll owner; the fixture must establish visibility after
returning rather than assuming the previous geometry. `TrackCard::prepaint`
intersects the content mask/window bounds and `write_a11y_info` hides clipped
cards. Focus eligibility also excludes clipped content.

Both drivers now call the existing guarded outer-page reveal helper immediately
after remount, recording before/after bounds. They do not issue Home, reset the
selection or modify editor values at this point. The same selection, original
fresh draft and retained draft assertions remain in place. Timer intervals,
pause samples, drag previews, interruption conditions and deadlines are unchanged.

The corrected local twelve-case lifecycle run measures horizontal remount bounds
`(593,-233,360,210)` and vertical `(593,-273,360,250)` before reveal. Both move to
y=319 afterward. Thus the missing editor was offscreen, not deleted or renamed.
All four theme/axis page-return cases then find the original fresh editor text.

## Validation

Run the two focused sections against the repository gallery and an installed
consumer, sequentially because they use actual foreground input:

```sh
python3 scripts/test_gallery.py --section carousel-lifecycle --images scratch/carousel-lifecycle
python3 scripts/test_gallery.py --section carousel-automatic --images scratch/carousel-automatic
# Repeat with --executable <installed-main.exe>.
python3 -m py_compile scripts/gallery_carousel_track.py scripts/gallery_carousel_lifecycle.py
ruff check scripts/gallery_carousel_track.py scripts/gallery_carousel_lifecycle.py
```

Each local command has a 180-second exception watchdog that preserves normal
harness cleanup. This is scoped desktop input/retirement evidence, not VoiceOver,
physical presentation, hardware trackpad or resource qualification. It does not
resolve the separate loaded-list overlap report. OCH-41/OCH-17 remain open.


Final repository and installed-consumer runs both pass all twelve lifecycle
cases and five fresh automatic resumes (ten hold samples, including no-burst
checks). The original failing local report/log and both hosted failure excerpts
are retained beside the four successful final reports/logs. The installed
consumer binary is reused unchanged from the message-follow repair; no rebuild
or fresh installation was needed for this driver-only change. All owned
applications close normally.

The [verified archive](carousel-remount-harness-och41/reports.tar.gz) includes
those logs/reports and the exact two changed drivers. Its
[manifest](carousel-remount-harness-och41/manifest.json) records member hashes,
source base, platform and both binary hashes. Python syntax and focused Ruff,
example inventory (432 sources/268 groups), catalog audit, actionlint and
whitespace checks pass. The still-running Foundation predates this change;
updated-source hosted qualification remains required.
