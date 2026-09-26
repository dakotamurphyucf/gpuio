# Calendar Lab

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
keyboard and pointer dispatch. Popup composition and full OCH-35 acceptance are
still in progress; see [the design](../../docs/design/calendar.md).
