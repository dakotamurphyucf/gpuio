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

## Pointer ownership and cancellation follow-up — 2026-10-08

At base `5d07a92b`, the repository gallery and the independent consumer built for
the [scroll adapter checkpoint](macos-scroll-phases-och41.md) pass twenty pointer
cases each. No production code changes or new consumer build are needed here.
The fixture covers both axes and Dark/Light at Comfortable scale in a compact
viewport with the default movement policy.

For each combination, a 200-pixel background drag reveals the next card while
the accepted selection remains Capture. Releasing selects Explore through the
public Bonsai reducer. Repeating that preview and pressing Escape before release
keeps Capture selected. The final fixture checks the candidate button's visible
center before cancellation, so an undelivered drag cannot pass as a cancelled
one. Cross-axis movement, a five-pixel movement and disabled navigation all keep
Capture selected. A native drag inside the card's text input selects nonempty
text without changing the carousel selection. Post-release sampling checks for
delayed commits; it is not an every-frame assertion.

```sh
python3 scripts/test_gallery.py --section carousel-drag --images scratch/carousel-drag
python3 scripts/test_gallery.py --section carousel-drag --executable /path/to/installed/main.exe --images scratch/carousel-drag-installed
```

The initial fixture incorrectly requested focus on a disabled viewport and
failed after the first four cases. Disabled viewports intentionally reject focus;
the corrected fixture preserves the preceding Capture selection and verifies
actual disabled pointer behavior. Local `002` then passed all cases. Final local
`003` and installed `001` add theme metadata and positive preview checks and both
pass. All children exit normally. Local runs use the 180-second exception/cleanup
wrapper; Foundation adds a separate three-minute step. Syntax, documentation and
catalog audits, actionlint and whitespace checks pass.

The [pointer archive](carousel-track-macos-och41/drag-reports.tar.gz) and verified
[manifest](carousel-track-macos-och41/drag-manifest.json) retain all attempts,
reports and the final fixture/application source. Executable identities are in
the raw logs and match the scroll adapter checkpoint. The adjacent walkthrough
now explains native preview ownership, release-to-Bonsai selection, cancellation
and native child input rather than presenting dragging as an OCaml callback.

This qualifies OS pointer routing and Escape cancellation for these cases.
Hardware trackpad/line-wheel ownership and edges, capture loss, other sizes/scales,
full Tab/VoiceOver, movement-time retirement, seamless loop geometry and measured
resource/presentation behavior remain separate. It does not close the loaded-list
overlap report or establish performance from sampled AX positions.

## Line-wheel routing and edge handoff — 2026-10-08

At base `b700d964`, both the repository gallery and reused scroll-phase installed
consumer pass `--section carousel-wheel` on their first attempts. No production
change or new build is needed. The fixture posts actual discrete line-wheel
CGEvents through AppKit, verifies their X/Y delta fields and non-precise flag,
and checks desktop ownership at each target before posting.

Four cases per binary cover horizontal/vertical and Dark/Light at Comfortable
scale, compact viewport and immediate movement. Each case navigates Capture →
Explore → Refine → Publish → Refine with the outer viewport rectangle unchanged.
At Capture/Publish, outward horizontal input keeps selection and viewport position
unchanged. Outward vertical input keeps selection but scrolls the enclosing page
in the appropriate direction. Vertical input over a horizontal track also scrolls
the page. Each vertical handoff measured 78 logical pixels of page displacement
in these runs; this is observed geometry, not a promised wheel scaling factor.

```sh
python3 scripts/test_gallery.py --section carousel-wheel --images scratch/carousel-wheel
python3 scripts/test_gallery.py --section carousel-wheel --executable /path/to/installed/main.exe --images scratch/carousel-wheel-installed
python3 -m py_compile scripts/gallery_carousel_wheel.py scripts/test_gallery.py
ruff check scripts/gallery_carousel_wheel.py
python3 scripts/audit_example_docs.py
python3 scripts/audit_component_catalog.py
git diff --check
```

Both executables exit normally. Local runs use the existing 180-second
exception/cleanup wrapper; Foundation adds a separate three-minute step.
Syntax, targeted lint, documentation/catalog audits, actionlint and whitespace
checks pass. The [six-file archive](carousel-track-macos-och41/wheel-reports.tar.gz)
contains both logs/reports and exact fixture/application source. All members were
verified against the [manifest](carousel-track-macos-och41/wheel-manifest.json).
The [summary](carousel-track-macos-och41/wheel-summary.json) records executable
hashes, which match the scroll adapter checkpoint; this reuses its independently
installed consumer rather than claiming a new installation.

This completes the discrete line-wheel desktop subset of the physical plan.
Events are separated beyond the quiet deadline; multi-event burst timing remains
covered by native deterministic tests, not this desktop fixture. Precise hardware
trackpad routing/cancellation, full focus/VoiceOver, additional geometry,
movement-time retirement, continuous-loop presentation and measured resources
remain separate. It does not resolve the loaded-list overlap report.
