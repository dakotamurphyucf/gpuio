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

## Automatic pause/resume follow-up — 2026-10-08

At base `f5109d84`, the same repository and installed executables pass
`--section carousel-automatic` on their first attempts. No production changes or
new build were needed. The independent consumer is reused, not newly installed.

The actual four-second automatic policy preserves selection for at least 5.2
seconds under each of five conditions: viewport focus, track hover with focus
outside, disabled navigation, the original window inactive while a second owned
window is open, and application Reduced motion. Each release advances exactly to
the expected next card after a fresh interval. Observed intervals are 4.12–4.35
seconds across the ten resumes; they include setup, dispatch, Bonsai delivery
and AX sampling and are not timer precision or presentation-latency measurements.
After each observed transition the fixture focuses the viewport and samples a
further 0.4 seconds of unchanged selection; it does not assert every unsampled
frame or independently measure native pending-proposal counts.

Leaving the page while a deadline is eligible, waiting beyond the interval and
returning preserves the last selection. The retired timer does not advance the
inactive Bonsai model in this sequence. The second window and main app close
normally; automatic advancement is disabled and application motion restored to
System before exit. No macOS preferences, clipboard or input source change.

```sh
python3 scripts/test_gallery.py --section carousel-automatic --images scratch/carousel-automatic
python3 scripts/test_gallery.py --section carousel-automatic --executable /path/to/installed/main.exe --images scratch/carousel-automatic-installed
```

Local invocations retain the same 180-second exception/cleanup wrapper. A
separate three-minute Foundation step is added. Python syntax, example/catalog
audits, actionlint and diff checks pass. The
[seven-file automatic archive](carousel-track-macos-och41/automatic-reports.tar.gz)
retains both logs/reports, exact fixture/application source and original consumer
build log. All members were verified against the
[manifest](carousel-track-macos-och41/automatic-manifest.json); the
[summary](carousel-track-macos-och41/automatic-summary.json) retains each interval
observation and executable identity.

This supersedes the automatic-advancement pause/resume gap above for these
conditions. Captured drag/wheel pause, precise gesture cancellation/edges, full
Tab/VoiceOver, all sizes/scales, movement-time retirement, continuous-loop visual
geometry and measured resource/frame performance remain separate work. Opening
a second window here qualifies an inactive-window pause, not independent carousel
contents or cross-window model isolation.
