# Measured carousel desktop navigation — OCH-41

2026-10-08, physical macOS 14.5 arm64, base `847747ff`. This records a completed
subset of the [physical walkthrough plan](carousel-track-gallery-och41.md), not
full carousel or release acceptance. No production code changed.

The repository gallery and previously staged independent consumer both pass:

- Real foreground US/ABC keyboard editing of Capture's native draft, then
  viewport Home/End and axis-appropriate arrows through four unequal cards.
  Four compact cases cover horizontal/vertical and Dark/Light at Comfortable
  scale. Fully clipped Capture is absent from the exposed AX tree; Home restores
  the edited value.
- A real pointer click in the intersection of the compact viewport and the
  partially clipped Explore button selects that neighbor through Bonsai. The
  fixture asserts that the button is partially clipped, not simply fully visible.
- Reversing stable IDs retains the selected card/draft and updates Home/End
  ordering. Switching between full and compact viewport retains the draft.
- With immediate movement and looping enabled, Left from Capture reaches Publish
  and Right wraps back. Disabled navigation exposes disabled Previous/Next
  controls within the measured track's own group; re-enabling restores navigation.
- Page departure removes the track from accessibility; remount restores the
  application selection and creates a new page-scoped editor with its initial
  text. This distinguishes mounted-card retention from persistent application
  drafts. Both executables close normally.

Representative Dark/horizontal and Light/vertical screenshots were inspected.
The first local run passed; a second adds the stronger scoped disabled-control
assertion. Local `002` and consumer `001` are the final evidence. The earlier
local `001` report remains retained; no failed run was retried into a pass.

## Commands and provenance

```sh
python3 scripts/test_gallery.py --section carousel-track --images scratch/carousel-track
python3 scripts/test_gallery.py --section carousel-track --executable /path/to/installed/main.exe --images scratch/carousel-track-installed
python3 -m py_compile scripts/gallery_carousel_track.py scripts/test_gallery.py
python3 scripts/audit_example_docs.py
python3 scripts/audit_component_catalog.py
git diff --check
```

Local invocations are bounded by a 180-second Python SIGALRM that raises
`TimeoutError`, allowing the existing runner's child cleanup. The checked-in
Foundation step has a three-minute limit. Syntax, both audits, actionlint and
whitespace checks pass. No new unit test is needed for this fixture-only change;
the [renderer/input evidence](carousel-track-visibility-och41.md) retains its
separate model/TestPlatform coverage.

Repository executable SHA-256:
`b2453383c1932e4b95831bfc8c4115485e2c68d8e787779dac29f17dcafc3f4e`.
Installed executable:
`a1083299477d1221012a0e7ad1a7a3fca4c7129e269c33f42b3a7c8381539acc`.
These are the binaries verified in the [sidebar checkpoint](sidebar-macos-och41.md).
The installed consumer was built for the notification checkpoint; unchanged
carousel/application code is exercised again, not claimed as a new install.

The [21-file archive](carousel-track-macos-och41/reports.tar.gz) includes all three
logs/reports/captures, final fixture, application source and consumer build log.
All members were verified against its
[manifest](carousel-track-macos-och41/manifest.json); the
[summary](carousel-track-macos-och41/summary.json) identifies final results and
executable hashes. The [adjacent walkthrough](../../examples/gallery/carousel_track_preview.md)
explains the actual key → request → Bonsai reducer → measured native update path.

Remaining physical plan items include full focus/Tab and VoiceOver navigation,
all scales/window sizes, a second window, dragging/cancellation, precise-trackpad
and line-wheel ownership/edge handoff, native automatic-advancement pause/resume,
reduced-motion/loop geometry and movement-time retirement. Sampled geometry and
normal exit do not establish smooth presentation, idle CPU or bounded resource
use. Those requirements remain open; Linux GUI qualification stays deferred to
OCH-47 while nongraphical checks remain required.
