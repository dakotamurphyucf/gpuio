# Window pointer readiness — OCH-17

Local macOS 14.5 arm64 follow-up, 2026-10-05, on `e7e16a4` plus the archived
harness changes. The earlier hosted macOS 15.7.9 failure remains recorded in
[run 37374125077](hosted-presentation-calibration-och17.md#repeat-on-hosted-run-37374125077).
Its unknown PID 2330 covered the Fullscreen pointer location after a successful
custom title-bar drag; no Fullscreen click was issued. The artifacts do not
establish why that process covered the target.

The lifecycle driver now explicitly raises the owned window after the drag,
re-reads the button's position, and waits for foreground activation plus stable
coordinates and an app-owned physical pointer target. Its polling deadline is
12 seconds; individual OS calls are not preemptible, and an overrun does not
admit a late click. The existing final ownership/window-control guard remains
in place immediately before posting input. A bounded 24-entry transition log
records ownership status, semantic role, coordinates, foreground state and the
process executable basename; it does not record other applications' UI text.
Persistent occlusion still fails.

Seven portable regressions pass: transient occlusion with changing coordinates,
persistent occlusion, accidental native-window controls, last-moment ownership
loss, a background app, bounded diagnostics for a continuously moving target,
and an OS lookup returning after the deadline. The existing five accessibility
traversal tests pass. The new tests run in both required foundation CI jobs.
Python syntax checks and `git diff --check` pass.

Both full local standard/custom window walkthroughs pass against the independently
installed public gallery executable:
`f6476346c40b04872c72f7bec798e967c5d1abb76f708512a34de7efda256cbb`.
They preserve the Unicode draft and selection through three minimize/restore
cycles, then prove deletion/undo and second-window-close retention. The custom
sequence additionally passes native title-bar movement, actual Fullscreen and
restoration, AppKit border resize, and the existing double-click zoom policy.
No OS preference is changed; both child applications are closed/reaped.

The local pointer was app-owned on its first observation and became ready in
135 ms. **The remote occlusion was not reproduced locally.** This evidence
qualifies the additional readiness guard and the local complete walkthroughs;
it does not prove the hosted failure resolved. The next current-source hosted
run remains necessary. Both hosted Metal presentation failures, current-source
performance/resources, VoiceOver and other release gates remain open.

[Commands, source patch, portable/native logs and reports](window-readiness-och17/validation.tar.gz)
are retained with a [checksum manifest](window-readiness-och17/manifest.json).

## Owned-window placement follow-up — 2026-10-06 UTC

Hosted run 37387307992 identified `NotificationCenter` at the Fullscreen
pointer target `(1321.5, 235.5)` after the title-bar drag. The button was never
clicked. The later AX lookup returned no target; that observation alone does
not establish why the target disappeared. The original failed report remains
in the [hosted archive](hosted-presentation-calibration-och17.md#repeat-on-hosted-run-37387307992).

The fixture now places only its own window near the main display's upper left
before creating any transition baseline. It constrains size to the display,
leaves room for the later right/down drag, and records display bounds plus
requested/actual window geometry. It waits for each AX size/position operation
to settle. The existing foreground, stable-target, physical ownership and
native-window-control guards remain required. Notification settings are unchanged;
this placement reduces likely overlap and does not guarantee absence of occluders.

At `cf3023c` plus the archived harness patch, nine portable readiness/placement
tests and five AX traversal tests pass. Both complete installed-gallery window
walkthroughs pass on macOS 14.5 arm64, with a 1728×1117 logical display. The custom
window starts at `(20, 60, 1120, 820)` and the actual drag moves it to
`(52, 82, 1120, 820)`. The Fullscreen target is app-owned at `(1101.5, 105.5)`;
readiness takes 128 ms. Fullscreen restoration, border resize, double-click zoom,
selection/delete/undo, three minimize/restore cycles and second-window cleanup
all pass. Both child processes are reaped. Binary SHA-256:
`057501a21aae024f0cbdf86e4e756aea8c3fae75f4949af5a7d4edecceef0ee3`.

[Exact commands, patch, reports, screenshots and logs](window-readiness-och17/placement/validation.tar.gz)
have a [verified checksum manifest](window-readiness-och17/placement/manifest.json).
The hosted notification overlap was not reproduced locally. Hosted confirmation
and the separate Metal/performance/accessibility/distribution gates remain open.
