# Initial black window — open OCH-17 observation

On 2026-09-30 the owner reported a black window during the public gallery test.
A scoped local macOS 14.5 arm64 capture subsequently reproduced black client
content in the first captured window image. The following three startup images
and seven page-transition images showed rendered gallery content. The same run
completed the real Settings keyboard-composition test and closed normally.

The first capture completed about 0.282 seconds after initializing the Python
AX helper. Subsequent startup captures were separated by a 150 ms delay plus
capture overhead. This is evidence of an initial blank frame, not an exact
duration measurement or proof of the cause/duration of the owner's earlier
observation. An earlier run's first successful capture, at about 1.269 seconds,
already showed the gallery. A final/failure screenshot alone would miss this.

Capture only the child PID's named window through `test_canvas.screenshot`,
starting when `CGWindowListCopyWindowInfo` first reports it; continue through
startup and page transitions. Keep the child in a bounded test process group
and close/reap it after the keyboard scenario. The local evidence is under
`scratch/agents/root-20260929-m7-resumed/settings-composition-frames/` and
`settings-composition-native.log`; these are diagnostics, not build inputs.
The first image's sampled client pixels were uniformly black; later captures
contained hundreds of distinct sampled colors and visible gallery controls.

This observation is **not fixed**. Investigate the interval between showing the
native window, receiving the initial OCaml tree and presenting its first frame.
The host currently creates a visible window before the initial tree transaction.
Any change must preserve background-window behavior, focus policy, window-open
acknowledgments and prompt rendering without starting a recurring idle timer.
Validate a fresh installed consumer and multi-window startup as well as the
repository example. Linux graphical startup remains deferred qualification.

## Reproducible startup capture

`scripts/capture_window_startup.py` now supplies the diagnostic that previously
existed only as a local wrapper. Build the application first, then run, for example:

```sh
python3 scripts/capture_window_startup.py \
  --output scratch/gallery-startup-01 \
  --title 'GPUIO · Component Studio 1' \
  --seconds 5 \
  -- _build/default/examples/gallery/main.exe --background
```

Use a **new** output directory for each run. Pass a built executable directly,
including an independently installed consumer executable; a launcher that spawns
the real application would have a different PID. Repeat `--title` to observe
multiple windows opened by that application. The tool never activates a window,
sends input or creates Accessibility objects. The application may still focus
its own windows according to its launch options. This is a capture-only tool;
it does not exercise keyboard, IME, close requests or other native behavior.

CoreGraphics enumeration is checked before launch. A missing desktop produces an
incomplete report without starting an app. During acquisition, the tool selects
only the launched PID's named, normal, on-screen windows. It records the first
observation and each capture's start/end relative to process launch, retains PNGs
and the child log, and closes/reaps its own process group even after failure.
Acquisition has both time and frame-count bounds; individual `screencapture`
calls have deadlines. Pixel analysis happens after closing the child and cannot
extend the time its window stays open. Reports preserve partial observations.

`report.json` describes the **center half** of each image: exact opaque-black
pixels, maximum RGB channel value and sampled distinct colors. The center crop
avoids ordinary borders/titlebars; it does not certify the rest of the window.
An intentionally black application is valid, and a colored placeholder does not
prove readiness. `status: collected` means diagnostic collection completed,
**not** application acceptance. Screenshots have acquisition overhead and may
miss intervening frames; these timestamps are not physical presentation latency
or an exact duration of a blank interval. No arbitrary startup performance budget
is introduced by this diagnostic.

On 2026-10-01, offline analysis of all eleven retained macOS captures classified
the original first image's center as opaque black; the other ten had 29–40 sampled
colors and were not all black. Transparency and a colored pixel outside the
sparse color sample were also checked so neither can be mislabeled opaque black.
CLI bounds, early-child-exit/deadline control flow with a deterministic enumeration
source, and real non-GUI child cleanup (including a child ignoring SIGTERM) passed.
Terminating the diagnostic itself with SIGTERM also produced an incomplete report
and reaped its owned child; it did not leave the child running after the driver.
The current session's real CoreGraphics enumeration returns no list. Its new
preflight was verified to stop **before launching the command**, with an incomplete
JSON report. **The new tool has not yet captured a fresh native window**; this is
offline/control-flow evidence, not new GUI acceptance. Local details are in
`scratch/agents/root-20260929-m7-resumed/OCH-17-startup.md`.

## Source ordering and repair constraints

The source review confirms `host.rs` creates a visible GPUI window before
`Session::open` and the `Opened` response. `App.open_window_config` builds its
driver first, but native admission precedes the first tree submission. The root
view can therefore draw without an application root. At the pinned Zed revision
`a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b`, `gpui_macos/src/window.rs` orders a
window front during construction and submits its Metal scene later. Its occlusion
callback stops the display link for hidden windows. Simply setting `show: false`
and waiting for a frame risks stranding startup. GPUI's `on_next_frame` callback
also runs before the following draw/present; it is not a GPU presentation
completion signal.

These facts explain an opportunity for a brief blank frame. They do **not** prove
the cause of the later `native_images` timeout, which logged macOS
`hiservices-xpcservice` connection errors. A repair needs an explicit first-content
presentation/show contract, validated with focused and background windows, delayed
or rejected initial transactions, close-before-first-content and multiple windows.
Preserve native-open acknowledgments, input ownership and idle behavior. Neither
the diagnostic nor this source review changes production visibility or establishes
that the black-window defect is fixed.

## Foreground capture after desktop access was restored

On 2026-10-04, macOS 14.5 arm64, the current gallery build produced twelve
foreground startup captures with the checked-in tool. The first window was
observed at 0.456 seconds; its first capture ran from 0.456 to 0.801 seconds and
visually contains the Presentation page. None of the twelve center regions is
opaque black. These are sampled acquisition times, not presentation latency or
proof that no earlier blank frame occurred. No production startup fix is claimed.

```sh
python3 scripts/capture_window_startup.py \
  --output scratch/agents/root-20261004-resumed/gallery-startup-foreground-001 \
  --title 'GPUIO · Component Studio 1' --seconds 10 --max-frames 12 \
  -- _build/default/examples/gallery/main.exe
```

Two preceding background acquisitions (5 and 30 seconds) found no on-screen
matching window. A separate owned-process diagnostic found the correctly named
window in the all-windows list, and a stack sample showed the native main thread
in its normal AppKit event loop and the OCaml UI domain polling. This establishes
that those background capture failures were not evidence of a startup deadlock;
it does not explain the window's off-screen/Space visibility or qualify background
presentation. All diagnostic children were terminated and reaped. Reports,
screenshots and the owned-process sample remain in the same ignored workspace.
