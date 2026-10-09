# Sidebar motion sampling correction — OCH-17

2026-10-05. Hosted [run 37343201640](https://github.com/dakotamurphyucf/gpuio/actions/runs/37343201640)
failed the public sidebar width-motion step at branch `470210a` (runner merge
`04dcd08`). It had observed intermediate widths for collapse/reversal and reached
the 56-point compact width. After “Offcanvas mode,” every recorded sample was
zero; the required intermediate interval was missed.

The old driver performed a negative search of the entire AX tree before it began
sampling, and searched again for the content node during every measurement. Those
operations can consume the two-second animation interval. The hosted log is
consistent with that observation gap; it does not prove that the native animation
itself skipped directly to zero, nor does it record individual AX call durations.

Correction `a4ed98f` retains the stable window/content AX owners before motion and
reads their sizes directly. Full-tree absence assertions run after the width
samples. All original intermediate and endpoint ranges, interruption check,
compact inner-width assertion, hidden-content checks and reduced-motion deadline
remain. No duration, tolerance or timeout was enlarged. Absence checks establish
the settled hidden state; they do not establish exact first-frame input inertness.
No application, adapter or protocol code changed.

Both complete local runs exit 0 on macOS 14.5 arm64 / M1 Max:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 examples/navigation/main.exe
python3 scripts/test_sidebar.py --motion
python3 scripts/test_sidebar.py --motion --right
```

The application was rebuilt at `2dd6692`; executable SHA-256
`7c3c766710a8c6bb34fe967b42253caf96310c7c82cc19c9396b0747b0994fd9`.
Each run observes intermediate compact-to-offcanvas and expanded-to-offcanvas
widths, completes interruption/reveal/reduced-motion checks, closes the window
and reaps the child. Python compilation and whitespace checks pass.

[Archived evidence](sidebar-ci-sampling-och17/local-validation.tar.gz) and its
[manifest](sidebar-ci-sampling-och17/manifest.json) contain the original hosted
failure, exact patch, build log, both passing walkthroughs and execution metadata.
All six archived file hashes were verified after packaging.

Corrected hosted execution remains required. These are native AX geometry and
application lifecycle checks, not physical presentation latency or VoiceOver
acceptance. Linux graphical acceptance remains separately deferred to OCH-47.
