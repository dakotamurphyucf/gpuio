# Calendar Lab

Read the [inline calendar walkthrough](main.md) for its actual Bonsai graph,
native controller commands and diagnostic boundaries, then the
[popup picker walkthrough](picker.md) for confirmed-value/draft ownership and
session lifetimes. Both entry points are independent executables in [dune](dune).

Build with `GPUIO_JOBS=2 ./scripts/gpuio build examples/calendar/main.exe`, then
run `_build/default/examples/calendar/main.exe`.

Two public `Gpuio_eio.Calendar` controllers demonstrate single dates and inclusive
ranges. Rust owns selection, the displayed month and the keyboard cursor; Bonsai
receives observations and sends explicit commands. Seed values apply once per
mount. All values are civil `Core.Date.t` dates, without time-zone conversion.
The example fixes its initial month and today marker for reproducibility.
March 6, 2024 is disabled, including the interior of an otherwise valid range.

Arrow keys navigate dates, Home/End move to week boundaries, Page Up/Down move
months, and Shift+Page Up/Down move years. The month/year headings open their
respective pages; M/Y/D also choose presentations. Enter/Space selects, and
Backspace/Delete clears. Range selection first creates a partial range, then
completes it on an equal/later date; an earlier second date restarts it.

Buttons exercise focus, leap-day replacement, clear, navigation, state reads,
disabled/read-only configuration and unmount/remount. Programmatic changes emit
observations, never user selection-completion events. Replacements honor current
constraints even when the widget is disabled or read-only.

`_build/default/examples/calendar/main.exe --self-test` opens and closes a local
window and exercises the actual OCaml/Rust bridge: both modes, revision/lease
guards, disabled dates/interiors, navigation, focus, hidden/disabled/read-only
policy, the 64-pending-request limit, remount initialization and close ordering.
It is an integration test of commands/lifetimes, not external OS keyboard or
accessibility validation. The native calendar harness separately tests GPUI
keyboard and pointer dispatch. Local OCH-35 acceptance passes; consolidated hosted gates remain pending. See [the design](../../docs/design/calendar.md).

## Popup date picker

Build `examples/calendar/picker.exe` and run
`_build/default/examples/calendar/picker.exe`. It uses
`Gpuio_eio.Date_picker` with an application-owned confirmed single date or range. Day clicks only
change the native draft; Apply accepts an empty or complete allowed selection.
Cancel, Escape and outside clicks leave the confirmed dates unchanged. The trigger
and confirmed-value text use explicit ISO date formatting.

The example includes mode switching, external value reset, read-only and disabled
controls, a nested modal dialog and left/right-edge placement. Apply
is disabled for partial ranges and read-only values; the command still revalidates
current state rather than relying on the button. Changing the application value
closes an obsolete draft. Bounds/disabled-date changes preserve existing draft
state and validate again on Apply.

`picker.exe --self-test` exercises actual bridge reads/commands, partial/complete
confirmation, single-date Apply/Cancel, mode-change invalidation, cancellation
while a confirmation is in flight, external resets,
policy changes, historical disallowed values, view remounts, keyed Bonsai
activation/deactivation, obsolete leases and window close. It closes its window.

`python3 scripts/test_date_picker.py` uses macOS accessibility actions and actual
OS keyboard/pointer events to exercise selection, Apply/Cancel, Escape, outside
clicks, focus restoration/preservation, clear and disabled/read-only behavior. It
also checks nested Escape/focus, single-date selection and actual popup bounds at
the right window edge. Set `GPUIO_PICKER_SCREENSHOT_DIR` for owned-window captures.
Pointer clicks are sent only after a system hit-test confirms the target belongs
to the test child. The script closes/reaps its child even on failure.
