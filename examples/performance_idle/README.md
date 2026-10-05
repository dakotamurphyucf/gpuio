# Focused and unfocused settled idle

Qualification driver under validation. The full reference window is 1200×800
logical pixels, as declared in the performance plan. The two-second smoke passes both phases;
full repeated 60-second measurements remain pending. This uses public Eio/Bonsai
APIs, a native text editor and a focusable button, plus the optional native probe.

The macOS collector activates only the owned test app, focuses its editor long
enough to exercise the caret, then moves focus to the settle button. It verifies
editor focus/blur through AX; the app independently queries the native editor
snapshot and focused-input owner. The first phase measures the active app. The
second gives foreground to the existing Finder without changing its files or
windows, then measures the inactive test app. Every phase has the native probe's
two-second settling period followed by 60 seconds (two in smoke).

Both phases require zero draw, dirty-to-submission, animation-submission and
input-frame histogram deltas, zero dropped timestamps, no activation changes,
and complete owned-resource retirement afterward. A full optimized run is needed;
smoke is not acceptance. Warm up explicitly, then repeat three full runs.

The probe samples native activation and AppKit `NSWindow.occlusionState` at
absolute one-second deadlines, on the native thread without requesting frames or
notifying views. Unavailable OS visibility is rejected, as are invisible samples
or observation gaps above 1.5 seconds. The collector independently samples the
application's AXFrontmost state. The observations are bounded to 128 entries and
the native task is cancelled at Finish or component retirement. They establish
sampled native visibility/activation, not physical display presentation or
continuous visibility between samples. Rectangular, opaque ordinary windows are
used; AppKit's documented bounding-shape/transparency limitations do not establish
pixel-perfect exposure for arbitrary shaped/translucent windows.

A first collector attempt subtracted CoreGraphics window bounding rectangles.
It rejected the smoke before timing because the Dock reports a screen-sized
rectangle whose contents are largely transparent. That approximation was removed;
AppKit's actual occlusion flag is authoritative for the native display-link gate.
Those failed preflights remain retained locally and are not passing measurements.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release examples/performance_idle/main.exe
python3 scripts/test_measure_idle.py
python3 scripts/measure_idle.py --build-profile release --smoke --output scratch/idle-smoke
python3 scripts/measure_idle.py --build-profile release --output scratch/idle-full-1
```

The probe uses schema v3, independently fingerprinted on both languages from
`examples/performance_probe/schema.txt`. Other workload Begin commands do not
start idle sampling. Linux compilation/parser coverage is useful, but this OS
visibility/foreground collector deliberately requires macOS and does not claim
Linux GUI acceptance.
