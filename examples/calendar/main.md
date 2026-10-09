# Inline Calendar Lab: native state and explicit commands

[main.ml](main.ml) mounts two retained calendars: Appointment selects one civil
date, Travel range selects an inclusive range. This executable has no separate
`.mli`; [calendar.mli](../../lib/core/calendar.mli) supplies the public value/event
contracts and [Eio calendar.mli](../../lib/eio/calendar.mli) the controller API.
Read the fixture constructors, `component`, its `report`/`card` helpers and startup
before the optional `run_self_test` machinery.

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/calendar/main.exe
./scripts/gpuio exec _build/default/examples/calendar/main.exe
# Optional bridge/lifetime diagnostic; opens and closes its own window:
./scripts/gpuio exec _build/default/examples/calendar/main.exe --self-test
```

There are no external assets or I/O prerequisites beyond the
[development toolchain](../../docs/development.md). This executable recognizes
`--self-test`; it does not implement a background-launch flag. Normal launch keeps
a 780×620 window open until Close or window dismissal. The diagnostic does not
substitute for actual OS keyboard, pointer or accessibility checks; current release
scope is macOS-first and compilation does not establish Linux GUI acceptance.

## Values and the Bonsai/controller boundary

`day` parses fixed `Core.Date.t` strings, `month` validates a `Calendar.Month`, and
`single`/`partial`/`range` construct typed selection variants. Initial Appointment
selection is 2024-02-29; both calendars start in February 2024. The Appointment today
marker is explicitly that leap day, while Travel supplies no today marker. No clock
or time-zone conversion is involved. Shared constraints disable March 6, including
its interior presence in a proposed range. These checked fixture constructors use
`ok_exn`; handle errors explicitly for external input.

`policy` is an application record of disabled/read_only/hidden flags, initially
false. `B.state` allocates that record, shown=true and notice text, each with a setter
effect. These are reactive values, not copies of native calendar selection. The
config's `let%arr policy = policy` derives a validated `Calendar.Config` from the
current flags. An effect is work scheduled by a handler; deriving configuration
does not run a command.

`C.create` creates the Appointment controller with reactive config, initial selection,
initial month and reactive `on_event`. A second controller uses Range mode, Empty
selection and the same constraints. A third `unplaced` controller is intentionally
never passed to `C.view`; it exists to test Not_mounted. Initial seeds apply once
per native mount, not on every config recomputation. Native Rust owns selection,
shown month and keyboard cursor; `C.snapshot` is the last accepted observation.
Use one `C.view` per controller and keep mode immutable until remount.

`on_event` handles Selected by displaying its selection and Rejected by displaying
the reason. Observed/Changed do not update that notice, though the controller still
updates its own accepted snapshot. The card reads that snapshot to show native
selection under each widget. Programmatic replacement emits observations, not
Selected. The `received` ref records events only for self-test; it does not control
user state. `B.map2` derives the two snapshots and `B.Edge.on_change` watches them
with typed equality, tracking the newest Appointment controller in a ref and
starting self-test once both mounted observations exist.

## Follow a native selection and a command

Click a Travel date: native code updates its Range_start draft and queues the new
snapshot. Click an equal/later end date: native range completion emits Selected;
`on_event` runs `set_status`, Bonsai derives the status/card text and GPUIO renders
it. A disabled interior date rejects the completion instead. Native navigation
and intermediate cursor movement stay native; Bonsai does not calculate the grid.

Click **Leap day** for Appointment. Its `on_click` is `report (C.replace code initial)`.
The command effect requests explicit native replacement; `report` uses
`Bonsai.Effect.Let_syntax`'s `let%bind` to await the asynchronous Result, then derives
status from its snapshot or error. This effect syntax sequences actions, unlike
Bonsai `let%arr`, which derives a value from reactive inputs. The handler never
feeds an observation back into replacement automatically.

**Focus**, **Clear**, **February 2024** and **Read native state** respectively use
`C.focus`, `C.clear`, `C.show_month` and `C.read_snapshot`. Navigation preserves
selection; reading does not focus or change it. Disable/read-only buttons derive
new config, while hidden policy is used only by self-test. Explicit replacement
still validates constraints when hidden/disabled/read-only; focus obeys native
visibility/enabled/modal gates. The outer style's Visibility is distinct from
shown=false, which removes Appointment's view. Unmount invalidates its native
lease; Remount seeds a fresh calendar with the original leap day. A retained
controller may still expose an old snapshot, so commands must check their Result.

## Optional verification and lifetime

`run_self_test` uses `expect`, `error` and `check` to assert typed responses. `frame`
and `settle` request native render callbacks through Expert effect plumbing; two
callbacks help sequence layout changes but do not prove physical presentation.
`concurrent` admits all effects in one UI turn before replies drain, preserving
result order; 65 reads must yield 64 successes and one Busy response.

The diagnostic checks unplaced/state reads, native focus, guarded replacement and
Stale_revision, wrong mode, disabled date/interior, bounded month movement, disabled
focus but allowed programmatic replacement, read-only/hidden policy, unmount/remount
and stale lease/snapshot rejection. It verifies commands generated no Selected
events, then submits reads around window close and checks successful-before/Closed-after
ordering. `completed` must become true after App.run returns; only then does an
Eio stdout write print `GPUIO_CALENDAR_PUBLIC_OK`. There is no new test evidence
from writing this guide, and no in-file time-based watchdog; test automation must
bound a stalled native run externally.

`App.run` owns native startup/event delivery and window lifetime; there is no file,
network or background producer in this application. Controller/window teardown
invalidates command leases and suppresses stale delivery. `E.Expert.of_fun/eval`,
frame sequencing, concurrency counters and refs in the test are diagnostic helpers,
not the first template for application I/O. The [calendar design](../../docs/design/calendar.md)
explains input/navigation and admission bounds.

To add another disabled day, change `constraints` using the validated date constructor;
keep fixture selection legal for its mode. To use application-controlled confirmation,
start with the [popup example](picker.md), rather than copying native snapshots into
calendar configuration. For an optimistic command, pass an accepted snapshot to
`C.replace_if_unchanged`; its lease and revision guard prevents overwriting a newer
selection or a remounted widget.
