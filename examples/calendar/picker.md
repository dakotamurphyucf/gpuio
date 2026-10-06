# Popup date picker: draft first, confirmed dates on Apply

[picker.ml](picker.ml) composes `Gpuio_eio.Date_picker` with a confirmed application
selection, policy controls and an optional containing dialog. There is no separate
`.mli`; public contracts are [Eio date_picker.mli](../../lib/eio/date_picker.mli),
[pure date-picker policy](../../lib/core/date_picker.mli) and
[calendar values](../../lib/core/calendar.mli). Read fixture constructors, `format`,
`component` and its keyed controller graph before the diagnostic `exercise`.

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/calendar/picker.exe
./scripts/gpuio exec _build/default/examples/calendar/picker.exe
# Optional command/session diagnostic; closes its own window:
./scripts/gpuio exec _build/default/examples/calendar/picker.exe --self-test
```

Open the trigger, choose dates and Apply, or dismiss with Cancel/Escape/outside click.
Try Single date, external Reset dates, read-only/disabled, dialog placement and Right
edge. No assets or network service are required. This executable parses only
`--self-test`, not a background flag. Toolchain prerequisites are in the
[development guide](../../docs/development.md). The optional self-test uses the real
bridge but does not prove physical keyboard/focus/VoiceOver acceptance; the
[README](README.md) links separate owned-child native tests. Linux GUI qualification
remains outside current macOS-first release acceptance.

## Confirmed model and per-opening native draft

`initial` is the inclusive range February 27–29, 2024; `single_initial` is leap day.
The initial displayed month is explicitly February 2024, without consulting a
clock. `format` pattern-matches Empty, Single, Range_start and Range, calling
`Calendar.Format.format Iso` for dates; display formatting does not change values.

`B.state` creates confirmed `value`, `flags`, placed/active=true and dialog/right-edge=false
with setter effects. Flags initially permit range selection, editing and input.
The reactive config derives Single or Range mode and optional disabled leap-day
constraints (restricted is exercised only by self-test). `on_change` records the
confirmed callback and runs `set_value`; user day/range interactions alone never
call it. The application owns `value`, while one native calendar owns the draft for
an opening. `P.draft` is an accepted native snapshot, absent while mounting.

`B.map active` creates a keyed Int map containing entry 0 or nothing. `B.assoc`
constructs `P.create` only for active entry 0; removing it deactivates the picker
computation, distinct from placed=false, which removes its view but retains the
controller computation. `B.map` transforms reactive inputs; `B.assoc` builds keyed
subgraphs; `let%arr` derives the latest view/controller callbacks. These are Bonsai
operations, not native state mutations. `B.Edge.on_change` tracks current open/draft/error
and confirmed value in refs for diagnostics. `B.Edge.lifecycle` starts self-test
once, while normal operation uses the same controller without the exercise.

## Trace Apply, cancel and external replacement

Click the formatted range trigger. `P.view` injects an opening and mounts a native
calendar draft seeded from the confirmed value. The effect finishing does not mean
a native draft observation is ready. Selecting February 20 then 22 updates only
that draft; “Confirmed” remains February 27–29. **Apply** re-reads native state,
revalidates the current opening, application value, complete selection, constraints
and read-only policy, then calls `on_change` with February 20–22 and closes. Its
effect changes Bonsai value; `let%arr` formats the new trigger and confirmed text.
A partial range cannot confirm, and `can_confirm` is only a UI hint: the operation
checks current state again.

Cancel/Escape/outside dismissal discards the opening without changing confirmed
dates. A captured cancel from a previous opening cannot close a new one. **Reset
dates** directly changes the application value, invalidating an obsolete open draft.
**Single date/Date range** updates both mode and a compatible seed value with
`E.Many`, closing an obsolete session. Read-only can retain a draft but reject Apply;
disabling closes the opening. If newer constraints invalidate historical confirmed
dates, a new draft starts Empty while the confirmed value remains unchanged until
successful Apply. The example allows empty confirmations as well as complete dates.

`P.view ~overlay` uses an accessible trigger and 340-pixel popup configuration with
outside dismissal. The example places it once: either in its row or the dialog
body. Before moving it between dialog/left-right placement it runs `P.cancel`.
`View.dialog` uses key `picker-dialog`, a 420-pixel width and dismissal callback
that cancels the picker and closes dialog state. Native adapters own popup placement,
focus restoration and dismissal; this file does not calculate screen coordinates.
`error_message` translates typed policy errors into notice text.

## Diagnostic fences and runtime limits

`exercise` calls `P.open_popup`, settles frames and waits for the **current** reactive
controller's open+draft state. `ready` allows up to 120 frame requests and also stops
on picker errors; an old captured `t` does not acquire newer snapshots. Window frame
callbacks do not acknowledge calendar mounting or physical screen presentation.
The file has no wall-clock watchdog; the external runner must bound stalled frames.

The diagnostic rejects closed confirmation and partial ranges, applies a complete
range, cancels an edited draft, rejects old-opening commands, remounts placement,
and injects cancellation during an in-flight confirmation with Expert effect evaluation.
It checks keyed deactivation/reactivation, read-only and restricted policy, historical
Empty fallback, disabling, external reset, range→single mode invalidation, single
Apply/Cancel and close ordering. One subtle distinction is explicit in the source:
a captured native command rejects an old calendar mount, while date-picker `confirm`
resolves the current draft for its opening. After closing, confirmation returns
Not_open and a captured native read returns Closed. Only successful completion prints
`GPUIO_DATE_PICKER_PUBLIC_OK`; none of these cases were run for this documentation review.

`App.run` opens a 760×600 window and owns cleanup; the controllers fence stale native
leases/session IDs and deactivate with their Bonsai branch. There is no application
file/network I/O, Eio producer or polling timer. `Eio_main.run` at the end only prints
the diagnostic marker after App.run has exited. Expert callbacks, frame loops and
tracking refs support verification, while ordinary code should use typed Result effects.

For a small adaptation, change initial dates through validated `Calendar.Range` and
Selection constructors. Keep mode and controlled value compatible, use civil dates
without time-zone arithmetic, and keep `on_change` as the explicit confirmation
boundary. Add business constraints to config and handle historical invalid values;
never apply a native day observation directly to confirmed application value.
