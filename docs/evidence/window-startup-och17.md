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
